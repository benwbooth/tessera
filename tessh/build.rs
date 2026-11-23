// Where this toolchain's libstdc++ is, for tessh to load at start: wheels
// from PyPI (numpy, pandas, matplotlib) link to it, and NixOS has none on
// the library path (runner::init).
//
// The standalone build (feature "standalone", for musl: tessh/launcher)
// instead gets a long placeholder for its program loader, which the
// launcher overwrites with the path of the musl loader it unpacks, and finds
// libpython3.so (uv's CPython, linked in by the launcher) beside itself;
// built for glibc (its other half), the system's loader instead.
const INTERP_PLACEHOLDER_LEN: usize = 240;

fn main() {
    println!("cargo:rerun-if-env-changed=CXX");
    let musl = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("musl");
    if std::env::var_os("CARGO_FEATURE_STANDALONE").is_some() && musl {
        let placeholder = format!("/TESSH-INTERP-{}", "X".repeat(INTERP_PLACEHOLDER_LEN - 14));
        println!("cargo:rustc-link-arg-bins=-Wl,--dynamic-linker={placeholder}");
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN/../lib");
        return;
    }
    // (The standalone glibc build, picked by the launcher where the system
    // has glibc: the system's loader, libpython3.so beside it.)
    if std::env::var_os("CARGO_FEATURE_STANDALONE").is_some() {
        println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN/../lib");
        return;
    }
    if musl {
        return;
    }
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
