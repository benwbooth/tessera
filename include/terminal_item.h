// The terminal view: a QQuickItem that shows one tessera::Session.
//
// Each row is painted with QPainter into an image (native FreeType text,
// glyphs placed exactly on cell boundaries) only when it changes, then shown
// as a texture in the scene graph, so scrolling and compositing run on the
// GPU while the text looks exactly like the rest of the desktop.
#pragma once

#include <QtCore/QPointer>
#include <QtCore/QSocketNotifier>
#include <QtCore/QVariantMap>
#include <QtCore/QVector>
#include <QtGui/QColor>
#include <QtGui/QFont>
#include <QtGui/QImage>
#include <QtGui/QRawFont>
#include <QtQuick/QQuickItem>

namespace tessera {
struct Session;
struct Frame;
}

class TerminalItem : public QQuickItem {
    Q_OBJECT
    Q_PROPERTY(QString attachId READ attachId WRITE setAttachId NOTIFY attachIdChanged)
    Q_PROPERTY(QString tmuxPane READ tmuxPane WRITE setTmuxPane NOTIFY tmuxPaneChanged)
    Q_PROPERTY(QString cwd READ cwd WRITE setCwd NOTIFY cwdChanged)
    Q_PROPERTY(QString fontFamily READ fontFamily WRITE setFontFamily NOTIFY fontChanged)
    Q_PROPERTY(qreal fontPointSize READ fontPointSize WRITE setFontPointSize NOTIFY fontChanged)
    Q_PROPERTY(QString sessionId READ sessionId NOTIFY sessionIdChanged)
    Q_PROPERTY(QString title READ title NOTIFY titleChanged)
    Q_PROPERTY(QString pwd READ pwd NOTIFY pwdChanged)
    Q_PROPERTY(int tmuxPaneId READ tmuxPaneId NOTIFY sessionIdChanged)
    Q_PROPERTY(QString tmuxGateway READ tmuxGateway NOTIFY tmuxGatewayChanged)
    Q_PROPERTY(bool exited READ exited NOTIFY exitedChanged)
    Q_PROPERTY(int columns READ columns NOTIFY gridChanged)
    Q_PROPERTY(int rows READ rows NOTIFY gridChanged)
    Q_PROPERTY(qreal cellWidth READ cellWidth NOTIFY metricsChanged)
    Q_PROPERTY(qreal cellHeight READ cellHeight NOTIFY metricsChanged)
    Q_PROPERTY(qreal padding READ padding NOTIFY metricsChanged)
    Q_PROPERTY(QColor background READ background NOTIFY backgroundChanged)
    Q_PROPERTY(bool hasSelection READ hasSelection NOTIFY selectionChanged)

public:
    explicit TerminalItem(QQuickItem* parent = nullptr);
    ~TerminalItem() override;

    QString attachId() const { return m_attachId; }
    void setAttachId(const QString& v);
    QString tmuxPane() const { return m_tmuxPane; }
    void setTmuxPane(const QString& v);
    QString cwd() const { return m_cwd; }
    void setCwd(const QString& v);
    QString fontFamily() const { return m_fontFamily; }
    void setFontFamily(const QString& v);
    qreal fontPointSize() const { return m_fontPointSize; }
    void setFontPointSize(qreal v);
    QString sessionId() const { return m_sessionId; }
    QString title() const { return m_title; }
    QString pwd() const { return m_pwd; }
    int tmuxPaneId() const;
    QString tmuxGateway() const { return m_tmuxGateway; }
    bool exited() const { return m_exited; }
    int columns() const { return m_cols; }
    int rows() const { return m_rows; }
    qreal cellWidth() const { return m_cw / m_dpr; }
    qreal cellHeight() const { return m_ch / m_dpr; }
    qreal padding() const { return m_pad / m_dpr; }
    QColor background() const { return m_bg; }
    bool hasSelection() const { return m_hasSelection; }

    Q_INVOKABLE void copy();
    Q_INVOKABLE void paste();
    Q_INVOKABLE void sendText(const QString& text);
    Q_INVOKABLE void killSession();
    Q_INVOKABLE void tmuxCommand(const QString& command);
    Q_INVOKABLE void scrollToBottom();

signals:
    void attachIdChanged();
    void tmuxPaneChanged();
    void cwdChanged();
    void fontChanged();
    void sessionIdChanged();
    void titleChanged();
    void pwdChanged();
    void tmuxGatewayChanged();
    void exitedChanged();
    void gridChanged();
    void metricsChanged();
    void backgroundChanged();
    void selectionChanged();
    void bell();
    void tmuxEvent(const QVariantMap& event);
    void openFailed(const QString& message);

protected:
    void componentComplete() override;
    QSGNode* updatePaintNode(QSGNode* old, UpdatePaintNodeData*) override;
    void updatePolish() override;
    void geometryChange(const QRectF& newGeometry, const QRectF& oldGeometry) override;
    void itemChange(ItemChange change, const ItemChangeData& value) override;
    void keyPressEvent(QKeyEvent* e) override;
    void keyReleaseEvent(QKeyEvent* e) override;
    void inputMethodEvent(QInputMethodEvent* e) override;
    QVariant inputMethodQuery(Qt::InputMethodQuery query) const override;
    void mousePressEvent(QMouseEvent* e) override;
    void mouseMoveEvent(QMouseEvent* e) override;
    void mouseReleaseEvent(QMouseEvent* e) override;
    void hoverMoveEvent(QHoverEvent* e) override;
    void wheelEvent(QWheelEvent* e) override;
    void focusInEvent(QFocusEvent* e) override;
    void focusOutEvent(QFocusEvent* e) override;

private:
    struct RunData {
        int col;
        int width;
        QString text;
        QRgb fg;
        QRgb bg;
        quint32 flags;
    };

    void open();
    void onReadable();
    void handleEvents();
    void pullFrame(bool full);
    void paintRow(int row, const QVector<RunData>& runs);
    void updateMetrics();
    void resizeGrid();
    void mouse(int action, int button, QPointF pos, Qt::KeyboardModifiers mods);
    void publishSelection();
    const QRawFont& rawFont(quint32 flags);
    QFont fontFor(quint32 flags) const;

    tessera::Session* m_session = nullptr;
    tessera::Frame* m_frame = nullptr;
    QSocketNotifier* m_notifier = nullptr;

    QString m_attachId, m_tmuxPane, m_cwd, m_sessionId, m_title, m_pwd, m_tmuxGateway;
    QString m_fontFamily;
    qreal m_fontPointSize = 0;
    bool m_exited = false;
    bool m_complete = false;
    bool m_forceFull = true;
    bool m_hasSelection = false;
    bool m_selecting = false;

    // Metrics in device pixels.
    qreal m_dpr = 1;
    int m_cw = 8, m_ch = 16, m_ascent = 12, m_pad = 4;
    QFont m_font;
    QRawFont m_raw[4];
    bool m_rawValid[4] = {};

    int m_cols = 0, m_rows = 0;
    QColor m_bg = QColor(0x1d, 0x1f, 0x21);
    QColor m_fg = QColor(0xc5, 0xc8, 0xc6);

    // Cursor (from the last frame).
    bool m_cursorVisible = false;
    int m_cursorX = -1, m_cursorY = -1, m_cursorStyle = 0;
    QRgb m_cursorColor = 0;
    QString m_cursorText;
    bool m_cursorWide = false;

    QVector<QImage> m_rowImages;
    QVector<bool> m_rowUpload;
    bool m_rebuildNodes = true;
    int m_wheelAccum = 0;
};
