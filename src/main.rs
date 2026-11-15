mod bridge;
mod client;
mod contrast;
mod daemon;
mod find;
mod keymap;
mod launchers;
mod proto;
mod settings;
mod session;
mod tmux;

use cxx_qt_lib::{QQmlApplicationEngine, QString, QUrl};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("daemon") {
        if let Err(e) = daemon::run() {
            eprintln!("tessera daemon: {e}");
            std::process::exit(1);
        }
        return;
    }
    // Options for the QML side (screenshots, a first command for testing).
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--screenshot" => {
                if let Some(p) = args.get(i + 1) {
                    let abs = std::env::current_dir().unwrap_or_default().join(p);
                    std::env::set_var("TESSERA_SCREENSHOT", abs);
                }
                i += 1;
            }
            "--new-window" => std::env::set_var("TESSERA_NO_RESTORE", "1"),
            "--find" => {
                if let Some(q) = args.get(i + 1) {
                    std::env::set_var("TESSERA_FIND", q);
                }
                i += 1;
            }
            "--tab" => {
                if let Some(f) = args.get(i + 1) {
                    std::env::set_var("TESSERA_TAB_FILE", f);
                }
                i += 1;
            }
            "--prefs" => {
                if let Some(c) = args.get(i + 1) {
                    std::env::set_var("TESSERA_PREFS", c);
                }
                i += 1;
            }
            "--then" => {
                if let Some(c) = args.get(i + 1) {
                    std::env::set_var("TESSERA_THEN", c);
                }
                i += 1;
            }
            "--run" => {
                if let Some(c) = args.get(i + 1) {
                    std::env::set_var("TESSERA_RUN", c);
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    std::env::set_var("QT_QUICK_CONTROLS_STYLE", std::env::var("TESSERA_STYLE").unwrap_or_else(|_| "Basic".into()));
    bridge::ffi::tessera_create_app();
    bridge::ffi::tessera_set_icon(&QString::from(concat!(env!("CARGO_MANIFEST_DIR"), "/icons/tessera.svg")));
    let mut engine = QQmlApplicationEngine::new();
    if let Some(mut engine) = engine.as_mut() {
        if let Some(qml) = option_env!("TESSERA_QT_QML") {
            if std::path::Path::new(qml).exists() {
                engine.as_mut().add_import_path(&QString::from(qml));
            }
        }
        engine.load(&QUrl::from("qrc:/qt/qml/Tessera/qml/Main.qml"));
    }
    let code = bridge::ffi::tessera_exec();
    std::process::exit(code);
}
