#include "terminal_item.h"

#include "tessera/src/bridge.cxxqt.h"

#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtCore/QJsonObject>
#include <QtGui/QClipboard>
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

void TerminalItem::setFontFamily(const QString& v)
{
    if (v.isEmpty() || v == m_fontFamily) return;
    m_fontFamily = v;
    updateMetrics();
    emit fontChanged();
}

void TerminalItem::setFontPointSize(qreal v)
{
    if (v <= 0 || qFuzzyCompare(v, m_fontPointSize)) return;
    m_fontPointSize = v;
    updateMetrics();
    emit fontChanged();
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
    if (flags & FLAG_BOLD) f.setWeight(QFont::Bold);
    if (flags & FLAG_ITALIC) f.setItalic(true);
    return f;
}

const QRawFont& TerminalItem::rawFont(quint32 flags)
{
    const int i = ((flags & FLAG_BOLD) ? 1 : 0) | ((flags & FLAG_ITALIC) ? 2 : 0);
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
    m_cw = qMax(1, qRound(fm.horizontalAdvance(QLatin1Char('M'))));
    const int glyphH = qCeil(fm.ascent() + fm.descent());
    m_ch = qMax(1, glyphH + qRound(m_dpr)); // a little leading
    m_ascent = qRound(fm.ascent()) + (m_ch - glyphH) / 2;
    m_pad = qRound(5 * m_dpr);
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
        auto box = tessera::session_open(rs(a), rs(t), rs(c), uint16_t(cols), uint16_t(rows), uint32_t(m_cw),
                                         uint32_t(m_ch));
        m_session = box.into_raw();
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
    if (sig & SIG_DIRTY) polish();
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
            emit bell();
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
    for (uint16_t row : f.dirty_rows) {
        if (row < m_rows) {
            paintRow(row, byRow.value(row));
            m_rowUpload[row] = true;
        }
    }
    update();
}

void TerminalItem::paintRow(int row, const QVector<RunData>& runs)
{
    const QSize size(qMax(1, m_cols * m_cw), m_ch);
    QImage& img = m_rowImages[row];
    if (img.size() != size) img = QImage(size, QImage::Format_RGB32);
    img.fill(m_bg);
    QPainter p(&img);
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
        if (r.flags & FLAG_HAS_BG) p.fillRect(x, 0, w, m_ch, QColor::fromRgb(r.bg));
        if (r.flags & FLAG_INVISIBLE) continue;
        QRgb fg = r.fg;
        if (r.flags & FLAG_FAINT) fg = blend(fg, (r.flags & FLAG_HAS_BG) ? r.bg : m_bg.rgb(), 0.4);
        const bool blank = r.text.trimmed().isEmpty();
        if (!blank) drawText(x, r.text, fg, r.flags, r.width);
        if (r.flags & FLAG_UNDERLINE)
            p.fillRect(QRectF(x, qMin<qreal>(m_ch - lineW, m_ascent + lineW * 1.5), w, lineW), QColor::fromRgb(fg));
        if (r.flags & FLAG_STRIKE) p.fillRect(QRectF(x, m_ascent * 0.65, w, lineW), QColor::fromRgb(fg));
    }

    if (m_cursorVisible && row == m_cursorY && m_cursorX >= 0) {
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
    node->bg->setColor(m_bg);

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
    return node;
}

// ---- input -------------------------------------------------------------

void TerminalItem::keyPressEvent(QKeyEvent* e)
{
    if (!m_session || m_exited) return QQuickItem::keyPressEvent(e);
    const QByteArray text = e->text().toUtf8();
    if (m_session->key_event(e->key(), uint32_t(e->modifiers()), rs(text), false, e->nativeVirtualKey())) {
        e->accept();
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
    e->accept();
}

void TerminalItem::inputMethodEvent(QInputMethodEvent* e)
{
    if (m_session && !e->commitString().isEmpty()) {
        const QByteArray t = e->commitString().toUtf8();
        m_session->send_text(rs(t));
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
    m_selecting = e->button() == Qt::LeftButton;
    mouse(0, buttonCode(e->button()), e->position(), e->modifiers());
    e->accept();
}

void TerminalItem::mouseMoveEvent(QMouseEvent* e)
{
    mouse(2, 0, e->position(), e->modifiers());
    e->accept();
}

void TerminalItem::mouseReleaseEvent(QMouseEvent* e)
{
    mouse(1, buttonCode(e->button()), e->position(), e->modifiers());
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

void TerminalItem::wheelEvent(QWheelEvent* e)
{
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
        // Copy on select, as iTerm2 does by default; also the X/Wayland primary.
        QGuiApplication::clipboard()->setText(text, QClipboard::Clipboard);
        if (QGuiApplication::clipboard()->supportsSelection())
            QGuiApplication::clipboard()->setText(text, QClipboard::Selection);
    }
    if (has != m_hasSelection) {
        m_hasSelection = has;
        emit selectionChanged();
    }
}

void TerminalItem::focusInEvent(QFocusEvent* e)
{
    QQuickItem::focusInEvent(e);
    if (m_session) m_session->focus(true);
    m_forceFull = true;
    polish();
}

void TerminalItem::focusOutEvent(QFocusEvent* e)
{
    QQuickItem::focusOutEvent(e);
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

void TerminalItem::paste()
{
    if (!m_session) return;
    const QByteArray t = QGuiApplication::clipboard()->text().toUtf8();
    if (!t.isEmpty()) m_session->paste(rs(t));
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

void TerminalItem::scrollToBottom()
{
    if (!m_session) return;
    m_session->scroll_to_bottom();
    polish();
}
