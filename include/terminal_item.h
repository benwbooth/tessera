// The terminal view: a QQuickItem that shows one tessera::Session.
//
// Each row is painted with QPainter into an image (native FreeType text,
// glyphs placed exactly on cell boundaries) only when it changes, then shown
// as a texture in the scene graph, so scrolling and compositing run on the
// GPU while the text looks exactly like the rest of the desktop.
#pragma once

#include <QtCore/QPointer>
#include <QtCore/QSocketNotifier>
#include <QtCore/QTimer>
#include <QtGui/QKeySequence>
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
    // The shell waits at its prompt (shell integration), and whether a
    // full-screen program (alternate screen) is up: the input editor shows
    // only at a prompt with no full-screen program.
    Q_PROPERTY(bool atPrompt READ atPrompt NOTIFY promptChanged)
    Q_PROPERTY(bool fullScreen READ fullScreen NOTIFY promptChanged)
    // The last command's exit status and how long it ran (ms), from the marks.
    Q_PROPERTY(int lastStatus READ lastStatus NOTIFY commandFinished)
    Q_PROPERTY(qint64 lastDuration READ lastDuration NOTIFY commandFinished)
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
    Q_PROPERTY(QString themeJson READ themeJson WRITE setThemeJson NOTIFY themeJsonChanged)
    Q_PROPERTY(int cursorShape READ cursorShape WRITE setCursorShape NOTIFY cursorShapeChanged)
    Q_PROPERTY(int scrollback READ scrollback WRITE setScrollback NOTIFY scrollbackChanged)
    Q_PROPERTY(qreal lineHeight READ lineHeight WRITE setLineHeight NOTIFY fontChanged)
    Q_PROPERTY(qreal paddingSize READ paddingSize WRITE setPaddingSize NOTIFY metricsChanged)
    Q_PROPERTY(bool copyOnSelect READ copyOnSelect WRITE setCopyOnSelect NOTIFY copyOnSelectChanged)
    // Profile settings applied by the view.
    Q_PROPERTY(QString command MEMBER m_command NOTIFY optionsChanged)
    Q_PROPERTY(QString termName MEMBER m_termName NOTIFY optionsChanged)
    Q_PROPERTY(QString startupInput MEMBER m_startupInput NOTIFY optionsChanged)
    Q_PROPERTY(QString optionsJson READ optionsJson WRITE setOptionsJson NOTIFY optionsChanged)
    Q_PROPERTY(qreal faintOpacity READ faintOpacity WRITE setFaintOpacity NOTIFY optionsChanged)
    Q_PROPERTY(qreal charSpacing READ charSpacing WRITE setCharSpacing NOTIFY optionsChanged)
    Q_PROPERTY(bool cursorBlink READ cursorBlink WRITE setCursorBlink NOTIFY optionsChanged)
    Q_PROPERTY(bool cursorGuide READ cursorGuide WRITE setCursorGuide NOTIFY optionsChanged)
    Q_PROPERTY(bool boldFont READ boldFont WRITE setBoldFont NOTIFY optionsChanged)
    Q_PROPERTY(bool italicText READ italicText WRITE setItalicText NOTIFY optionsChanged)
    Q_PROPERTY(qreal transparency READ transparency WRITE setTransparency NOTIFY optionsChanged)
    Q_PROPERTY(bool keepBackgroundsOpaque READ keepBackgroundsOpaque WRITE setKeepBackgroundsOpaque NOTIFY optionsChanged)
    Q_PROPERTY(bool visualBell MEMBER m_visualBell NOTIFY optionsChanged)
    Q_PROPERTY(bool silenceBell MEMBER m_silenceBell NOTIFY optionsChanged)
    Q_PROPERTY(bool middleClickPastes MEMBER m_middleClickPastes NOTIFY optionsChanged)
    Q_PROPERTY(bool rightClickPastes MEMBER m_rightClickPastes NOTIFY optionsChanged)
    Q_PROPERTY(bool focusFollowsMouse MEMBER m_focusFollowsMouse NOTIFY optionsChanged)
    // Other views that receive this one's typing (Broadcast Input).
    Q_PROPERTY(QVariantList broadcastTo MEMBER m_broadcastTo NOTIFY optionsChanged)
    Q_PROPERTY(bool logging READ logging NOTIFY loggingChanged)
    // Shell integration mark indicators in the left margin.
    Q_PROPERTY(bool showMarks MEMBER m_showMarks NOTIFY optionsChanged)
    Q_PROPERTY(QColor markColor MEMBER m_markColor NOTIFY optionsChanged)
    // Ctrl+click opens URLs and files.
    Q_PROPERTY(bool ctrlClickOpens MEMBER m_ctrlClickOpens NOTIFY optionsChanged)
    // Key mappings: [{key: "Ctrl+Alt+K", action: "send_text"|"send_hex"|"send_escape"|"ignore"|"action", value}]
    // A saved screen shown above a new shell (its old session is gone).
    Q_PROPERTY(QString preloadFile MEMBER m_preloadFile NOTIFY optionsChanged)
    Q_PROPERTY(QString keyMappingsJson READ keyMappingsJson WRITE setKeyMappingsJson NOTIFY optionsChanged)
    // Scrollback position, for the scrollbar (rows).
    Q_PROPERTY(qreal scrollTotal READ scrollTotal NOTIFY scrollChanged)
    Q_PROPERTY(qreal scrollOffset READ scrollOffset NOTIFY scrollChanged)
    Q_PROPERTY(qreal scrollLength READ scrollLength NOTIFY scrollChanged)

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
    QString themeJson() const { return m_themeJson; }
    void setThemeJson(const QString& v);
    int cursorShape() const { return m_cursorShape; }
    void setCursorShape(int v);
    int scrollback() const { return m_scrollback; }
    void setScrollback(int v);
    qreal lineHeight() const { return m_lineHeight; }
    void setLineHeight(qreal v);
    qreal paddingSize() const { return m_paddingSize; }
    void setPaddingSize(qreal v);
    bool copyOnSelect() const { return m_copyOnSelect; }
    void setCopyOnSelect(bool v);

    Q_INVOKABLE void copy();
    Q_INVOKABLE void paste();
    Q_INVOKABLE void sendText(const QString& text);
    Q_INVOKABLE void killSession();
    Q_INVOKABLE void tmuxCommand(const QString& command);
    Q_INVOKABLE void scrollToBottom();
    Q_INVOKABLE void scrollLines(int n);
    Q_INVOKABLE void selectAll();
    Q_INVOKABLE void clearScrollback();
    Q_INVOKABLE void copyWithStyles();
    Q_INVOKABLE void copyWithControlSequences();
    Q_INVOKABLE void pasteSelection();
    Q_INVOKABLE void pasteSlowly();
    Q_INVOKABLE void pasteText(const QString& text);
    Q_INVOKABLE QString selectedText() const;
    Q_INVOKABLE QString contentsText() const;
    Q_INVOKABLE void resetTerminal();
    Q_INVOKABLE void restart();
    Q_INVOKABLE void showMessage(const QString& text);
    Q_INVOKABLE bool startLogging(const QString& path);
    Q_INVOKABLE void stopLogging();
    Q_INVOKABLE void findCursor();
    // Broadcast Input: replay another view's typing here.
    Q_INVOKABLE void receiveKey(int key, int mods, const QString& text, bool release);
    Q_INVOKABLE void receiveText(const QString& text);
    // Find in scrollback.
    Q_INVOKABLE int find(const QString& query, bool regex);
    Q_INVOKABLE int findStep(int dir);
    Q_INVOKABLE int findRefresh();
    Q_INVOKABLE int findIndex() const;
    Q_INVOKABLE void endFind();
    Q_INVOKABLE void scrollToRow(qreal row);
    Q_INVOKABLE bool jumpToPrompt(int dir);
    Q_INVOKABLE bool saveSnapshot(const QString& path);
    // What Ctrl+click at this point (item coordinates) would open, or "".
    Q_INVOKABLE QString linkAt(qreal x, qreal y) const;
    Q_INVOKABLE QString selectLastOutput();

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
    void themeJsonChanged();
    void cursorShapeChanged();
    void scrollbackChanged();
    void copyOnSelectChanged();
    void contextMenuRequested(qreal x, qreal y);
    void optionsChanged();
    void loggingChanged();
    void activity();
    // The session was attached by another window (a tab dragged there).
    void takenOver();
    void scrollChanged();
    // A key mapping asked for an app action (new_tab, split_right, ...).
    void keyAction(const QString& name);
    // Text went to the clipboard (copy or copy-on-select), for Paste History.
    void copied(const QString& text);

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
    void hoverEnterEvent(QHoverEvent* e) override;

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
    bool m_atPrompt = false, m_fullScreen = false;
    int m_lastStatus = 0;
    qint64 m_lastDuration = 0;
public:
    int lastStatus() const { return m_lastStatus; }
    qint64 lastDuration() const { return m_lastDuration; }
    Q_SIGNAL void commandFinished();
    bool atPrompt() const { return m_atPrompt; }
    bool fullScreen() const { return m_fullScreen; }
    Q_SIGNAL void promptChanged();
private:
    Qt::KeyboardModifiers m_hoverMods;
    void updatePointer(QPointF pos);
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

    QString optionsJson() const { return m_optionsJson; }
    void setOptionsJson(const QString& v);
    qreal faintOpacity() const { return m_faintOpacity; }
    void setFaintOpacity(qreal v) { m_faintOpacity = v; repaintAll(); emit optionsChanged(); }
    qreal charSpacing() const { return m_charSpacing; }
    void setCharSpacing(qreal v);
    bool cursorBlink() const { return m_cursorBlink; }
    void setCursorBlink(bool v);
    bool cursorGuide() const { return m_cursorGuide; }
    void setCursorGuide(bool v) { m_cursorGuide = v; repaintAll(); emit optionsChanged(); }
    bool boldFont() const { return m_boldFont; }
    void setBoldFont(bool v) { m_boldFont = v; repaintAll(); emit optionsChanged(); }
    bool italicText() const { return m_italicText; }
    void setItalicText(bool v) { m_italicText = v; repaintAll(); emit optionsChanged(); }
    qreal transparency() const { return m_transparency; }
    void setTransparency(qreal v) { m_transparency = qBound<qreal>(0, v, 0.9); repaintAll(); emit optionsChanged(); }
    bool keepBackgroundsOpaque() const { return m_keepOpaque; }
    void setKeepBackgroundsOpaque(bool v) { m_keepOpaque = v; repaintAll(); emit optionsChanged(); }
    bool logging() const;

    void repaintAll();
    void repaintRow(int row);
    void forwardKey(QKeyEvent* e, bool release);

    QVector<QVector<RunData>> m_rowRuns;
    QString m_command, m_termName, m_startupInput, m_optionsJson;
    qreal m_faintOpacity = 0.6;
    qreal m_charSpacing = 1.0;
    bool m_cursorBlink = false, m_blinkOn = true, m_cursorGuide = false;
    bool m_boldFont = true, m_italicText = true;
    qreal m_transparency = 0;
    bool m_keepOpaque = true;
    bool m_visualBell = true, m_silenceBell = false;
    bool m_middleClickPastes = true, m_rightClickPastes = false, m_focusFollowsMouse = false;
    QVariantList m_broadcastTo;
    QTimer* m_blinkTimer = nullptr;
    QTimer* m_pasteTimer = nullptr;
    QByteArray m_slowPaste;
    qint64 m_flashUntil = 0;
    qint64 m_findCursorUntil = 0;

    qreal scrollTotal() const { return m_scrollTotal; }
    qreal scrollOffset() const { return m_scrollOffset; }
    qreal scrollLength() const { return m_scrollLength; }
    void scrollPixels(qreal dy);
    qreal m_scrollTotal = 0, m_scrollOffset = 0, m_scrollLength = 0;
    // Touchpad scrolling: leftover fraction of a row, and momentum.
    qreal m_scrollRemainder = 0;
    qreal m_velocity = 0;          // logical px per ms
    qint64 m_lastWheelMs = 0;
    QTimer* m_momentum = nullptr;
    QTimer* m_autoscroll = nullptr;   // selection dragged past an edge
    // Opening and resizing wait for the layout to settle: every size the
    // shell is told makes it redraw its prompt (zsh leaves the old first
    // line of a two-line prompt behind each time).
    QTimer* m_openTimer = nullptr;
    QTimer* m_resizeTimer = nullptr;
    QTimer* m_smooth = nullptr;       // (unused)
    qreal m_smoothPending = 0;
    qreal m_decay = 0.95;             // momentum kept per 16 ms frame
    qreal m_fingerVelocity = 0;       // touchpad: speed while the fingers move
    qreal m_carry = 0;                // momentum still going when a new flick began
    int m_streak = 0;                 // wheel notches in quick succession
    qint64 m_lastNotchMs = 0;
    qint64 m_autoscrollStart = 0;
    QPointF m_lastPointer;

    QString keyMappingsJson() const { return m_keyMappingsJson; }
    void setKeyMappingsJson(const QString& v);
    bool applyKeyMapping(QKeyEvent* e);
    struct KeyMapping { QKeySequence seq; QString action; QString value; };
    QVector<KeyMapping> m_keyMappings;
    QString m_keyMappingsJson;
    QString m_preloadFile;
    QVector<bool> m_rowMark;
    bool m_showMarks = true;
    QColor m_markColor = QColor(0x62, 0xa8, 0xf0);
    bool m_ctrlClickOpens = true;

    QString m_themeJson;
    int m_cursorShape = 0;
    int m_scrollback = 10000;
    qreal m_lineHeight = 1.1;
    qreal m_paddingSize = 6;
    bool m_copyOnSelect = true;
};
