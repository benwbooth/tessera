//! PEP 723 inline script metadata (the blocks `uv run` reads): the PyPI
//! packages a script or cell names, installed with uv into a folder kept by
//! their set (built for this Python) and put on sys.path.

use pyo3::prelude::*;
use std::process::Command;

/// The `script` block's TOML, if there is one.
pub fn metadata(text: &str) -> Option<toml::Table> {
    let mut lines = text.lines();
    while let Some(l) = lines.next() {
        if l.trim_end() != "# /// script" {
            continue;
        }
        let mut body = String::new();
        for l in lines.by_ref() {
            if l.trim_end() == "# ///" {
                return body.parse::<toml::Table>().ok();
            }
            if let Some(r) = l.strip_prefix("# ") {
                body.push_str(r);
            } else if l == "#" {
            } else {
                break;
            }
            body.push('\n');
        }
    }
    None
}

fn cache_dir() -> std::path::PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache"));
    base.join("tessh").join("deps")
}

fn uv_command() -> Option<Vec<String>> {
    if let Some(uv) = crate::runtime::which("uv") {
        return Some(vec![uv]);
    }
    if crate::runtime::which("nix").is_some() {
        return Some(["nix", "run", "--quiet", "nixpkgs#uv", "--"].map(String::from).to_vec());
    }
    None
}

/// The Python program for uv to install for: this interpreter's.
fn python_program(py: Python<'_>) -> PyResult<String> {
    let sys = py.import("sys")?;
    let prefix: String = sys.getattr("base_prefix")?.extract()?;
    let (major, minor): (u8, u8) = (sys.getattr("version_info")?.get_item(0)?.extract()?, sys.getattr("version_info")?.get_item(1)?.extract()?);
    for name in [format!("python{major}.{minor}"), format!("python{major}"), "python".into()] {
        let p = std::path::Path::new(&prefix).join("bin").join(&name);
        if p.exists() {
            return Ok(p.to_string_lossy().to_string());
        }
    }
    Ok(format!("{major}.{minor}"))
}

pub fn ensure(py: Python<'_>, deps: &[String], requires: Option<&str>) -> PyResult<()> {
    if let Some(req) = requires {
        let here = py.import("platform")?.call_method0("python_version")?.extract::<String>()?;
        let ok = py.import("packaging.specifiers").and_then(|m| m.getattr("SpecifierSet")?.call1((req,))?.contains(&here)).unwrap_or(true);
        if !ok {
            eprintln!("tessh: this script wants Python {req}; this is {here}");
        }
    }
    let mut deps: Vec<String> = deps.to_vec();
    deps.sort();
    deps.dedup();
    if deps.is_empty() {
        return Ok(());
    }
    let version: String = py.import("sys")?.getattr("version")?.extract()?;
    use sha2::Digest;
    let key = hex::encode(&sha2::Sha256::digest(format!("{}\n{version}", deps.join("\n")).as_bytes())[..8]);
    let target = cache_dir().join(key);
    let done = target.join(".tessh-done");
    if !done.exists() {
        let Some(uv) = uv_command() else {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!("this script needs packages ({}) and uv isn't installed (nor Nix to get it)", deps.join(", "))));
        };
        eprintln!("tessh: installing {} with uv…", deps.join(", "));
        std::fs::create_dir_all(&target)?;
        let python = python_program(py)?;
        let status = py.detach(|| Command::new(&uv[0]).args(&uv[1..]).args(["pip", "install", "--quiet", "--python", &python, "--target"]).arg(&target).args(&deps).status());
        if !status.map(|s| s.success()).unwrap_or(false) {
            return Err(pyo3::exceptions::PyRuntimeError::new_err(format!("uv couldn't install {}", deps.join(", "))));
        }
        std::fs::write(&done, b"")?;
    }
    let path = py.import("sys")?.getattr("path")?;
    let t = target.to_string_lossy().to_string();
    if !path.contains(&t)? {
        path.call_method1("insert", (0, t))?;
    }
    Ok(())
}

pub fn ensure_from_source(py: Python<'_>, text: &str) -> PyResult<()> {
    if !text.contains("# /// script") {
        return Ok(());
    }
    let Some(meta) = metadata(text) else { return Ok(()) };
    let deps: Vec<String> = meta.get("dependencies").and_then(|d| d.as_array()).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default();
    ensure(py, &deps, meta.get("requires-python").and_then(|r| r.as_str()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_the_block() {
        let m = super::metadata("# /// script\n# dependencies = [\"rich\"]\n# ///\nprint hi\n").unwrap();
        assert_eq!(m["dependencies"].as_array().unwrap()[0].as_str(), Some("rich"));
        assert!(super::metadata("print hi").is_none());
    }
}
