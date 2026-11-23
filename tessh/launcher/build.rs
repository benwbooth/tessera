// The payload inside the launcher (macOS: a signed program can't have data
// added after it, so it's built in): TESSH_PAYLOAD=<payload.tar.zst> at
// build time. Without it the payload is appended to the file (Linux).
fn main() {
    println!("cargo:rustc-check-cfg=cfg(tessh_embedded)");
    println!("cargo:rerun-if-env-changed=TESSH_PAYLOAD");
    let Some(p) = std::env::var_os("TESSH_PAYLOAD") else { return };
    let p = std::fs::canonicalize(&p).expect("TESSH_PAYLOAD: no such file");
    println!("cargo:rerun-if-changed={}", p.display());
    let data = std::fs::read(&p).expect("TESSH_PAYLOAD: can't read it");
    // (FNV-1a: names the folder it unpacks to, a new one per payload.)
    let mut h: u64 = 0xcbf29ce484222325;
    for b in &data {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    println!("cargo:rustc-cfg=tessh_embedded");
    println!("cargo:rustc-env=TESSH_PAYLOAD_FILE={}", p.display());
    println!("cargo:rustc-env=TESSH_PAYLOAD_HASH={h:016x}");
}
