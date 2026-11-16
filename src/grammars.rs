//! More languages for the editor, loaded when needed: Helix's tree-sitter
//! collection (a compiled grammar and highlight queries for each of ~250
//! languages), wherever Helix keeps its runtime.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tree_sitter::Language;

/// Helix runtime folders that hold grammars and queries.
pub fn runtime_dirs() -> &'static [PathBuf] {
    static DIRS: OnceLock<Vec<PathBuf>> = OnceLock::new();
    DIRS.get_or_init(|| {
        let mut dirs: Vec<PathBuf> = Vec::new();
        if let Some(r) = std::env::var_os("HELIX_RUNTIME") {
            dirs.push(PathBuf::from(r));
        }
        if let Some(h) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(&h).join(".config/helix/runtime"));
            dirs.push(PathBuf::from(&h).join(".local/share/tessera/helix-runtime"));
        }
        // Nix wraps hx with its runtime's path: read it from the wrapper.
        if let Some(path) = std::env::var_os("PATH") {
            for d in std::env::split_paths(&path) {
                let hx = d.join("hx");
                if let Ok(bytes) = std::fs::read(&hx) {
                    if let Some(i) = bytes.windows(14).position(|w| w == b"HELIX_RUNTIME=") {
                        let rest = &bytes[i + 14..];
                        let end = rest.iter().position(|&b| b == 0 || b == b' ' || b == b'\n' || b == b'\'' || b == b'"').unwrap_or(rest.len());
                        dirs.push(PathBuf::from(String::from_utf8_lossy(&rest[..end]).into_owned()));
                    }
                    if let Ok(real) = std::fs::canonicalize(&hx) {
                        if let Some(prefix) = real.parent().and_then(Path::parent) {
                            dirs.push(prefix.join("lib/runtime"));
                            dirs.push(prefix.join("share/helix/runtime"));
                        }
                    }
                }
            }
        }
        dirs.push(PathBuf::from("/usr/lib/helix/runtime"));
        dirs.push(PathBuf::from("/usr/share/helix/runtime"));
        dirs.retain(|d| d.join("grammars").is_dir());
        dirs.dedup();
        dirs
    })
}

fn grammar_file(name: &str) -> Option<PathBuf> {
    let ext = if cfg!(windows) { "dll" } else if cfg!(target_os = "macos") { "dylib" } else { "so" };
    runtime_dirs().iter().map(|d| d.join("grammars").join(format!("{name}.{ext}"))).find(|p| p.is_file())
        .or_else(|| runtime_dirs().iter().map(|d| d.join("grammars").join(format!("{name}.so"))).find(|p| p.is_file()))
}

/// Every language there's a grammar for, sorted.
pub fn available() -> &'static [String] {
    static LIST: OnceLock<Vec<String>> = OnceLock::new();
    LIST.get_or_init(|| {
        let mut out: Vec<String> = Vec::new();
        for d in runtime_dirs() {
            if let Ok(rd) = std::fs::read_dir(d.join("grammars")) {
                for e in rd.flatten() {
                    let p = e.path();
                    if let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().into_owned()) {
                        if !out.contains(&stem) {
                            out.push(stem);
                        }
                    }
                }
            }
        }
        out.sort();
        out
    })
}

/// Fetch Helix's grammars and queries (its latest release, for this
/// computer) into Tessera's data folder. Returns how many languages.
pub fn download() -> Result<usize, String> {
    let (os, ext) = if cfg!(windows) { ("windows", "zip") } else if cfg!(target_os = "macos") { ("macos", "tar.xz") } else { ("linux", "tar.xz") };
    let arch = if cfg!(target_arch = "aarch64") { "aarch64" } else { "x86_64" };
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(std::time::Duration::from_secs(600))).build().into();
    let release: serde_json::Value = agent
        .get("https://api.github.com/repos/helix-editor/helix/releases/latest")
        .header("User-Agent", "tessera")
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json()
        .map_err(|e| e.to_string())?;
    let url = release["assets"]
        .as_array()
        .and_then(|a| a.iter().find_map(|x| {
            let n = x["name"].as_str()?;
            (n.contains(&format!("{arch}-{os}")) && n.ends_with(ext)).then(|| x["browser_download_url"].as_str().map(str::to_string)).flatten()
        }))
        .ok_or("no Helix release for this computer")?;
    let home = std::env::var_os("HOME").ok_or("no home folder")?;
    let dest = PathBuf::from(home).join(".local/share/tessera/helix-runtime");
    let tmp = std::env::temp_dir().join(format!("tessera-helix.{ext}"));
    let mut resp = agent.get(&url).call().map_err(|e| e.to_string())?;
    let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
    std::io::copy(&mut resp.body_mut().as_reader(), &mut f).map_err(|e| e.to_string())?;
    drop(f);
    let unpack = std::env::temp_dir().join("tessera-helix");
    let _ = std::fs::remove_dir_all(&unpack);
    std::fs::create_dir_all(&unpack).map_err(|e| e.to_string())?;
    let ok = std::process::Command::new("tar").arg("-xf").arg(&tmp).arg("-C").arg(&unpack).status().map_err(|e| format!("couldn't run tar: {e}"))?;
    if !ok.success() {
        return Err("couldn't unpack Helix's release".into());
    }
    // The archive's runtime folder (inside its top folder).
    let runtime = std::fs::read_dir(&unpack).map_err(|e| e.to_string())?.flatten().map(|e| e.path().join("runtime")).find(|p| p.join("grammars").is_dir()).ok_or("no grammars in the release")?;
    let _ = std::fs::remove_dir_all(&dest);
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    for part in ["grammars", "queries"] {
        copy_dir(&runtime.join(part), &dest.join(part)).map_err(|e| e.to_string())?;
    }
    let _ = std::fs::remove_dir_all(&unpack);
    let _ = std::fs::remove_file(&tmp);
    Ok(std::fs::read_dir(dest.join("grammars")).map(|r| r.count()).unwrap_or(0))
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let p = e.path();
        let t = to.join(e.file_name());
        if p.is_dir() {
            copy_dir(&p, &t)?;
        } else {
            std::fs::copy(&p, &t)?;
        }
    }
    Ok(())
}

pub fn count() -> usize {
    available().len()
}

/// A highlights query with Helix's "; inherits: a,b" lines resolved (the
/// parents' rules first).
pub fn highlights(name: &str) -> String {
    fn read(name: &str, depth: usize, out: &mut String) {
        if depth > 6 {
            return;
        }
        let Some(text) = runtime_dirs().iter().find_map(|d| std::fs::read_to_string(d.join("queries").join(name).join("highlights.scm")).ok()) else {
            return;
        };
        for line in text.lines() {
            if let Some(parents) = line.trim().strip_prefix("; inherits:") {
                for p in parents.split(',') {
                    read(p.trim(), depth + 1, out);
                }
            }
        }
        out.push_str(&text);
        out.push('\n');
    }
    let mut out = String::new();
    read(name, 0, &mut out);
    out
}

/// Load a grammar (kept loaded for the rest of the run).
pub fn load(name: &str) -> Option<Language> {
    let file = grammar_file(name)?;
    let lib = unsafe { libloading::Library::new(&file) }.ok()?;
    let lib: &'static libloading::Library = Box::leak(Box::new(lib));
    let sym = format!("tree_sitter_{}", name.replace('-', "_"));
    let f: libloading::Symbol<'static, unsafe extern "C" fn() -> *const ()> = unsafe { lib.get(sym.as_bytes()) }.ok()?;
    let lang: Language = unsafe { tree_sitter_language::LanguageFn::from_raw(*f) }.into();
    // A grammar from a tree-sitter this one can't read is left alone.
    let v = lang.abi_version();
    (tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION..=tree_sitter::LANGUAGE_VERSION).contains(&v).then_some(lang)
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore]
    fn helix_runtime() {
        println!("dirs: {:?}", super::runtime_dirs());
        println!("{} grammars", super::available().len());
        for n in ["haskell", "elixir", "zig", "nix", "kotlin"] {
            let l = super::load(n);
            let q = super::highlights(n);
            let ok = l.as_ref().map(|l| tree_sitter::Query::new(l, &q).map(|q| q.capture_names().len()).map_err(|e| e.to_string()));
            println!("{n}: loaded {} query {:?}", l.is_some(), ok);
        }
    }
}
