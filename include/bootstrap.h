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

inline void tessera_create_app()
{
    // Native (FreeType/platform) text for the whole UI; Qt Quick's default
    // distance-field text looks soft at fractional scales.
    QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
    static int argc = 1;
    static char name[] = "tessera";
    static char* argv[] = { name, nullptr };
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
