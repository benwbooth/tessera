// Process-wide Qt setup done before the application object exists.
#pragma once

#include <QtQml/qqml.h>
#include <QtQuick/QQuickWindow>

#include "terminal_item.h"

inline void tessera_prepare_qt()
{
    // Native (FreeType/platform) text for the whole UI; Qt Quick's default
    // distance-field text looks soft at fractional scales.
    QQuickWindow::setTextRenderType(QQuickWindow::NativeTextRendering);
    qmlRegisterType<TerminalItem>("Tessera.Native", 1, 0, "TerminalView");
}
