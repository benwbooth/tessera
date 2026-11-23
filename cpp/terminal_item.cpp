#include "terminal_item.h"

#include "tessera/src/bridge.cxxqt.h"

#include <QtCore/QJsonArray>
#include <QtCore/QHash>
#include <QtCore/QSet>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtCore/QDateTime>
#include <QtCore/QMimeData>
#include <QtCore/QUrl>
#include <QtGui/QClipboard>
#include <QtGui/QDesktopServices>
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
#include <algorithm>
#include <functional>
#include <QtGui/QPainterPath>

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
    QVector<QSGRectangleNode*> marks;
    // Pictures (kitty graphics): their nodes, and each image's texture with
    // the generation it was made from.
    QVector<QSGImageNode*> images;
    QHash<quint32, QPair<quint64, QSGTexture*>> textures;
    ~TerminalNode() override
    {
        for (QSGImageNode* n : rows)
            if (!n->parent()) delete n;
        for (QSGImageNode* n : images)
            if (!n->parent()) delete n;
        for (auto& t : textures) delete t.second;
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
        m_velocity *= m_decay;
        if (std::abs(m_velocity) < 0.015) {
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
        // ...and faster the longer it's held there.
        const qreal held = (QDateTime::currentMSecsSinceEpoch() - m_autoscrollStart) / 700.0;
        const uint32_t rows = uint32_t(qBound<qreal>(1, (1 + std::pow(dist / rowH, 1.4)) * (1 + held), 80));
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
    m_openTimer = new QTimer(this);
    m_openTimer->setSingleShot(true);
    m_openTimer->setInterval(60);
    connect(m_openTimer, &QTimer::timeout, this, [this] { if (width() > 0 && height() > 0) open(); });
    m_resizeTimer = new QTimer(this);
    m_resizeTimer->setSingleShot(true);
    m_resizeTimer->setInterval(80);
    connect(m_resizeTimer, &QTimer::timeout, this, [this] { resizeGrid(); });
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
    delete m_wakeNotifier;
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
bool TerminalItem::recording() const { return m_session && m_session->is_recording(); }

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
    // The theme's colors right away: a new terminal (no shell yet) mustn't
    // show a stand-in grey, and neither must the window's chrome (it follows
    // the active terminal's background).
    {
        const QJsonObject t = QJsonDocument::fromJson(v.toUtf8()).object();
        const QColor bg(t.value(u"background").toString());
        const QColor fg(t.value(u"foreground").toString());
        if (bg.isValid() && bg != m_bg) {
            m_bg = bg;
            emit backgroundChanged();
            update();
        }
        if (fg.isValid()) m_fg = fg;
    }
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


void TerminalItem::componentComplete()
{
    QQuickItem::componentComplete();
    m_complete = true;
    updateMetrics();
    if (width() > 0 && height() > 0) m_openTimer->start();
}

void TerminalItem::itemChange(ItemChange change, const ItemChangeData& value)
{
    if (change == ItemSceneChange || change == ItemDevicePixelRatioHasChanged) {
        updateMetrics();
    }
    // (After the layouts around it have settled: a moment after it's shown.)
    if (change == ItemVisibleHasChanged && value.boolValue && m_resizeWhenShown) m_resizeTimer->start();
    // (Shown again (a notebook card's terminal app moved between its card and
    // the whole pane): all of it drawn afresh, as focus does; what changed
    // while it was hidden isn't all in the next frame.)
    if (change == ItemVisibleHasChanged && value.boolValue) {
        m_forceFull = true;
        m_rebuildNodes = true;
        polish();
        update();
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
    // Debounced like a window resize: holding Ctrl+= resizes the shell once,
    // at the end, rather than at every step.
    if (m_session) m_resizeTimer->start(); else resizeGrid();
    polish();
}

void TerminalItem::open()
{
    if (m_session || m_exited || !m_complete || !m_ready) return;
    const int cols = qMax(2, int((width() * m_dpr - 2 * m_pad) / m_cw));
    const int rows = qMax(1, int((height() * m_dpr - 2 * m_pad) / m_ch));
    try {
        const QByteArray a = m_attachId.toUtf8(), c = m_cwd.toUtf8();
        const QByteArray cmd = m_command.toUtf8(), term = m_termName.toUtf8(), host = m_host.toUtf8();
        // Startup text only for a new shell, not one being reattached.
        const QByteArray input = m_attachId.isEmpty() && !m_startupInput.isEmpty()
            ? (m_startupInput + QStringLiteral("\n")).toUtf8() : QByteArray();
        auto box = tessera::session_open(rs(a), rs(c), uint16_t(cols), uint16_t(rows), uint32_t(m_cw),
                                         uint32_t(m_ch), uint32_t(qMax(0, m_scrollback)), rs(cmd), rs(term), rs(input), rs(host));
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
        if (!m_preloadFile.isEmpty()) {
            const QByteArray pf = m_preloadFile.toUtf8();
            m_session->preload(rs(pf));
        }
    } catch (const std::exception& e) {
        m_exited = true;
        emit openFailed(QString::fromUtf8(e.what()));
        emit exitedChanged();
        return;
    }
    m_sessionId = qs(m_session->id());
    emit sessionIdChanged();
    m_notifier = new QSocketNotifier(m_session->fd(), QSocketNotifier::Read, this);
    connect(m_notifier, &QSocketNotifier::activated, this, &TerminalItem::onReadable);
    // (Kept cells, read in the background, come through this one.)
    auto* wake = new QSocketNotifier(m_session->wake_fd(), QSocketNotifier::Read, this);
    connect(wake, &QSocketNotifier::activated, this, &TerminalItem::onReadable);
    m_wakeNotifier = wake;
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
        } else if (type == u"aliases") {
            m_aliases = QString::fromUtf8(QJsonDocument(e.value(u"map").toObject()).toJson(QJsonDocument::Compact));
            emit aliasesChanged();
        } else if (type == u"cell") {
            emit cellsChanged(quint64(e.value(u"id").toDouble()), e.value(u"done").toBool());
        } else if (type == u"rich") {
            m_richKind = e.value(u"kind").toString();
        } else if (type == u"command") {
            m_richKind.clear();
            // (A "rich" event for this command comes first, in the same batch.)
            for (const QJsonValue& w : events) {
                const QJsonObject o = w.toObject();
                if (o.value(u"type").toString() == u"rich") m_richKind = o.value(u"kind").toString();
            }
            m_lastStatus = e.value(u"status").toInt();
            m_lastDuration = qint64(e.value(u"ms").toDouble());
            emit commandFinished();
        } else if (type == u"prompt") {
            const bool at = e.value(u"state").toString() != u"C";
            if (at != m_atPrompt) {
                m_atPrompt = at;
                emit promptChanged();
            }
        } else {
            // (The rest, a trigger's notification say, for the window.)
            emit sessionEvent(e.toVariantMap());
        }
    }
}

void TerminalItem::resizeGrid()
{
    if (!m_session) return;
    // Hidden (another tab): its size there means nothing (layouts around it
    // collapse while hidden), and a resize makes the shell redraw its prompt,
    // which flickers when the tab comes back. Done when it's shown instead.
    if (!isVisible()) {
        m_resizeWhenShown = true;
        return;
    }
    m_resizeWhenShown = false;
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
        if (g.width() > 0 && g.height() > 0) m_openTimer->start();
    } else {
        m_resizeTimer->start();
    }
    m_rebuildNodes = true;
    m_forceFull = true;
    polish();
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
    {
        QVector<ImageSpot> spots;
        spots.reserve(int(f.images.size()));
        for (const tessera::ImagePlace& p : f.images)
            spots.append(ImageSpot{ p.id, p.generation, p.x, p.y, int(p.width), int(p.height), int(p.src_x), int(p.src_y), int(p.src_w), int(p.src_h), p.z });
        if (spots != m_images) {
            m_images = spots;
            update();
        }
    }

    const QColor bg = QColor::fromRgb(QRgb(f.default_bg));
    if (bg != m_bg) {
        m_bg = bg;
        emit backgroundChanged();
    }
    m_fg = QColor::fromRgb(QRgb(f.default_fg));
    if (const bool fs = m_session->full_screen(); fs != m_fullScreen) {
        m_fullScreen = fs;
        emit promptChanged();
    }
    // A program starting or ending changes the pointer without it moving.
    if (isUnderMouse() && !(m_hoverMods & Qt::ControlModifier)) updatePointer(QPointF(-1, -1));
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
    if (m_rowMark.size() != m_rows) m_rowMark.fill(false, m_rows);
    for (uint16_t row : f.dirty_rows)
        if (row < m_rows) m_rowMark[row] = false;
    for (uint16_t row : f.mark_rows)
        if (row < m_rows) m_rowMark[row] = true;
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

// Box drawing (U+2500-257F) and block elements (U+2580-259F) are drawn as
// shapes filling the whole cell, so lines join across rows and columns
// whatever the line height (a font's glyphs stop short of a taller cell).
// Each entry: the up, right, down and left arms, 2 bits each
// (0 none, 1 light, 2 heavy, 3 double). Dashes and diagonals: 0 (the font).
static const quint8 kBoxLines[128] = {
    0x44, 0x88, 0x11, 0x22, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x14, 0x18, 0x24, 0x28,
    0x50, 0x90, 0x60, 0xa0, 0x05, 0x09, 0x06, 0x0a,
    0x41, 0x81, 0x42, 0x82, 0x15, 0x19, 0x16, 0x25,
    0x26, 0x1a, 0x29, 0x2a, 0x51, 0x91, 0x52, 0x61,
    0x62, 0x92, 0xa1, 0xa2, 0x54, 0x94, 0x58, 0x98,
    0x64, 0xa4, 0x68, 0xa8, 0x45, 0x85, 0x49, 0x89,
    0x46, 0x86, 0x4a, 0x8a, 0x55, 0x95, 0x59, 0x99,
    0x56, 0x65, 0x66, 0x96, 0x5a, 0xa5, 0x69, 0x9a,
    0xa9, 0xa6, 0x6a, 0xaa, 0x00, 0x00, 0x00, 0x00,
    0xcc, 0x33, 0x1c, 0x34, 0x3c, 0xd0, 0x70, 0xf0,
    0x0d, 0x07, 0x0f, 0xc1, 0x43, 0xc3, 0x1d, 0x37,
    0x3f, 0xd1, 0x73, 0xf3, 0xdc, 0x74, 0xfc, 0xcd,
    0x47, 0xcf, 0xdd, 0x77, 0xff, 0x14, 0x50, 0x41,
    0x05, 0x00, 0x00, 0x00, 0x40, 0x01, 0x04, 0x10,
    0x80, 0x02, 0x08, 0x20, 0x48, 0x21, 0x84, 0x12,
};

static bool isBoxChar(QChar c)
{
    const ushort u = c.unicode();
    if (u >= 0x2580 && u <= 0x259f) return true;
    return u >= 0x2500 && u <= 0x257f && kBoxLines[u - 0x2500] != 0;
}

static void drawBoxChar(QPainter& p, int x, int cw, int ch, ushort u, const QColor& color, int light)
{
    if (u >= 0x2580) {
        // Block elements.
        auto fill = [&](qreal x0, qreal y0, qreal x1, qreal y1, qreal a = 1) {
            QColor c = color;
            c.setAlphaF(c.alphaF() * a);
            p.fillRect(QRect(x + qRound(x0 * cw), qRound(y0 * ch), qRound(x1 * cw) - qRound(x0 * cw), qRound(y1 * ch) - qRound(y0 * ch)), c);
        };
        if (u == 0x2580) fill(0, 0, 1, 0.5);
        else if (u <= 0x2588) fill(0, 1 - (u - 0x2580) / 8.0, 1, 1);          // lower eighths .. full
        else if (u <= 0x258f) fill(0, 0, (0x2590 - u) / 8.0, 1);              // left eighths
        else if (u == 0x2590) fill(0.5, 0, 1, 1);
        else if (u <= 0x2593) fill(0, 0, 1, 1, (u - 0x2590) * 0.25);          // shades
        else if (u == 0x2594) fill(0, 0, 1, 0.125);
        else if (u == 0x2595) fill(0.875, 0, 1, 1);
        else {
            // Quadrants: bits for upper-left, upper-right, lower-left, lower-right.
            static const quint8 q[10] = { 4, 8, 1, 1 | 4 | 8, 1 | 8, 1 | 2 | 4, 1 | 2 | 8, 2, 2 | 4, 2 | 4 | 8 };
            const quint8 m = q[u - 0x2596];
            if (m & 1) fill(0, 0, 0.5, 0.5);
            if (m & 2) fill(0.5, 0, 1, 0.5);
            if (m & 4) fill(0, 0.5, 0.5, 1);
            if (m & 8) fill(0.5, 0.5, 1, 1);
        }
        return;
    }
    const quint8 code = kBoxLines[u - 0x2500];
    const int up = code & 3, rt = (code >> 2) & 3, dn = (code >> 4) & 3, lf = (code >> 6) & 3;
    const int heavy = qMax(light + 1, 2 * light);
    const int g = light + qMax(1, light / 2) ;             // half the gap of a double line
    const int cx = x + cw / 2, cy = ch / 2;                 // the cell's center lines
    auto widthOf = [&](int w) { return w == 2 ? heavy : light; };

    if (u >= 0x256d && u <= 0x2570) {
        // Rounded corners.
        QPainterPath path;
        const qreal r = qMin(cw, ch) / 2.0, mx = cx + (light % 2) * 0.5, my = cy + (light % 2) * 0.5;
        const qreal ex = rt ? x + cw : x, ey = dn ? ch : 0;
        const qreal sx = rt ? 1 : -1, sy = dn ? 1 : -1;
        path.moveTo(ex, my);
        path.lineTo(mx + sx * r, my);
        path.quadTo(mx, my, mx, my + sy * r);
        path.lineTo(mx, ey);
        p.save();
        p.setRenderHint(QPainter::Antialiasing);
        p.setPen(QPen(color, light, Qt::SolidLine, Qt::FlatCap));
        p.setBrush(Qt::NoBrush);
        p.drawPath(path);
        p.restore();
        return;
    }

    const bool vDouble = up == 3 || dn == 3, hDouble = lf == 3 || rt == 3;
    // A vertical arm (dir -1 up, +1 down) of weight w.
    auto vArm = [&](int w, int dir) {
        if (!w) return;
        const int end = dir < 0 ? 0 : ch;
        if (w == 3) {
            for (int side = -1; side <= 1; side += 2) {
                // The line on a side that has a horizontal arm stops at it
                // (inside the junction); the other runs past the center.
                const bool inner = side < 0 ? lf != 0 : rt != 0;
                const int from = cy + dir * (inner ? g : -g) + (dir < 0 ? light : 0) * (inner ? 1 : 0);
                const int lx = cx + side * g - light / 2;
                const int a = qMin(from, end), b = qMax(from, end);
                p.fillRect(QRect(lx, a, light, b - a), color);
            }
            return;
        }
        const int t = widthOf(w);
        const int reach = hDouble ? g + light / 2 + 1 : (qMax(widthOf(lf), widthOf(rt)) + 1) / 2;
        const int from = cy - dir * reach;
        const int a = qMin(from, end), b = qMax(from, end);
        p.fillRect(QRect(cx - t / 2, a, t, b - a), color);
    };
    auto hArm = [&](int w, int dir) {
        if (!w) return;
        const int end = dir < 0 ? x : x + cw;
        if (w == 3) {
            for (int side = -1; side <= 1; side += 2) {
                const bool inner = side < 0 ? up != 0 : dn != 0;
                const int from = cx + dir * (inner ? g : -g) + (dir < 0 ? light : 0) * (inner ? 1 : 0);
                const int ly = cy + side * g - light / 2;
                const int a = qMin(from, end), b = qMax(from, end);
                p.fillRect(QRect(a, ly, b - a, light), color);
            }
            return;
        }
        const int t = widthOf(w);
        const int reach = vDouble ? g + light / 2 + 1 : (qMax(widthOf(up), widthOf(dn)) + 1) / 2;
        const int from = cx - dir * reach;
        const int a = qMin(from, end), b = qMax(from, end);
        p.fillRect(QRect(a, cy - t / 2, b - a, t), color);
    };
    vArm(up, -1);
    vArm(dn, 1);
    hArm(lf, -1);
    hArm(rt, 1);
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
    // `room`: cells the glyph may use (its own, plus a blank neighbor).
    std::function<void(int, const QString&, QRgb, quint32, int, int)> drawText;
    drawText = [&](int x, const QString& text, QRgb color, quint32 flags, int cells, int roomCells) {
        // Box drawing and blocks: shapes, one cell per character.
        if (text.size() == cells && std::any_of(text.begin(), text.end(), isBoxChar)) {
            int start = 0;
            for (int i = 0; i <= text.size(); ++i) {
                if (i < text.size() && !isBoxChar(text[i])) continue;
                if (i > start) drawText(x + start * m_cw, text.mid(start, i - start), color, flags, i - start, 0);
                if (i < text.size()) drawBoxChar(p, x + i * m_cw, m_cw, m_ch, text[i].unicode(), QColor::fromRgb(color), int(lineW));
                start = i + 1;
            }
            return;
        }
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
            // Symbols the terminal font lacks: let Qt pick a fallback font
            // (emoji, CJK, symbols), shrunk to fit its cell(s) when it's
            // bigger (color emoji usually are) and centered in them.
            QFont f = fontFor(flags);
            QFontMetricsF fm(f);
            qreal adv = fm.horizontalAdvance(text);
            const qreal room = qMax(cells, roomCells) * m_cw;
            const qreal scale = qMin<qreal>(1.0, qMin(adv > 0 ? room / adv : 1.0, m_ch / qMax<qreal>(1, fm.height())));
            qreal baseline = m_ascent;
            if (scale < 0.98) {
                f.setPixelSize(qMax(4, int(f.pixelSize() * scale)));
                fm = QFontMetricsF(f);
                adv = fm.horizontalAdvance(text);
                baseline = (m_ch - fm.height()) / 2 + fm.ascent();
            }
            p.setFont(f);
            // Centered in its own cell(s); spilling into the blank neighbor
            // only as far as it needs.
            const qreal own = cells * m_cw;
            const qreal off = adv <= own ? (own - adv) / 2 : 0;
            p.drawText(QPointF(x + off, baseline), text);
        }
    };

    // Backgrounds first, then text: a glyph wider than its cell (an emoji,
    // an italic overhang) must not be painted over by the next cell's color.
    for (const RunData& r : runs) {
        if (!(r.flags & FLAG_HAS_BG)) continue;
        QColor bgc = QColor::fromRgb(r.bg);
        if (!m_keepOpaque) bgc.setAlphaF(1 - m_transparency);
        p.setCompositionMode(QPainter::CompositionMode_Source);
        p.fillRect(r.col * m_cw, 0, r.width * m_cw, m_ch, bgc);
        p.setCompositionMode(QPainter::CompositionMode_SourceOver);
    }
    for (int ri = 0; ri < runs.size(); ++ri) {
        const RunData& r = runs[ri];
        const int x = r.col * m_cw;
        const int w = r.width * m_cw;
        if (r.flags & FLAG_INVISIBLE) continue;
        // A one-cell symbol followed by a blank cell may spread into it (as
        // Ghostty does), so emoji-style icons aren't shrunk to half size.
        int roomCells = r.width;
        if (r.width == 1 && r.text.size() <= 2) {
            const RunData* next = ri + 1 < runs.size() ? &runs[ri + 1] : nullptr;
            const bool blankNext = !next || next->col > r.col + 1 || next->text.startsWith(u' ');
            if (blankNext && r.col + 1 < m_cols) roomCells = 2;
        }
        QRgb fg = r.fg;
        if (r.flags & FLAG_FAINT) fg = blend(fg, (r.flags & FLAG_HAS_BG) ? r.bg : m_bg.rgb(), 1 - m_faintOpacity);
        const bool blank = r.text.trimmed().isEmpty();
        if (!blank) drawText(x, r.text, fg, r.flags, r.width, roomCells);
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
            if (!m_cursorText.trimmed().isEmpty()) drawText(int(cell.x()), m_cursorText, m_bg.rgb(), 0, m_cursorWide ? 2 : 1, 0);
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

    // Pictures: in the grid's coordinates (device pixels), clipped to it;
    // under the text (z < 0) or over it.
    {
        QSet<quint32> used;
        int n = 0;
        const qreal gridH = qreal(m_rows) * m_ch;
        for (const ImageSpot& im : std::as_const(m_images)) {
            if (im.w <= 0 || im.h <= 0) continue;
            auto cached = node->textures.value(im.id, { 0, nullptr });
            if (!cached.second || cached.first != im.gen) {
                const tessera::ImagePixels px = m_session ? m_session->image_pixels(im.id) : tessera::ImagePixels{};
                if (px.width == 0 || px.height == 0 || px.rgba.size() < size_t(px.width) * px.height * 4) continue;
                const QImage img = QImage(px.rgba.data(), int(px.width), int(px.height), QImage::Format_RGBA8888).copy();
                delete cached.second;
                cached = { im.gen, win->createTextureFromImage(img) };
                node->textures.insert(im.id, cached);
            }
            used.insert(im.id);
            // The visible part (an image partly scrolled off keeps its place).
            const qreal top = qMax<qreal>(0, im.y), bottom = qMin<qreal>(gridH, im.y + im.h);
            if (bottom <= top) continue;
            const qreal sy = im.sh * (top - im.y) / im.h, sh = im.sh * (bottom - top) / im.h;
            if (n >= node->images.size()) {
                QSGImageNode* in = win->createImageNode();
                in->setOwnsTexture(false);
                in->setFiltering(QSGTexture::Linear);
                node->images.append(in);
            }
            QSGImageNode* in = node->images[n++];
            if (in->parent()) node->removeChildNode(in);
            if (im.z < 0) node->insertChildNodeAfter(in, node->bg);
            else node->appendChildNode(in);
            in->setTexture(cached.second);
            in->setRect(QRectF((m_pad + im.x) / m_dpr, (m_pad + top) / m_dpr, im.w / m_dpr, (bottom - top) / m_dpr));
            in->setSourceRect(QRectF(im.sx, im.sy + sy, im.sw, sh));
        }
        for (int i = n; i < node->images.size(); ++i)
            if (node->images[i]->parent()) node->removeChildNode(node->images[i]);
        for (auto it = node->textures.begin(); it != node->textures.end();) {
            if (!used.contains(it.key())) { delete it.value().second; it = node->textures.erase(it); }
            else ++it;
        }
    }

    // Shell-integration marks: a short tick in the left margin at each prompt.
    int nMarks = 0;
    if (m_showMarks && m_pad >= 2) {
        for (int i = 0; i < m_rows && i < m_rowMark.size(); ++i) {
            if (!m_rowMark[i]) continue;
            if (nMarks >= node->marks.size()) {
                QSGRectangleNode* r = win->createRectangleNode();
                node->appendChildNode(r);
                node->marks.append(r);
            }
            QSGRectangleNode* r = node->marks[nMarks++];
            const qreal w = qMax<qreal>(2, qRound(m_pad * 0.45)) / m_dpr;
            const qreal h = m_ch * 0.62 / m_dpr;
            r->setRect(QRectF(qMax<qreal>(0.5, (m_pad / m_dpr - w) / 2), pad + i * m_ch / m_dpr + (m_ch / m_dpr - h) / 2, w, h));
            r->setColor(m_markColor);
        }
    }
    for (int i = nMarks; i < node->marks.size(); ++i) node->marks[i]->setRect(QRectF());

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
    if (applyKeyMapping(e)) return;
    // Shift+PgUp/PgDn scroll a page, Shift+Home/End jump to the top or
    // bottom of the scrollback (full-screen programs get these keys).
    if (e->modifiers() == Qt::ShiftModifier && m_session->scrolls_locally(0)) {
        const int page = qMax(1, m_rows - 1);
        bool handled = true;
        switch (e->key()) {
        case Qt::Key_PageUp: m_session->scroll_lines(-page); break;
        case Qt::Key_PageDown: m_session->scroll_lines(page); break;
        case Qt::Key_Home: m_session->scroll_to_row(0); break;
        case Qt::Key_End: m_session->scroll_to_bottom(); break;
        default: handled = false;
        }
        if (handled) {
            m_momentum->stop();
            polish();
            e->accept();
            return;
        }
    }
    const QByteArray text = e->text().toUtf8();
    if (m_session->key_event(e->key(), uint32_t(e->modifiers()), rs(text), false, e->nativeVirtualKey(), e->nativeScanCode())) {
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

bool TerminalItem::reportsAllKeys() const
{
    return m_session && m_session->reports_all_keys();
}

void TerminalItem::keyReleaseEvent(QKeyEvent* e)
{
    if (!m_session || m_exited) return QQuickItem::keyReleaseEvent(e);
    const QByteArray text = e->text().toUtf8();
    m_session->key_event(e->key(), uint32_t(e->modifiers()), rs(text), true, e->nativeVirtualKey(), e->nativeScanCode());
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
    m_session->key_event(key, uint32_t(mods), rs(t), release, 0, 0);
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
    // Any press takes the keys for the terminal (with the input editor on,
    // the pane hears so: a tap of Ctrl gives them back to the editor).
    if (m_editorBeside) emit terminalClicked();
    forceActiveFocus(Qt::MouseFocusReason);
    const bool appWantsMouse = m_session && m_session->wants_mouse(uint32_t(e->modifiers()));
    if (e->button() == Qt::LeftButton && (e->modifiers() & Qt::ControlModifier) && m_ctrlClickOpens && m_session) {
        const QString link = qs(m_session->link_at(float(e->position().x() * m_dpr), float(e->position().y() * m_dpr),
                                                   uint32_t(m_cw), uint32_t(m_ch), uint32_t(m_pad)));
        if (link.startsWith(QStringLiteral("ssh://"))) {
            // (A file on the pane's host: shown by Tessera, read there.)
            emit remoteLinkActivated(link);
            e->accept();
            return;
        }
        if (!link.isEmpty()) {
            QDesktopServices::openUrl(QUrl(link));
            e->accept();
            return;
        }
    }
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
        if (outside && !m_autoscroll->isActive()) {
            m_autoscrollStart = QDateTime::currentMSecsSinceEpoch();
            m_autoscroll->start();
        }
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

// The pointer: the pointing hand over a link with Ctrl held; an arrow while
// a full-screen or mouse-using program runs (Shift selects, so it gets the
// I-beam back); the I-beam over a shell.
void TerminalItem::updatePointer(QPointF pos)
{
    if (!m_session) return;
    Qt::CursorShape shape = Qt::IBeamCursor;
    const bool shift = m_hoverMods & Qt::ShiftModifier;
    if (!shift && (m_session->full_screen() || m_session->wants_mouse(uint32_t(m_hoverMods))))
        shape = Qt::ArrowCursor;
    if (m_ctrlClickOpens && (m_hoverMods & Qt::ControlModifier) && pos.x() >= 0
        && !m_session->link_at(float(pos.x() * m_dpr), float(pos.y() * m_dpr),
                               uint32_t(m_cw), uint32_t(m_ch), uint32_t(m_pad)).empty())
        shape = Qt::PointingHandCursor;
    if (cursor().shape() != shape) setCursor(shape);
}

void TerminalItem::hoverMoveEvent(QHoverEvent* e)
{
    m_hoverMods = e->modifiers();
    updatePointer(e->position());
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
    // Scrollback with momentum that builds up. Touchpads (which report
    // scroll phases) follow the fingers and glide on after a flick, and each
    // flick made while still gliding adds to the speed. A mouse wheel gives
    // each notch a push; notches in quick succession push harder and harder.
    // Programs that take the mouse (or full-screen ones) get wheel events or
    // arrow keys as before.
    if (m_session && m_session->scrolls_locally(uint32_t(e->modifiers()))) {
        const qint64 now = QDateTime::currentMSecsSinceEpoch();
        const qreal rowH = m_ch / m_dpr;
        const bool fingers = e->phase() != Qt::NoScrollPhase;
        const qreal dy = !e->pixelDelta().isNull() ? e->pixelDelta().y() : e->angleDelta().y() / 120.0 * 3 * rowH;
        if (fingers) {
            switch (e->phase()) {
            case Qt::ScrollBegin:
                m_carry = m_momentum->isActive() ? m_velocity : 0;
                m_momentum->stop();
                m_fingerVelocity = 0;
                m_lastWheelMs = now;
                break;
            case Qt::ScrollUpdate: {
                const qreal dt = qMax<qint64>(1, now - m_lastWheelMs);
                const qreal v = dy / dt;
                m_fingerVelocity = dt > 100 ? v : 0.6 * v + 0.4 * m_fingerVelocity;
                m_lastWheelMs = now;
                scrollPixels(dy);
                break;
            }
            case Qt::ScrollEnd: {
                qreal v = (now - m_lastWheelMs < 120) ? m_fingerVelocity : 0;
                if (v != 0 && m_carry != 0 && (v > 0) == (m_carry > 0)) v += m_carry * 0.9;   // keeps building
                m_carry = 0;
                if (std::abs(v) > 0.05) {
                    m_velocity = qBound<qreal>(-40, v, 40);
                    m_decay = 0.955;
                    m_momentum->start();
                }
                break;
            }
            case Qt::ScrollMomentum:
                scrollPixels(dy);   // the platform's own momentum (macOS)
                break;
            default:
                break;
            }
        } else if (dy != 0) {
            const bool sameWay = m_momentum->isActive() && (m_velocity > 0) == (dy > 0);
            m_streak = (sameWay && now - m_lastNotchMs < 300) ? qMin(m_streak + 1, 14) : 0;
            m_lastNotchMs = now;
            // About three rows for a lone notch; faster with every quick one.
            const qreal push = dy / 200.0 * (1 + 0.35 * m_streak);
            m_velocity = sameWay ? qBound<qreal>(-60, m_velocity + push, 60) : push;
            m_decay = 0.92;
            if (!m_momentum->isActive()) m_momentum->start();
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
        if (m_copyOnSelect) {
            QGuiApplication::clipboard()->setText(text, QClipboard::Clipboard);
            emit copied(text);
        }
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
    if (m_focusFollowsMouse && !m_editorBeside && !hasActiveFocus()) forceActiveFocus(Qt::MouseFocusReason);
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
    if (!text.isEmpty()) {
        QGuiApplication::clipboard()->setText(text);
        emit copied(text);
    }
}

void TerminalItem::paste() { pasteText(QGuiApplication::clipboard()->text()); }

QString TerminalItem::cells() const
{
    return m_session ? qs(m_session->cells()) : QStringLiteral("[]");
}

QString TerminalItem::cellOutput(quint64 id, const QString& format) const
{
    if (!m_session) return QString();
    const QByteArray f = format.toUtf8();
    return qs(m_session->cell_output(id, rust::Str(f.constData(), size_t(f.size()))));
}

QString TerminalItem::lastOutput() const
{
    return m_session ? qs(m_session->last_output()) : QString();
}

void TerminalItem::scrollLines(int n)
{
    if (!m_session) return;
    m_session->scroll_lines(n);
    m_momentum->stop();
    polish();
}

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
    delete m_wakeNotifier;
    m_wakeNotifier = nullptr;
    if (m_session) {
        m_session->kill();
        rust::Box<tessera::Session>::from_raw(m_session);
        m_session = nullptr;
    }
    m_attachId.clear();
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

QString TerminalItem::startRecording(const QString& path)
{
    if (!m_session) return {};
    QString p = path;
    if (p.startsWith(u"file://")) p = QUrl(p).toLocalFile();
    const QByteArray b = p.toUtf8();
    const QString out = qs(m_session->record_to_file(rs(b)));
    emit recordingChanged();
    return out;
}

QString TerminalItem::stopRecording()
{
    if (!m_session) return {};
    const QString out = qs(m_session->record_to_file(rs(QByteArray())));
    emit recordingChanged();
    return out;
}

bool TerminalItem::copyImageFile(const QString& path)
{
    QString p = path;
    if (p.startsWith(u"file://")) p = QUrl(p).toLocalFile();
    QImage img(p);
    if (img.isNull()) return false;
    QGuiApplication::clipboard()->setImage(img);
    return true;
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

void TerminalItem::setKeyMappingsJson(const QString& v)
{
    if (v == m_keyMappingsJson) return;
    m_keyMappingsJson = v;
    m_keyMappings.clear();
    const QJsonArray arr = QJsonDocument::fromJson(v.toUtf8()).array();
    for (const QJsonValue& x : arr) {
        const QJsonObject o = x.toObject();
        const QKeySequence seq(o.value(u"key").toString());
        if (seq.isEmpty()) continue;
        m_keyMappings.append({seq, o.value(u"action").toString(), o.value(u"value").toString()});
    }
    emit optionsChanged();
}

static QByteArray unescapeText(const QString& s)
{
    QString out;
    for (int i = 0; i < s.size(); ++i) {
        if (s[i] == u'\\' && i + 1 < s.size()) {
            const QChar c = s[++i];
            if (c == u'n') out += u'\n';
            else if (c == u'r') out += u'\r';
            else if (c == u't') out += u'\t';
            else if (c == u'e') out += QChar(0x1b);
            else out += c;
        } else {
            out += s[i];
        }
    }
    return out.toUtf8();
}

bool TerminalItem::applyKeyMapping(QKeyEvent* e)
{
    if (m_keyMappings.isEmpty()) return false;
    const QKeySequence pressed(QKeyCombination(e->modifiers() & ~Qt::KeypadModifier, Qt::Key(e->key())));
    for (const KeyMapping& m : std::as_const(m_keyMappings)) {
        if (m.seq.matches(pressed) != QKeySequence::ExactMatch) continue;
        QByteArray bytes;
        if (m.action == u"send_text") bytes = unescapeText(m.value);
        else if (m.action == u"send_escape") bytes = QByteArray("\x1b") + unescapeText(m.value);
        else if (m.action == u"send_hex") {
            for (const QString& h : m.value.split(u' ', Qt::SkipEmptyParts)) bytes.append(char(h.toUInt(nullptr, 16)));
        } else if (m.action == u"action") {
            emit keyAction(m.value);
        }
        if (!bytes.isEmpty()) m_session->send_text(rs(bytes));
        e->accept();
        return true;   // "ignore" swallows the key
    }
    return false;
}

QString TerminalItem::linkAt(qreal x, qreal y) const
{
    if (!m_session) return QString();
    return qs(m_session->link_at(float(x * m_dpr), float(y * m_dpr), uint32_t(m_cw), uint32_t(m_ch), uint32_t(m_pad)));
}

bool TerminalItem::saveSnapshot(const QString& path)
{
    if (!m_session) return false;
    const QByteArray p = path.toUtf8();
    return m_session->save_snapshot(rs(p));
}

bool TerminalItem::jumpToPrompt(int dir)
{
    if (!m_session) return false;
    const bool ok = m_session->jump_to_prompt(dir);
    polish();
    return ok;
}

QString TerminalItem::selectLastOutput()
{
    if (!m_session) return QString();
    const QString t = qs(m_session->select_last_output());
    publishSelection();
    polish();
    return t;
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
