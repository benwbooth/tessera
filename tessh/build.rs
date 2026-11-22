// Where this toolchain's libstdc++ is, for tessh to load at start: wheels
// from PyPI (numpy, pandas, matplotlib) link to it, and NixOS has none on
// the library path (runner::init).
fn main() {
    println!("cargo:rerun-if-env-changed=CXX");
    let cxx = std::env::var("CXX").unwrap_or_else(|_| "c++".into());
    for cc in [cxx.as_str(), "c++", "g++", "cc"] {
        let Ok(out) = std::process::Command::new(cc).arg("-print-file-name=libstdc++.so.6").output() else { continue };
        let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if p.starts_with('/') && std::path::Path::new(&p).exists() {
            let p = std::fs::canonicalize(&p).map(|x| x.to_string_lossy().to_string()).unwrap_or(p);
            println!("cargo:rustc-env=TESSH_LIBSTDCXX={p}");
            return;
        }
    }
}
