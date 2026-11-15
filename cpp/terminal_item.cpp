#include "terminal_item.h"

#include "tessera/src/bridge.cxxqt.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QDateTime>
#include <QtCore/QMimeData>
#include <QtCore/QUrl>
#include <QtGui/QClipboard>
#include <QtWidgets/QApplication>
#include <QtGui/QFontDatabase>
#include <QtGui/QFontMetricsF>
#include <QtGui/QGlyphRun>
#include <QtGui/QGuiApplication>
#include <QtGui/QPainter>
#include <QtQuick/QQuickWindow>
#include <QtQuick/QSGImageNode>
#include <QtQuick/QSGRectangleNode>

#include <cmath>

namespace {
constexpr quint32 FLAG_BOLD = 1, FLAG_ITALIC = 2, FLAG_UNDERLINE = 4, FLAG_STRIKE = 8, FLAG_FAINT = 16,
                  FLAG_HAS_BG = 32, FLAG_INVISIBLE = 64;
constexpr quint32 SIG_DIRTY = 1, SIG_EXITED = 2, SIG_EVENTS = 4;

QString qs(const rust::String& s) { return QString::fromUtf8(s.data(), qsizetype(s.size())); }
rust::Str rs(const QByteArray& b) { return rust::Str(b.constData(), size_t(b.size())); }

QRgb blend(QRgb a, QRgb b, qreal t)
{
    auto mix = [t](int x, int y) { return int(x + (y - x) * t); };
    return qRgb(mix(qRed(a), qRed(b)), mix(qGreen(a), qGreen(b)), mix(qBlue(a), qBlue(b)));
}

// The node tree: a background rectangle, then one image node per row.
// Row nodes join the tree only once they have a texture (a textureless image
// node crashes the software renderer).
class TerminalNode : public QSGNode {
public:
    QSGRectangleNode* bg = nullptr;
    QSGRectangleNode* overlay = nullptr;
    QVector<QSGImageNode*> rows;
    ~TerminalNode() override
    {
        for (QSGImageNode* n : rows)
            if (!n->parent()) delete n;
    }
    void clearRows()
    {
        for (QSGImageNode* n : rows) {
            if (n->parent()) removeChildNode(n);
            delete n;
        }
        rows.clear();
    }
};
} // namespace

TerminalItem::TerminalItem(QQuickItem* parent)
    : QQuickItem(parent)
    , m_frame(new tessera::Frame())
{
    setFlag(ItemHasContents);
    setFlag(ItemAcceptsInputMethod);
    setFlag(ItemIsFocusScope);
    setActiveFocusOnTab(true);
    setAcceptedMouseButtons(Qt::AllButtons);
    setAcceptHoverEvents(true);
    setCursor(Qt::IBeamCursor);
    const QFont fixed = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    m_fontFamily = fixed.family();
    m_fontPointSize = fixed.pointSizeF() > 0 ? fixed.pointSizeF() : 11;
    m_blinkTimer = new QTimer(this);
    m_blinkTimer->setInterval(530);
    connect(m_blinkTimer, &QTimer::timeout, this, [this] {
        m_blinkOn = !m_blinkOn;
        if (m_cursorY >= 0) repaintRow(m_cursorY);
    });
    // Momentum: after a touchpad flick the view keeps gliding and slows down.
    m_momentum = new QTimer(this);
    m_momentum->setInterval(16);
    connect(m_momentum, &QTimer::timeout, this, [this] {
        m_velocity *= 0.95;
        if (std::abs(m_velocity) < 0.02) {
            m_momentum->stop();
            m_velocity = 0;
            return;
        }
        scrollPixels(m_velocity * 16);
    });
    m_autoscroll = new QTimer(this);
    m_autoscroll->setInterval(40);
    connect(m_autoscroll, &QTimer::timeout, this, [this] {
        if (!m_session || !m_selecting) { m_autoscroll->stop(); return; }
        const QPointF p = m_lastPointer;
        // Faster the farther past the edge (and it keeps accelerating there).
        const qreal rowH = m_ch / m_dpr;
        const qreal top = m_pad / m_dpr, bottom = height() - m_pad / m_dpr;
        const qreal dist = p.y() < top ? top - p.y() : qMax<qreal>(0, p.y() - bottom);
        const uint32_t rows = uint32_t(qBound<qreal>(1, 1 + std::pow(dist / rowH, 1.4), 30));
        if (!m_session->autoscroll_tick(float(p.x() * m_dpr), float(p.y() * m_dpr), uint32_t(width() * m_dpr),
                                        uint32_t(height() * m_dpr), uint32_t(m_cw), uint32_t(m_ch), uint32_t(m_pad), rows))
            m_autoscroll->stop();
        polish();
    });
    // Mouse wheel: each notch adds three rows of travel, eased out frame by
    // frame, so wheel scrolling glides like a touchpad's.
    m_smooth = new QTimer(this);
    m_smooth->setInterval(16);
    connect(m_smooth, &QTimer::timeout, this, [this] {
        const qreal rowH = m_ch / m_dpr;
        qreal stepPx = m_smoothPending * 0.22;
        if (std::abs(stepPx) < rowH * 0.25) stepPx = std::copysign(qMin(std::abs(m_smoothPending), rowH * 0.25), m_smoothPending);
        m_smoothPending -= stepPx;
        scrollPixels(stepPx);
        if (std::abs(m_smoothPending) < 0.5) {
            m_smoothPending = 0;
            m_smooth->stop();
        }
    });
    m_pasteTimer = new QTimer(this);
    m_pasteTimer->setInterval(125);
    connect(m_pasteTimer, &QTimer::timeout, this, [this] {
        // Paste Slowly: 16 bytes at a time.
        if (!m_session || m_slowPaste.isEmpty()) { m_pasteTimer->stop(); return; }
        const QByteArray chunk = m_slowPaste.left(16);
        m_slowPaste.remove(0, chunk.size());
        m_session->send_text(rs(chunk));
    });
}

TerminalItem::~TerminalItem()
{
    delete m_notifier;
    if (m_session)
        rust::Box<tessera::Session>::from_raw(m_session); // drops: detaches
    delete m_frame;
}

void TerminalItem::setAttachId(const QString& v)
{
    if (v == m_attachId) return;
    m_attachId = v;
    emit attachIdChanged();
}

void TerminalItem::setTmuxPane(const QString& v)
{
    if (v == m_tmuxPane) return;
    m_tmuxPane = v;
    emit tmuxPaneChanged();
}

void TerminalItem::setCwd(const QString& v)
{
    if (v == m_cwd) return;
    m_cwd = v;
    emit cwdChanged();
}

void TerminalItem::setFontFamily(const QString& in)
{
    // Empty = the system's monospace font.
    const QString v = in.isEmpty() ? QFontDatabase::systemFont(QFontDatabase::FixedFont).family() : in;
    if (v == m_fontFamily) return;
    m_fontFamily = v;
    updateMetrics();
    emit fontChanged();
}

void TerminalItem::setFontPointSize(qreal v)
{
    // 0 = the system's monospace size.
    if (v <= 0) {
        const QFont fixed = QFontDatabase::systemFont(QFontDatabase::FixedFont);
        v = fixed.pointSizeF() > 0 ? fixed.pointSizeF() : 11;
    }
    if (qFuzzyCompare(v, m_fontPointSize)) return;
    m_fontPointSize = v;
    updateMetrics();
    emit fontChanged();
}

void TerminalItem::setOptionsJson(const QString& v)
{
    if (v == m_optionsJson) return;
    m_optionsJson = v;
    if (m_session) {
        const QByteArray b = v.toUtf8();
        m_session->set_options(rs(b));
    }
    emit optionsChanged();
}

void TerminalItem::setCharSpacing(qreal v)
{
    v = qBound<qreal>(0.8, v, 1.5);
    if (qFuzzyCompare(v, m_charSpacing)) return;
    m_charSpacing = v;
    updateMetrics();
    emit optionsChanged();
}

void TerminalItem::setCursorBlink(bool v)
{
    if (v == m_cursorBlink) return;
    m_cursorBlink = v;
    m_blinkOn = true;
    if (v && hasActiveFocus()) m_blinkTimer->start();
    else m_blinkTimer->stop();
    if (m_cursorY >= 0) repaintRow(m_cursorY);
    emit optionsChanged();
}

bool TerminalItem::logging() const { return m_session && m_session->is_logging(); }

void TerminalItem::repaintRow(int row)
{
    if (row < 0 || row >= m_rows || row >= m_rowRuns.size()) return;
    paintRow(row, m_rowRuns[row]);
    m_rowUpload[row] = true;
    update();
}

void TerminalItem::repaintAll()
{
    for (int r = 0; r < m_rows && r < m_rowRuns.size(); ++r) {
        paintRow(r, m_rowRuns[r]);
        m_rowUpload[r] = true;
    }
    update();
}

void TerminalItem::setThemeJson(const QString& v)
{
    if (v == m_themeJson) return;
    m_themeJson = v;
    if (m_session) {
        const QByteArray b = v.toUtf8();
        m_session->set_theme(rs(b));
        m_forceFull = true;
        polish();
    }
    emit themeJsonChanged();
}

void TerminalItem::setCursorShape(int v)
{
    if (v == m_cursorShape) return;
    m_cursorShape = v;
    if (m_session) {
        m_session->set_cursor_style(uint8_t(v));
        polish();
    }
    emit cursorShapeChanged();
}

void TerminalItem::setScrollback(int v)
{
    if (v == m_scrollback) return;
    m_scrollback = v; // applies to terminals opened from now on
    emit scrollbackChanged();
}

void TerminalItem::setLineHeight(qreal v)
{
    v = qBound<qreal>(0.8, v, 2.0);
    if (qFuzzyCompare(v, m_lineHeight)) return;
    m_lineHeight = v;
    updateMetrics();
    emit fontChanged();
}

void TerminalItem::setPaddingSize(qreal v)
{
    v = qBound<qreal>(0, v, 40);
    if (qFuzzyCompare(v + 1, m_paddingSize + 1)) return;
    m_paddingSize = v;
    updateMetrics();
}

void TerminalItem::setCopyOnSelect(bool v)
{
    if (v == m_copyOnSelect) return;
    m_copyOnSelect = v;
    emit copyOnSelectChanged();
}

int TerminalItem::tmuxPaneId() const { return m_session ? m_session->tmux_pane_id() : -1; }

void TerminalItem::componentComplete()
{
    QQuickItem::componentComplete();
    m_complete = true;
    updateMetrics();
    if (width() > 0 && height() > 0) open();
}

void TerminalItem::itemChange(ItemChange change, const ItemChangeData& value)
{
    if (change == ItemSceneChange || change == ItemDevicePixelRatioHasChanged) {
        updateMetrics();
    }
    QQuickItem::itemChange(change, value);
}

QFont TerminalItem::fontFor(quint32 flags) const
{
    QFont f = m_font;
    if ((flags & FLAG_BOLD) && m_boldFont) f.setWeight(QFont::Bold);
    if ((flags & FLAG_ITALIC) && m_italicText) f.setItalic(true);
    return f;
}

const QRawFont& TerminalItem::rawFont(quint32 flags)
{
    const int i = ((flags & FLAG_BOLD) && m_boldFont ? 1 : 0) | ((flags & FLAG_ITALIC) && m_italicText ? 2 : 0);
    if (!m_rawValid[i]) {
        m_raw[i] = QRawFont::fromFont(fontFor(flags));
        m_rawValid[i] = true;
    }
    return m_raw[i];
}

void TerminalItem::updateMetrics()
{
    const qreal dpr = window() ? window()->effectiveDevicePixelRatio() : qApp->devicePixelRatio();
    m_dpr = dpr > 0 ? dpr : 1;
    // Work in device pixels so every cell sits on whole pixels.
    m_font = QFont(m_fontFamily);
    m_font.setStyleHint(QFont::Monospace);
    m_font.setFixedPitch(true);
    m_font.setPixelSize(qMax(4, qRound(m_fontPointSize * 96.0 / 72.0 * m_dpr)));
    m_font.setKerning(false);
    for (bool& v : m_rawValid) v = false;
    const QFontMetricsF fm(m_font);
    m_cw = qMax(1, qRound(fm.horizontalAdvance(QLatin1Char('M')) * m_charSpacing));
    const int glyphH = qCeil(fm.ascent() + fm.descent());
    m_ch = qMax(glyphH, qRound(glyphH * m_lineHeight));
    m_ascent = qRound(fm.ascent()) + (m_ch - glyphH) / 2;
    m_pad = qRound(m_paddingSize * m_dpr);
    emit metricsChanged();
    m_forceFull = true;
    m_rebuildNodes = true;
    resizeGrid();
    polish();
}

void TerminalItem::open()
{
    if (m_session || m_exited || !m_complete) return;
    const int cols = qMax(2, int((width() * m_dpr - 2 * m_pad) / m_cw));
    const int rows = qMax(1, int((height() * m_dpr - 2 * m_pad) / m_ch));
    try {
        const QByteArray a = m_attachId.toUtf8(), t = m_tmuxPane.toUtf8(), c = m_cwd.toUtf8();
        const QByteArray cmd = m_command.toUtf8(), term = m_termName.toUtf8();
        // Startup text only for a new shell, not one being reattached.
        const QByteArray input = m_attachId.isEmpty() && m_tmuxPane.isEmpty() && !m_startupInput.isEmpty()
            ? (m_startupInput + QStringLiteral("\n")).toUtf8() : QByteArray();
        auto box = tessera::session_open(rs(a), rs(t), rs(c), uint16_t(cols), uint16_t(rows), uint32_t(m_cw),
                                         uint32_t(m_ch), uint32_t(qMax(0, m_scrollback)), rs(cmd), rs(term), rs(input));
        m_session = box.into_raw();
        if (!m_themeJson.isEmpty()) {
            const QByteArray b = m_themeJson.toUtf8();
            m_session->set_theme(rs(b));
        }
        if (m_cursorShape != 0) m_session->set_cursor_style(uint8_t(m_cursorShape));
        if (!m_optionsJson.isEmpty()) {
            const QByteArray o = m_optionsJson.toUtf8();
            m_session->set_options(rs(o));
        }
    } catch (const std::exception& e) {
        m_exited = true;
        emit openFailed(QString::fromUtf8(e.what()));
        emit exitedChanged();
        return;
    }
    m_sessionId = qs(m_session->id());
    m_tmuxGateway = qs(m_session->tmux_gateway());
    emit sessionIdChanged();
    if (!m_tmuxGateway.isEmpty()) emit tmuxGatewayChanged();
    m_notifier = new QSocketNotifier(m_session->fd(), QSocketNotifier::Read, this);
    connect(m_notifier, &QSocketNotifier::activated, this, &TerminalItem::onReadable);
    m_forceFull = true;
    handleEvents();
    polish();
}

void TerminalItem::onReadable()
{
    if (!m_session) return;
    const quint32 sig = m_session->on_readable();
    if (sig & SIG_EVENTS) handleEvents();
    if (sig & SIG_DIRTY) {
        polish();
        emit activity();
    }
    if (sig & SIG_EXITED) {
        m_notifier->setEnabled(false);
        polish();
        if (!m_exited) {
            m_exited = true;
            emit exitedChanged();
        }
    }
}

void TerminalItem::handleEvents()
{
    if (!m_session) return;
    const QString json = qs(m_session->take_events());
    const QJsonArray events = QJsonDocument::fromJson(json.toUtf8()).array();
    for (const QJsonValue& v : events) {
        const QJsonObject e = v.toObject();
        const QString type = e.value(u"type").toString();
        if (type == u"title") {
            m_title = e.value(u"title").toString();
            emit titleChanged();
        } else if (type == u"pwd") {
            m_pwd = e.value(u"pwd").toString();
            emit pwdChanged();
        } else if (type == u"attached") {
            m_sessionId = e.value(u"id").toString();
            emit sessionIdChanged();
        } else if (type == u"bell") {
            if (m_visualBell) {
                m_flashUntil = QDateTime::currentMSecsSinceEpoch() + 140;
                update();
                QTimer::singleShot(150, this, [this] { update(); });
            }
            if (!m_silenceBell) QApplication::beep();
            emit bell();
        } else if (type == u"taken") {
            emit takenOver();
        } else if (type == u"clipboard") {
            QGuiApplication::clipboard()->setText(e.value(u"text").toString());
        } else if (type == u"grid") {
            m_forceFull = true;
        } else {
            if (type == u"tmuxStart") {
                m_tmuxGateway = e.value(u"gateway").toString();
                emit tmuxGatewayChanged();
            } else if (type == u"tmuxExit") {
                m_tmuxGateway.clear();
                emit tmuxGatewayChanged();
            }
            emit tmuxEvent(e.toVariantMap());
        }
    }
}

void TerminalItem::resizeGrid()
{
    if (!m_session) return;
    const int cols = qMax(2, int((width() * m_dpr - 2 * m_pad) / m_cw));
    const int rows = qMax(1, int((height() * m_dpr - 2 * m_pad) / m_ch));
    m_session->resize(uint16_t(cols), uint16_t(rows), uint32_t(m_cw), uint32_t(m_ch));
    polish();
}

void TerminalItem::geometryChange(const QRectF& g, const QRectF& old)
{
    QQuickItem::geometryChange(g, old);
    if (g.size() == old.size()) return;
    if (!m_session) {
        if (g.width() > 0 && g.height() > 0) open();
    } else {
        resizeGrid();
    }
    m_rebuildNodes = true;
    update();
}

void TerminalItem::updatePolish()
{
    pullFrame(m_forceFull);
    m_forceFull = false;
}

void TerminalItem::pullFrame(bool full)
{
    if (!m_session) return;
    tessera::Frame& f = *m_frame;
    if (!m_session->frame(f, full)) return;

    const QColor bg = QColor::fromRgb(QRgb(f.default_bg));
    if (bg != m_bg) {
        m_bg = bg;
        emit backgroundChanged();
    }
    m_fg = QColor::fromRgb(QRgb(f.default_fg));
    if (f.cols != m_cols || f.rows != m_rows) {
        m_cols = f.cols;
        m_rows = f.rows;
        m_rowImages.resize(m_rows);
        m_rowUpload.fill(true, m_rows);
        m_rebuildNodes = true;
        emit gridChanged();
    }
    if (f.scroll_total != m_scrollTotal || f.scroll_offset != m_scrollOffset || f.scroll_len != m_scrollLength) {
        m_scrollTotal = f.scroll_total;
        m_scrollOffset = f.scroll_offset;
        m_scrollLength = f.scroll_len;
        emit scrollChanged();
    }
    m_cursorVisible = f.cursor_visible;
    m_cursorX = f.cursor_x;
    m_cursorY = f.cursor_y;
    m_cursorStyle = f.cursor_style;
    m_cursorColor = QRgb(f.cursor_color);
    m_cursorText = qs(f.cursor_text);
    m_cursorWide = f.cursor_wide;

    // Runs arrive grouped by row, in row order.
    QHash<int, QVector<RunData>> byRow;
    for (const tessera::Run& r : f.runs) {
        byRow[r.row].append(RunData{r.col, r.width, qs(r.text), QRgb(r.fg), QRgb(r.bg), r.flags});
    }
    if (m_rowRuns.size() != m_rows) m_rowRuns.resize(m_rows);
    for (uint16_t row : f.dirty_rows) {
        if (row < m_rows) {
            m_rowRuns[row] = byRow.value(row);
            paintRow(row, m_rowRuns[row]);
            m_rowUpload[row] = true;
        }
    }
    if (m_cursorBlink && hasActiveFocus() && !f.dirty_rows.empty()) {
        // Typing shows the cursor at once and restarts the blink.
        m_blinkOn = true;
        m_blinkTimer->start();
    }
    update();
}

void TerminalItem::paintRow(int row, const QVector<RunData>& runs)
{
    const QSize size(qMax(1, m_cols * m_cw), m_ch);
    QImage& img = m_rowImages[row];
    if (img.size() != size || img.format() != QImage::Format_ARGB32_Premultiplied)
        img = QImage(size, QImage::Format_ARGB32_Premultiplied);
    QColor base = m_bg;
    base.setAlphaF(1 - m_transparency);
    img.fill(base);
    QPainter p(&img);
    if (m_cursorGuide && row == m_cursorY && m_cursorVisible) {
        QColor g = m_fg;
        g.setAlphaF(0.08);
        p.fillRect(QRect(0, 0, size.width(), m_ch), g);
    }
    p.setRenderHint(QPainter::TextAntialiasing);

    const qreal lineW = qMax<qreal>(1, qRound(m_dpr));
    auto drawText = [&](int x, const QString& text, QRgb color, quint32 flags, int cells) {
        p.setPen(QColor::fromRgb(color));
        const QRawFont& raw = rawFont(flags);
        QVector<quint32> glyphs = raw.isValid() ? raw.glyphIndexesForString(text) : QVector<quint32>();
        const bool allFound = !glyphs.isEmpty() && !glyphs.contains(0u) && glyphs.size() == text.size();
        if (allFound) {
            // One glyph per cell, each placed exactly at its column.
            QVector<QPointF> pos(glyphs.size());
            for (int i = 0; i < glyphs.size(); ++i) pos[i] = QPointF(x + i * m_cw, m_ascent);
            QGlyphRun gr;
            gr.setRawFont(raw);
            gr.setGlyphIndexes(glyphs);
            gr.setPositions(pos);
            p.drawGlyphRun(QPointF(0, 0), gr);
        } else {
            // Symbols the terminal font lacks: let Qt pick a fallback font,
            // centered in its cell(s) (CJK fallbacks are often narrower).
            const QFont f = fontFor(flags);
            p.setFont(f);
            const qreal adv = QFontMetricsF(f).horizontalAdvance(text);
            const qreal off = adv < cells * m_cw ? (cells * m_cw - adv) / 2 : 0;
            p.drawText(QPointF(x + off, m_ascent), text);
        }
    };

    for (const RunData& r : runs) {
        const int x = r.col * m_cw;
        const int w = r.width * m_cw;
        if (r.flags & FLAG_HAS_BG) {
            QColor bgc = QColor::fromRgb(r.bg);
            if (!m_keepOpaque) bgc.setAlphaF(1 - m_transparency);
            p.setCompositionMode(QPainter::CompositionMode_Source);
            p.fillRect(x, 0, w, m_ch, bgc);
            p.setCompositionMode(QPainter::CompositionMode_SourceOver);
        }
        if (r.flags & FLAG_INVISIBLE) continue;
        QRgb fg = r.fg;
        if (r.flags & FLAG_FAINT) fg = blend(fg, (r.flags & FLAG_HAS_BG) ? r.bg : m_bg.rgb(), 1 - m_faintOpacity);
        const bool blank = r.text.trimmed().isEmpty();
        if (!blank) drawText(x, r.text, fg, r.flags, r.width);
        if (r.flags & FLAG_UNDERLINE)
            p.fillRect(QRectF(x, qMin<qreal>(m_ch - lineW, m_ascent + lineW * 1.5), w, lineW), QColor::fromRgb(fg));
        if (r.flags & FLAG_STRIKE) p.fillRect(QRectF(x, m_ascent * 0.65, w, lineW), QColor::fromRgb(fg));
    }

    const bool blinkHidden = m_cursorBlink && hasActiveFocus() && !m_blinkOn;
    if (m_cursorVisible && !blinkHidden && row == m_cursorY && m_cursorX >= 0) {
        const QRectF cell(m_cursorX * m_cw, 0, (m_cursorWide ? 2 : 1) * m_cw, m_ch);
        const QColor cc = QColor::fromRgb(m_cursorColor);
        const bool focused = hasActiveFocus();
        if (!focused || m_cursorStyle == 3) {
            p.setPen(QPen(cc, lineW));
            p.setBrush(Qt::NoBrush);
            p.drawRect(cell.adjusted(lineW / 2, lineW / 2, -lineW / 2, -lineW / 2));
        } else if (m_cursorStyle == 1) {
            p.fillRect(QRectF(cell.x(), 0, qMax<qreal>(2, qRound(m_dpr * 1.5)), m_ch), cc);
        } else if (m_cursorStyle == 2) {
            p.fillRect(QRectF(cell.x(), m_ch - 2 * lineW, cell.width(), 2 * lineW), cc);
        } else {
            p.fillRect(cell, cc);
            if (!m_cursorText.trimmed().isEmpty()) drawText(int(cell.x()), m_cursorText, m_bg.rgb(), 0, m_cursorWide ? 2 : 1);
        }
    }
}

QSGNode* TerminalItem::updatePaintNode(QSGNode* old, UpdatePaintNodeData*)
{
    auto* node = static_cast<TerminalNode*>(old);
    QQuickWindow* win = window();
    if (!win) return old;
    if (!node) {
        node = new TerminalNode;
        node->bg = win->createRectangleNode();
        node->appendChildNode(node->bg);
        m_rebuildNodes = true;
    }
    node->bg->setRect(boundingRect());
    QColor base = m_bg;
    base.setAlphaF(1 - m_transparency);
    node->bg->setColor(base);

    if (m_rebuildNodes || node->rows.size() != m_rows) {
        node->clearRows();
        for (int i = 0; i < m_rows; ++i) {
            QSGImageNode* n = win->createImageNode();
            n->setOwnsTexture(true);
            n->setFiltering(QSGTexture::Nearest);
            node->rows.append(n);
        }
        m_rowUpload.fill(true, m_rows);
        m_rebuildNodes = false;
    }

    // Snap the grid to whole device pixels.
    const qreal pad = m_pad / m_dpr;
    for (int i = 0; i < m_rows; ++i) {
        QSGImageNode* n = node->rows[i];
        const QImage& img = m_rowImages.value(i);
        if (img.isNull()) continue;
        if (m_rowUpload.value(i)) {
            n->setTexture(win->createTextureFromImage(img));
            m_rowUpload[i] = false;
            if (!n->parent()) node->appendChildNode(n);
        }
        if (!n->parent()) continue;
        n->setRect(QRectF(pad, pad + i * m_ch / m_dpr, img.width() / m_dpr, img.height() / m_dpr));
        n->setSourceRect(QRectF(0, 0, img.width(), img.height()));
    }

    // Visual bell flash and Find Cursor highlight, on top.
    const qint64 now = QDateTime::currentMSecsSinceEpoch();
    if (!node->overlay) {
        node->overlay = win->createRectangleNode();
        node->appendChildNode(node->overlay);
    } else {
        // Keep it last (row nodes may have joined since).
        node->removeChildNode(node->overlay);
        node->appendChildNode(node->overlay);
    }
    if (now < m_flashUntil) {
        node->overlay->setRect(boundingRect());
        node->overlay->setColor(QColor(255, 255, 255, 38));
    } else if (now < m_findCursorUntil && m_cursorX >= 0) {
        const QRectF c((m_pad + m_cursorX * m_cw) / m_dpr, (m_pad + m_cursorY * m_ch) / m_dpr, m_cw / m_dpr, m_ch / m_dpr);
        node->overlay->setRect(c.adjusted(-30, -30, 30, 30));
        QColor a = QColor::fromRgb(m_cursorColor);
        a.setAlpha(70);
        node->overlay->setColor(a);
    } else {
        node->overlay->setRect(QRectF());
        node->overlay->setColor(Qt::transparent);
    }
    return node;
}

// ---- input -------------------------------------------------------------

void TerminalItem::keyPressEvent(QKeyEvent* e)
{
    if (!m_session || m_exited) return QQuickItem::keyPressEvent(e);
    const QByteArray text = e->text().toUtf8();
    if (m_session->key_event(e->key(), uint32_t(e->modifiers()), rs(text), false, e->nativeVirtualKey())) {
        e->accept();
        forwardKey(e, false);
        if (m_hasSelection && !(e->modifiers() & (Qt::ControlModifier | Qt::MetaModifier))
            && e->key() < Qt::Key_Shift) {
            // Typing clears the highlight (the copied text stays on the clipboard).
            m_session->clear_selection();
            m_hasSelection = false;
            emit selectionChanged();
            polish();
        }
    } else {
        QQuickItem::keyPressEvent(e);
    }
}

void TerminalItem::keyReleaseEvent(QKeyEvent* e)
{
    if (!m_session || m_exited) return QQuickItem::keyReleaseEvent(e);
    const QByteArray text = e->text().toUtf8();
    m_session->key_event(e->key(), uint32_t(e->modifiers()), rs(text), true, e->nativeVirtualKey());
    forwardKey(e, true);
    e->accept();
}

void TerminalItem::forwardKey(QKeyEvent* e, bool release)
{
    for (const QVariant& v : std::as_const(m_broadcastTo)) {
        auto* peer = qobject_cast<TerminalItem*>(v.value<QObject*>());
        if (peer && peer != this) peer->receiveKey(e->key(), int(e->modifiers()), e->text(), release);
    }
}

void TerminalItem::receiveKey(int key, int mods, const QString& text, bool release)
{
    if (!m_session || m_exited) return;
    const QByteArray t = text.toUtf8();
    m_session->key_event(key, uint32_t(mods), rs(t), release, 0);
}

void TerminalItem::receiveText(const QString& text)
{
    if (!m_session || m_exited) return;
    const QByteArray t = text.toUtf8();
    m_session->send_text(rs(t));
}

void TerminalItem::inputMethodEvent(QInputMethodEvent* e)
{
    if (m_session && !e->commitString().isEmpty()) {
        const QByteArray t = e->commitString().toUtf8();
        m_session->send_text(rs(t));
        for (const QVariant& v : std::as_const(m_broadcastTo)) {
            auto* peer = qobject_cast<TerminalItem*>(v.value<QObject*>());
            if (peer && peer != this) peer->receiveText(e->commitString());
        }
    }
    e->accept();
}

QVariant TerminalItem::inputMethodQuery(Qt::InputMethodQuery query) const
{
    switch (query) {
    case Qt::ImEnabled:
        return true;
    case Qt::ImCursorRectangle:
        return QRectF((m_pad + qMax(0, m_cursorX) * m_cw) / m_dpr, (m_pad + qMax(0, m_cursorY) * m_ch) / m_dpr,
                      m_cw / m_dpr, m_ch / m_dpr);
    case Qt::ImHints:
        return int(Qt::ImhNoPredictiveText | Qt::ImhNoAutoUppercase);
    default:
        return QQuickItem::inputMethodQuery(query);
    }
}

static int buttonCode(Qt::MouseButton b)
{
    switch (b) {
    case Qt::LeftButton: return 1;
    case Qt::RightButton: return 2;
    case Qt::MiddleButton: return 3;
    default: return 0;
    }
}

void TerminalItem::mouse(int action, int button, QPointF pos, Qt::KeyboardModifiers mods)
{
    if (!m_session) return;
    const bool changed = m_session->mouse_event(uint8_t(action), uint8_t(button), float(pos.x() * m_dpr),
                                                float(pos.y() * m_dpr), uint32_t(mods), uint32_t(width() * m_dpr),
                                                uint32_t(height() * m_dpr), uint32_t(m_cw), uint32_t(m_ch),
                                                uint32_t(m_pad));
    if (changed) polish();
}

void TerminalItem::mousePressEvent(QMouseEvent* e)
{
    forceActiveFocus(Qt::MouseFocusReason);
    const bool appWantsMouse = m_session && m_session->wants_mouse(uint32_t(e->modifiers()));
    if (e->button() == Qt::RightButton && !appWantsMouse) {
        if (m_rightClickPastes) paste();
        else emit contextMenuRequested(e->position().x(), e->position().y());
        e->accept();
        return;
    }
    if (e->button() == Qt::MiddleButton && !appWantsMouse) {
        if (m_middleClickPastes) pasteSelection();
        e->accept();
        return;
    }
    m_selecting = e->button() == Qt::LeftButton;
    mouse(0, buttonCode(e->button()), e->position(), e->modifiers());
    e->accept();
}

void TerminalItem::mouseMoveEvent(QMouseEvent* e)
{
    mouse(2, 0, e->position(), e->modifiers());
    m_lastPointer = e->position();
    // Dragging a selection past the top or bottom keeps scrolling.
    if (m_selecting) {
        const bool outside = e->position().y() < m_pad / m_dpr || e->position().y() > height() - m_pad / m_dpr;
        if (outside && !m_autoscroll->isActive()) m_autoscroll->start();
    }
    e->accept();
}

void TerminalItem::mouseReleaseEvent(QMouseEvent* e)
{
    mouse(1, buttonCode(e->button()), e->position(), e->modifiers());
    m_autoscroll->stop();
    if (e->button() == Qt::LeftButton && m_selecting) {
        m_selecting = false;
        publishSelection();
    }
    e->accept();
}

void TerminalItem::hoverMoveEvent(QHoverEvent* e)
{
    // Motion reports for programs that track the pointer (no buttons held).
    if (m_session) mouse(2, 0, e->position(), e->modifiers());
    QQuickItem::hoverMoveEvent(e);
}

void TerminalItem::scrollPixels(qreal dy)
{
    // dy > 0: content moves down (towards older output).
    if (!m_session) return;
    m_scrollRemainder += dy;
    const qreal rowH = m_ch / m_dpr;
    const int rows = int(m_scrollRemainder / rowH);
    if (rows != 0) {
        m_scrollRemainder -= rows * rowH;
        m_session->scroll_lines(-rows);
        polish();
    }
}

void TerminalItem::scrollToRow(qreal row)
{
    if (!m_session) return;
    m_momentum->stop();
    m_session->scroll_to_row(uint64_t(qMax<qreal>(0, row)));
    polish();
}

void TerminalItem::wheelEvent(QWheelEvent* e)
{
    // Scrollback, smoothly: touchpads scroll by pixels and glide on after a
    // flick; a mouse wheel moves three rows a notch. Programs that take the
    // mouse (or full-screen ones) get wheel events/arrow keys as before.
    if (m_session && m_session->scrolls_locally(uint32_t(e->modifiers()))) {
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        const bool touchpad = !e->pixelDelta().isNull();
        if (e->phase() == Qt::ScrollBegin || (!touchpad && e->phase() == Qt::NoScrollPhase)) m_momentum->stop();
        if (touchpad) {
            const qreal dy = e->pixelDelta().y();
            if (e->phase() == Qt::ScrollEnd) {
                if (std::abs(m_velocity) > 0.15 && now - m_lastWheelMs < 80) m_momentum->start();
                else m_velocity = 0;
            } else if (e->phase() == Qt::ScrollMomentum) {
                scrollPixels(dy);   // the platform's own momentum (macOS)
            } else {
                m_momentum->stop();
                const qreal dt = qMax<qint64>(1, now - m_lastWheelMs);
                const qreal v = dy / dt;
                m_velocity = dt > 100 ? v : 0.6 * v + 0.4 * m_velocity;
                m_lastWheelMs = now;
                scrollPixels(dy);
            }
        } else {
            // Three rows a notch (fractions for high-resolution wheels), eased.
            m_smoothPending += e->angleDelta().y() / 120.0 * 3 * (m_ch / m_dpr);
            if (!m_smooth->isActive()) m_smooth->start();
        }
        e->accept();
        return;
    }
    m_wheelAccum += e->angleDelta().y();
    while (std::abs(m_wheelAccum) >= 120) {
        const bool up = m_wheelAccum > 0;
        m_wheelAccum += up ? -120 : 120;
        mouse(0, up ? 4 : 5, e->position(), e->modifiers());
    }
    e->accept();
}

void TerminalItem::publishSelection()
{
    if (!m_session) return;
    const QString text = qs(m_session->selected_text());
    const bool has = !text.isEmpty();
    if (has) {
        // Copy on select (iTerm2's default, a preference); the X/Wayland
        // primary selection always.
        if (m_copyOnSelect) QGuiApplication::clipboard()->setText(text, QClipboard::Clipboard);
        if (QGuiApplication::clipboard()->supportsSelection())
            QGuiApplication::clipboard()->setText(text, QClipboard::Selection);
    }
    if (has != m_hasSelection) {
        m_hasSelection = has;
        emit selectionChanged();
    }
}

void TerminalItem::hoverEnterEvent(QHoverEvent* e)
{
    if (m_focusFollowsMouse && !hasActiveFocus()) forceActiveFocus(Qt::MouseFocusReason);
    QQuickItem::hoverEnterEvent(e);
}

void TerminalItem::focusInEvent(QFocusEvent* e)
{
    QQuickItem::focusInEvent(e);
    m_blinkOn = true;
    if (m_cursorBlink) m_blinkTimer->start();
    if (m_session) m_session->focus(true);
    m_forceFull = true;
    polish();
}

void TerminalItem::focusOutEvent(QFocusEvent* e)
{
    QQuickItem::focusOutEvent(e);
    m_blinkTimer->stop();
    m_blinkOn = true;
    if (m_session) m_session->focus(false);
    m_forceFull = true;
    polish();
}

// ---- actions for QML ---------------------------------------------------

void TerminalItem::copy()
{
    if (!m_session) return;
    const QString text = qs(m_session->selected_text());
    if (!text.isEmpty()) QGuiApplication::clipboard()->setText(text);
}

void TerminalItem::paste() { pasteText(QGuiApplication::clipboard()->text()); }

void TerminalItem::pasteText(const QString& text)
{
    if (!m_session || text.isEmpty()) return;
    const QByteArray t = text.toUtf8();
    m_session->paste(rs(t));
    for (const QVariant& v : std::as_const(m_broadcastTo)) {
        auto* peer = qobject_cast<TerminalItem*>(v.value<QObject*>());
        if (peer && peer != this && peer->m_session) peer->m_session->paste(rs(t));
    }
}

void TerminalItem::pasteSelection()
{
    const QClipboard::Mode mode = QGuiApplication::clipboard()->supportsSelection() ? QClipboard::Selection : QClipboard::Clipboard;
    pasteText(QGuiApplication::clipboard()->text(mode));
}

void TerminalItem::pasteSlowly()
{
    m_slowPaste += QGuiApplication::clipboard()->text().toUtf8();
    m_pasteTimer->start();
}

QString TerminalItem::selectedText() const { return m_session ? qs(m_session->selected_text()) : QString(); }

QString TerminalItem::contentsText() const { return m_session ? qs(m_session->contents_text()) : QString(); }

void TerminalItem::copyWithStyles()
{
    if (!m_session) return;
    auto* mime = new QMimeData;
    mime->setHtml(qs(m_session->selected_html()));
    mime->setText(qs(m_session->selected_text()));
    QGuiApplication::clipboard()->setMimeData(mime);
}

void TerminalItem::copyWithControlSequences()
{
    if (!m_session) return;
    QGuiApplication::clipboard()->setText(qs(m_session->selected_vt()));
}

void TerminalItem::resetTerminal()
{
    if (!m_session) return;
    m_session->reset();
    m_forceFull = true;
    polish();
}

void TerminalItem::restart()
{
    delete m_notifier;
    m_notifier = nullptr;
    if (m_session) {
        m_session->kill();
        rust::Box<tessera::Session>::from_raw(m_session);
        m_session = nullptr;
    }
    m_attachId.clear();
    m_tmuxPane.clear();
    m_exited = false;
    emit exitedChanged();
    open();
}

void TerminalItem::showMessage(const QString& text)
{
    if (!m_session) return;
    const QByteArray t = text.toUtf8();
    m_session->show_message(rs(t));
    m_forceFull = true;
    polish();
}

bool TerminalItem::startLogging(const QString& path)
{
    if (!m_session) return false;
    QString p = path;
    if (p.startsWith(u"file://")) p = QUrl(p).toLocalFile();
    const QByteArray b = p.toUtf8();
    const bool ok = m_session->log_to_file(rs(b));
    emit loggingChanged();
    return ok;
}

void TerminalItem::stopLogging()
{
    if (!m_session) return;
    m_session->log_to_file(rs(QByteArray()));
    emit loggingChanged();
}

int TerminalItem::find(const QString& query, bool regex)
{
    if (!m_session) return 0;
    const QByteArray q = query.toUtf8();
    const int n = m_session->find(rs(q), regex);
    polish();
    return n;
}

int TerminalItem::findStep(int dir)
{
    if (!m_session) return -1;
    const int i = m_session->find_step(dir);
    polish();
    return i;
}

int TerminalItem::findRefresh()
{
    if (!m_session) return 0;
    const int n = m_session->find_refresh();
    polish();
    return n;
}

int TerminalItem::findIndex() const { return m_session ? m_session->find_index() : -1; }

void TerminalItem::endFind()
{
    if (!m_session) return;
    m_session->find_clear();
    polish();
}

void TerminalItem::findCursor()
{
    m_findCursorUntil = QDateTime::currentMSecsSinceEpoch() + 900;
    update();
    QTimer::singleShot(950, this, [this] { update(); });
}

void TerminalItem::sendText(const QString& text)
{
    if (!m_session) return;
    const QByteArray t = text.toUtf8();
    m_session->send_text(rs(t));
}

void TerminalItem::killSession()
{
    if (m_session) m_session->kill();
}

void TerminalItem::tmuxCommand(const QString& command)
{
    if (!m_session) return;
    const QByteArray c = command.toUtf8();
    m_session->tmux_command(rs(c));
}

void TerminalItem::selectAll()
{
    if (!m_session) return;
    m_session->select_all();
    publishSelection();
    polish();
}

void TerminalItem::clearScrollback()
{
    if (!m_session) return;
    m_session->clear_scrollback();
    m_forceFull = true;
    polish();
}

void TerminalItem::scrollToBottom()
{
    if (!m_session) return;
    m_session->scroll_to_bottom();
    polish();
}
