// Process-wide Qt setup: the application object and QML registration.
#pragma once

#include <QtCore/QFile>
#include <QtCore/QJsonArray>
#include <QtCore/QJsonDocument>
#include <QtGui/QFontDatabase>
#include <QtGui/QIcon>
#include <QtQml/qqml.h>
#include <QtQuick/QQuickWindow>
#include <QtWidgets/QApplication>

#include "terminal_item.h"

// Wayland desktops place windows themselves; with the session-restore
// extension (xx-session-management) they put each one back where it was.
#if __has_include(<QtGui/qpa/qplatformwindow_p.h>)
#include <QtGui/qpa/qplatformwindow_p.h>
#if QT_CONFIG(wayland)
#define TESSERA_WAYLAND_RESTORE 1
#endif
#endif

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
    new QApplication(argc, argv);
    QApplication::setApplicationName(QStringLiteral("tessera"));
    QApplication::setApplicationDisplayName(QStringLiteral("Tessera"));
    QApplication::setOrganizationName(QStringLiteral("tessera"));
    // The Wayland app id / X11 class, matching tessera.desktop.
    QGuiApplication::setDesktopFileName(QStringLiteral("tessera"));
    if (QIcon::themeName().isEmpty()) QIcon::setThemeName(QStringLiteral("breeze"));
    QIcon::setFallbackThemeName(QStringLiteral("breeze"));
    QApplication::setWindowIcon(QIcon::fromTheme(QStringLiteral("tessera"),
                                                 QIcon::fromTheme(QStringLiteral("utilities-terminal"))));
    qmlRegisterType<TerminalItem>("Tessera.Native", 1, 0, "TerminalView");
}

inline void tessera_set_icon(const QString& path)
{
    if (QFile::exists(path)) QApplication::setWindowIcon(QIcon(path));
}

inline int tessera_exec() { return QApplication::exec(); }

inline QString tessera_session_id() { return qApp ? qApp->sessionId() : QString(); }

// Shows the main window (QML leaves it hidden), named first so the desktop
// can match it with the window it saw last time.
inline void tessera_show_windows()
{
    for (QWindow* w : QGuiApplication::topLevelWindows()) {
        if (!w->property("showAtStart").toBool() || w->isVisible()) continue;
        w->create();
#ifdef TESSERA_WAYLAND_RESTORE
        if (auto* wl = w->nativeInterface<QNativeInterface::Private::QWaylandWindow>())
            wl->setSessionRestoreId(w->objectName());
#endif
        w->setVisible(true);
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
