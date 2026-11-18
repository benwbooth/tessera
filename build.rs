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
        for module in ["QtCore", "QtGui"] {
            let dir = std::path::Path::new(&headers).join(module).join(&version);
            if dir.exists() {
                private_includes.push(dir.clone());
                private_includes.push(dir.join(module));
            }
        }
    }

    let mut builder = CxxQtBuilder::new_qml_module(QmlModule::new("Tessera").qml_files([
        QmlFile::from("qml/Main.qml"),
        QmlFile::from("qml/Theme.qml").singleton(true),
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
        QmlFile::from("qml/PromptBar.qml"),
        QmlFile::from("qml/AssistantPanel.qml"),
        QmlFile::from("qml/RichViewer.qml"),
        QmlFile::from("qml/NotebookView.qml"),
        QmlFile::from("qml/BlockSelect.qml"),
        QmlFile::from("qml/Chip.qml"),
        QmlFile::from("qml/SlimCombo.qml"),
        QmlFile::from("qml/SlimMenu.qml"),
        QmlFile::from("qml/SlimMenuItem.qml"),
        QmlFile::from("qml/StarshipLine.qml"),
        QmlFile::from("qml/FileBrowser.qml"),
        QmlFile::from("qml/FilePreview.qml"),
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
    .cpp_file("cpp/terminal_item.cpp")
    .files(["src/bridge.rs"])
    .build();
}
