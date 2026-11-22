#include <QElapsedTimer>
#include <QPointer>
#include <QTimer>
// Process-wide Qt setup: the application object and QML registration.
#pragma once

#include <QtCore/QDateTime>
#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtGui/QFontDatabase>
#include <QtGui/QIcon>
#include <QtGui/QWheelEvent>
#include <QtGui/QKeyEvent>
#include <QtGui/QPointingDevice>
#include <QtQml/qqml.h>
#include <QtQuick/QQuickItem>
#include <QtQuick/QQuickWindow>
#include <QtWidgets/QApplication>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>

#include "terminal_item.h"
#include "syntax_highlighter.h"
#include "file_clip.h"
#include "global_menu.h"

// Wayland desktops place windows themselves; with the session-restore
// extension (xx-session-management) they put each one back where it was.
#if __has_include(<QtGui/qpa/qplatformwindow_p.h>)
#include <QtGui/qpa/qplatformwindow_p.h>
#if QT_CONFIG(wayland)
#define TESSERA_WAYLAND_RESTORE 1
#endif
#endif

inline void tessera_webengine_initialize() { QtWebEngineQuick::initialize(); }

// Ctrl tapped on its own (pressed and let go, nothing else in between):
// between the pane's input editor and its terminal, from wherever the keys
// are. A terminal program never sees a lone Ctrl, so it takes nothing from
// them. Seen at the window, before anything below it; the window's
// toggleEditor() decides.
class TesseraCtrlTap : public QObject {
public:
    using QObject::QObject;
    bool eventFilter(QObject* o, QEvent* e) override
    {
        switch (e->type()) {
        case QEvent::KeyPress: {
            auto* k = static_cast<QKeyEvent*>(e);
            // (Any other key, wherever it's delivered (the window, or an item,
            // a popup, straight), spoils the tap: Ctrl+Enter, Ctrl+V.)
            if (k->key() != Qt::Key_Control) { armed = false; pending.clear(); return false; }
            if (!qobject_cast<QQuickWindow*>(o) || k->isAutoRepeat()) return false;
            armed = !(k->modifiers() & ~Qt::ControlModifier & ~Qt::KeypadModifier);
            // (Which one: right Ctrl's tap runs the editor's command in the
            // background; left's switches between editor and terminal. 105:
            // right Ctrl's evdev code + 8.)
            right = k->nativeScanCode() == 105;
            if (armed) since.start();
            return false;
        }
        case QEvent::KeyRelease: {
            auto* k = static_cast<QKeyEvent*>(e);
            if (k->key() != Qt::Key_Control) { armed = false; return false; }
            auto* w = qobject_cast<QQuickWindow*>(o);
            if (!w || k->isAutoRepeat() || !armed) return false;
            armed = false;
            // (Held a while: not a tap, someone changed their mind.)
            if (since.elapsed() > 500) return false;
            // (Not at once: a key remapper's dual key (keyd's overload(control,
            // enter): Enter held is Ctrl) sends a Ctrl tap and then the key
            // itself when it's tapped. A key right after cancels it.)
            pending = w;
            tappedRight = right;
            QTimer::singleShot(120, this, [this] {
                if (pending) QMetaObject::invokeMethod(pending.data(), "ctrlTapped", Q_ARG(QVariant, QVariant(tappedRight)));
                pending.clear();
            });
            return false;
        }
        // (Text from an input method, or a shortcut, between: not a tap.)
        case QEvent::InputMethod:
        case QEvent::Shortcut:
        case QEvent::MouseButtonPress:
        case QEvent::Wheel:
        case QEvent::FocusOut:
            armed = false;
            pending.clear();
            return false;
        default:
            return false;
        }
    }

private:
    bool armed = false;
    bool right = false, tappedRight = false;
    QElapsedTimer since;
    QPointer<QQuickWindow> pending;   // (a tap waiting to see no key follows)
};

// A window hidden by the desktop (covered, on another virtual desktop, in
// the overview) stops being drawn; shown again without the focus, nothing
// told the browser views (a Markdown file's card, HTML output) and they
// stayed black until clicked. This keeps each window's graphics while it's
// hidden and tells it when it's back (its QML `reexposed()` signal), so they
// draw again. (As Almanac's message pane.)
class TesseraExposeWatch : public QObject {
public:
    using QObject::QObject;
    bool eventFilter(QObject* o, QEvent* e) override
    {
        if (e->type() != QEvent::Expose) return false;
        auto* w = qobject_cast<QQuickWindow*>(o);
        if (!w) return false;
        if (!w->property("_tesseraPersistent").toBool()) {
            w->setProperty("_tesseraPersistent", true);
            w->setPersistentGraphics(true);
            w->setPersistentSceneGraph(true);
        }
        const bool now = w->isExposed();
        const QVariant was = w->property("_tesseraExposed");
        w->setProperty("_tesseraExposed", now);
        if (now && was.isValid() && !was.toBool()) {
            // (A burst of them counts once.)
            const qint64 ms = QDateTime::currentMSecsSinceEpoch();
            if (ms - w->property("_tesseraToldAt").toLongLong() >= 300 && w->metaObject()->indexOfSignal("reexposed()") >= 0) {
                w->setProperty("_tesseraToldAt", ms);
                QMetaObject::invokeMethod(w, "reexposed", Qt::QueuedConnection);
            }
        }
        return false;
    }
};

inline void tessera_create_app(const QString& session)
{
    // Native (FreeType/platform) text for the whole UI; Qt Quick's default
    // distance-field text looks soft at fractional scales.
    QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
    static int argc = 1;
    static char name[] = "tessera";
    static char flag[] = "-session";
    static QByteArray id;
    static char* argv[] = { name, nullptr, nullptr, nullptr };
    // Last run's session id, so the desktop can put the window back.
    if (!session.isEmpty()) {
        id = session.toLatin1();
        argv[1] = flag;
        argv[2] = id.data();
        argc = 3;
    }
    // QApplication (not QGuiApplication): Qt.labs.platform menus then use the
    // desktop's own menus (Breeze on KDE, with theme icons and shortcuts).
    // Its own Qt's plugins first: the desktop's QT_PLUGIN_PATH may be another
    // Qt's (a system update), and Qt won't load a platform plugin built for a
    // newer Qt than its own: the window wouldn't open. (Not QT_PLUGIN_PATH
    // itself: that would reach the shells and the programs run in them.)
    // (Put first just while Qt starts and reads it, then back as it was.)
    const QByteArray own = qgetenv("TESSERA_OWN_QT_PLUGINS");
    qunsetenv("TESSERA_OWN_QT_PLUGINS");
    const bool hadPath = qEnvironmentVariableIsSet("QT_PLUGIN_PATH");
    const QByteArray oldPath = qgetenv("QT_PLUGIN_PATH");
    if (!own.isEmpty()) qputenv("QT_PLUGIN_PATH", oldPath.isEmpty() ? own : own + ':' + oldPath);
    new QApplication(argc, argv);
    if (!own.isEmpty()) {
        if (hadPath) qputenv("QT_PLUGIN_PATH", oldPath);
        else qunsetenv("QT_PLUGIN_PATH");
    }
    qApp->installEventFilter(new TesseraCtrlTap(qApp));
    qApp->installEventFilter(new TesseraExposeWatch(qApp));
    QApplication::setApplicationName(QStringLiteral("tessera"));
    QApplication::setApplicationDisplayName(QStringLiteral("Tessera"));
    QApplication::setOrganizationName(QStringLiteral("tessera"));
    // The Wayland app id / X11 class, matching tessera.desktop.
    // (In a Flatpak the desktop file is named for the app id.)
    const QByteArray flatpakId = qgetenv("FLATPAK_ID");
    QGuiApplication::setDesktopFileName(flatpakId.isEmpty() ? QStringLiteral("tessera") : QString::fromUtf8(flatpakId));
    if (QIcon::themeName().isEmpty()) QIcon::setThemeName(QStringLiteral("breeze"));
    QIcon::setFallbackThemeName(QStringLiteral("breeze"));
    QApplication::setWindowIcon(QIcon::fromTheme(QStringLiteral("tessera"),
                                                 QIcon::fromTheme(QStringLiteral("utilities-terminal"))));
    qmlRegisterType<TerminalItem>("Tessera.Native", 1, 0, "TerminalView");
    qmlRegisterType<SyntaxHighlighter>("Tessera.Native", 1, 0, "SyntaxHighlighter");
    qmlRegisterType<FileClip>("Tessera.Native", 1, 0, "FileClip");
    // The window's menus in the desktop's global menu bar.
    qmlRegisterType<GlobalMenuBar>("GlobalMenu", 1, 0, "GlobalMenuBar");
}

inline void tessera_set_icon(const QString& path)
{
    if (QFile::exists(path)) QApplication::setWindowIcon(QIcon(path));
}

inline int tessera_exec() { return QApplication::exec(); }

inline QString tessera_session_id() { return qApp ? qApp->sessionId() : QString(); }

// Shows the main window (QML leaves it hidden), named first so the desktop
// can match it with the window it saw last time.
// For tests: a mouse wheel notch (dy: +120 up, -120 down) at a point in the
// main window, as the desktop would send it.
// phase: 0 none (a wheel), 1 begin, 2 update, 3 end (a touchpad, with
// pixel movement `px`).
// (A scroll with phases comes from a touchpad.)
inline const QPointingDevice* testTouchpad()
{
    static QPointingDevice pad("test touchpad", 9901, QInputDevice::DeviceType::TouchPad, QPointingDevice::PointerType::Finger,
                               QInputDevice::Capability::Position | QInputDevice::Capability::Scroll | QInputDevice::Capability::PixelScroll, 2, 0);
    return &pad;
}

inline void tessera_test_wheel(double x, double y, int dy, int px, int phase)
{
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        if (!w->property("showAtStart").toBool() || !w->isVisible()) continue;
        const QPointF p(x, y);
        const Qt::ScrollPhase ph = phase == 1 ? Qt::ScrollBegin : phase == 2 ? Qt::ScrollUpdate : phase == 3 ? Qt::ScrollEnd : Qt::NoScrollPhase;
        QWheelEvent e(p, w->mapToGlobal(p), QPoint(0, px), QPoint(0, dy), Qt::NoButton, Qt::NoModifier, ph, false, Qt::MouseEventNotSynthesized,
                      phase != 0 ? testTouchpad() : QPointingDevice::primaryPointingDevice());
        QGuiApplication::sendEvent(w, &e);
        return;
    }
}

// For tests: a mouse event in the main window (kind 0 press, 1 move, 2 release;
// button 1 left, 2 right).
inline void tessera_test_mouse(int kind, double x, double y, int button, int mods)
{
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        if (!w->property("showAtStart").toBool() || !w->isVisible()) continue;
        const QPointF p(x, y);
        const Qt::MouseButton b = button == 2 ? Qt::RightButton : Qt::LeftButton;
        // (kind 3: a move with no button held, to hover.)
        const QEvent::Type t = kind == 0 ? QEvent::MouseButtonPress : kind == 1 || kind == 3 ? QEvent::MouseMove : QEvent::MouseButtonRelease;
        const Qt::MouseButton which = kind == 1 || kind == 3 ? Qt::NoButton : b;
        const Qt::MouseButtons held = kind == 2 || kind == 3 ? Qt::MouseButtons() : Qt::MouseButtons(b);
        QMouseEvent e(t, p, p, w->mapToGlobal(p), which, held, Qt::KeyboardModifiers(mods));
        // (A real timestamp: taps are timed from the press.)
        static QElapsedTimer clock;
        if (!clock.isValid()) clock.start();
        e.setTimestamp(quint64(clock.elapsed()) + 1);
        QGuiApplication::sendEvent(w, &e);
        return;
    }
}

// For tests: a key press and release, to whatever has the keys in the main
// window (scan: the native scan code, for telling left and right Shift apart).
inline void tessera_test_key(int key, int mods, const QString& text, int scan)
{
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        if (!w->property("showAtStart").toBool() || !w->isVisible()) continue;
        const auto m = Qt::KeyboardModifiers(mods);
        QKeyEvent press(QEvent::KeyPress, key, m, quint32(scan), 0, 0, text);
        QGuiApplication::sendEvent(w, &press);
        QKeyEvent release(QEvent::KeyRelease, key, m, quint32(scan), 0, 0, text);
        QGuiApplication::sendEvent(w, &release);
        return;
    }
}

inline void tessera_show_windows()
{
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        if (!w->property("showAtStart").toBool() || w->isVisible()) continue;
        w->create();
#ifdef TESSERA_WAYLAND_RESTORE
        if (auto* wl = w->nativeInterface<QNativeInterface::Private::QWaylandWindow>())
            wl->setSessionRestoreId(w->objectName());
#endif
        // (Maximized when it was closed so: asked for in its first show.)
        if (w->property("startMaximized").toBool()) w->showMaximized();
        else w->setVisible(true);
    }
}

inline QString tessera_monospace_families()
{
    QJsonArray out;
    const QStringList families = QFontDatabase::families();
    for (const QString& f : families) {
        if (QFontDatabase::isPrivateFamily(f)) continue;
        if (QFontDatabase::isFixedPitch(f)) out.append(f);
    }
    return QString::fromUtf8(QJsonDocument(out).toJson(QJsonDocument::Compact));
}
