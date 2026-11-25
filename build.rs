use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    // The dev shell links a merged Qt tree here; elsewhere set QMAKE or put
    // qmake on PATH.
    if std::env::var_os("QMAKE").is_none() {
        let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
        let local = manifest.join(".qt-env").join("bin").join("qmake");
        if local.exists() {
            std::env::set_var("QMAKE", &local);
        }
    }
    println!("cargo::rerun-if-env-changed=QMAKE");
    println!("cargo::rerun-if-changed=include");
    println!("cargo::rerun-if-changed=cpp");
    let qmake = std::env::var("QMAKE").unwrap_or_else(|_| "qmake6".into());
    if let Ok(o) = std::process::Command::new(&qmake).args(["-query", "QT_INSTALL_QML"]).output() {
        let qml = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !qml.is_empty() {
            println!("cargo::rustc-env=TESSERA_QT_QML={qml}");
        }
    }
    // Its own Qt's plugins, looked in first (see bootstrap.h).
    if let Ok(o) = std::process::Command::new(&qmake).args(["-query", "QT_INSTALL_PLUGINS"]).output() {
        let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !p.is_empty() {
            println!("cargo::rustc-env=TESSERA_QT_PLUGINS={p}");
        }
    }

    // Qt's private window interface (Wayland session restore: the desktop
    // puts the window back where it was).
    let mut private_includes = Vec::new();
    let query = |var: &str| {
        std::process::Command::new(&qmake)
            .args(["-query", var])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    };
    if let (Some(headers), Some(version)) = (query("QT_INSTALL_HEADERS"), query("QT_VERSION")) {
        // (macOS's Qt keeps them in its frameworks: QtGui.framework/Headers/<version>.)
        let libs = query("QT_INSTALL_LIBS").unwrap_or_default();
        for module in ["QtCore", "QtGui"] {
            for dir in [
                std::path::Path::new(&headers).join(module).join(&version),
                std::path::Path::new(&libs).join(format!("{module}.framework")).join("Headers").join(&version),
            ] {
                if dir.exists() {
                    private_includes.push(dir.clone());
                    private_includes.push(dir.join(module));
                }
            }
        }
    }

    let mut builder = CxxQtBuilder::new_qml_module(QmlModule::new("Tessera").qml_files([
        QmlFile::from("qml/Main.qml"),
        QmlFile::from("qml/Theme.qml").singleton(true),
        QmlFile::from("qml/Async.qml").singleton(true),
        QmlFile::from("qml/TerminalPane.qml"),
        QmlFile::from("qml/PaneSplit.qml"),
        QmlFile::from("qml/TabButtonX.qml"),
        QmlFile::from("qml/Preferences.qml"),
        QmlFile::from("qml/FindBar.qml"),
        QmlFile::from("qml/Sheet.qml"),
        QmlFile::from("qml/BarButton.qml"),
        QmlFile::from("qml/SplitButton.qml"),
        QmlFile::from("qml/WindowControls.qml"),
        QmlFile::from("qml/MomentumWheel.qml"),
        QmlFile::from("qml/ComposeBox.qml"),
        QmlFile::from("qml/GuidePage.qml"),
        QmlFile::from("qml/ValueTree.qml"),
        QmlFile::from("qml/PromptBar.qml"),
        QmlFile::from("qml/AssistantPanel.qml"),
        QmlFile::from("qml/RichViewer.qml"),
        QmlFile::from("qml/NotebookView.qml"),
        QmlFile::from("qml/BlockSelect.qml"),
        QmlFile::from("qml/Chip.qml"),
        QmlFile::from("qml/SlimCombo.qml"),
        QmlFile::from("qml/ThemeBar.qml"),
        QmlFile::from("qml/NixHelp.qml"),
        QmlFile::from("qml/CapShape.qml"),
        QmlFile::from("qml/SlimMenu.qml"),
        QmlFile::from("qml/SlimMenuItem.qml"),
        QmlFile::from("qml/AppMenu.qml"),
        QmlFile::from("qml/MacComboBox.qml"),
        QmlFile::from("qml/MacComboItem.qml"),
        QmlFile::from("qml/MacComboPopup.qml"),
        QmlFile::from("qml/MacMenuBackground.qml"),
        QmlFile::from("qml/MacMenuIcon.qml"),
        QmlFile::from("qml/MacMenuItem.qml"),
        QmlFile::from("qml/MacMenu.qml"),
        QmlFile::from("qml/MacMenuSeparator.qml"),
        QmlFile::from("qml/MacMenuStyle.qml"),
        QmlFile::from("qml/ChromeButtons.qml"),
        QmlFile::from("qml/ChromeResizeEdges.qml"),
        QmlFile::from("qml/ChromeTitleArea.qml"),
        QmlFile::from("qml/StarshipLine.qml"),
        QmlFile::from("qml/FileBrowser.qml"),
        QmlFile::from("qml/FilePreview.qml"),
        QmlFile::from("qml/FileView.qml"),
        QmlFile::from("qml/CsvTable.qml"),
        QmlFile::from("qml/EditKeys.qml"),
        QmlFile::from("qml/ViKeys.qml"),
        QmlFile::from("qml/ViBlock.qml"),
        QmlFile::from("qml/ExCommands.qml"),
        QmlFile::from("qml/ExLine.qml"),
    ]))
    .crate_include_root(None)
    .include_dir("include");
    for dir in private_includes {
        builder = builder.include_dir(dir);
    }
    builder
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("QuickShapes")
    .qt_module("Widgets")
    .qt_module("WebEngineQuick")
    .cpp_file("include/terminal_item.h")
    .cpp_file("include/syntax_highlighter.h")
    .cpp_file("include/file_clip.h")
    .cpp_file("include/icon_theme.h")
    .cpp_file("cpp/terminal_item.cpp")
    // The menus in the desktop's global menu bar (QPA private headers).
    .cpp_file("include/global_menu.h")
    .cpp_file("cpp/global_menu.cpp")
    .files(["src/bridge.rs"])
    .build();
}
