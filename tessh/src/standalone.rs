//! The standalone build (feature "standalone"; see tessh/launcher): this
//! binary runs out of `~/.cache/tessh/rt-*/py/<version>/bin/tessh`, set up by
//! the static launcher, with that folder's `lib` holding links to the
//! CPython uv installed (libpython3.so, lib/python3.X) and its `bin` links
//! `python3`, `python3.X` back to this binary.
//!
//! Called as `python3` it is that Python (for uv, and sys.executable).
//! Otherwise embedded Python is pointed at that folder, since an embedded
//! interpreter would look for `python3` on PATH and take that one's library.

use pyo3::prelude::*;
use std::ffi::{CString, OsString};
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

extern "C" {
    fn Py_BytesMain(argc: libc::c_int, argv: *mut *mut libc::c_char) -> libc::c_int;
}

/// The version folder this binary runs from, when it is one (bin/tessh,
/// with lib/python3.X beside).
fn home() -> Option<PathBuf> {
    let exe = std::fs::read_link("/proc/self/exe").ok()?;
    let home = exe.parent()?.parent()?.to_path_buf();
    let lib = std::fs::read_dir(home.join("lib")).ok()?;
    lib.flatten().any(|e| e.file_name().to_string_lossy().starts_with("python3.")).then_some(home)
}

fn is_python_name(argv0: &OsString) -> bool {
    let base = std::path::Path::new(argv0).file_name().map(|b| b.to_string_lossy().to_string()).unwrap_or_default();
    let rest = base.strip_prefix("python").unwrap_or("-");
    rest.is_empty() || rest == "3" || rest.strip_prefix("3.").is_some_and(|m| !m.is_empty() && m.chars().all(|c| c.is_ascii_digit()))
}

/// Called as python3 (one of the links): run as that Python and exit.
pub fn python_main() {
    let args: Vec<OsString> = std::env::args_os().collect();
    if args.first().is_none_or(|a| !is_python_name(a)) {
        return;
    }
    let Some(home) = home() else { return };
    let mut cs: Vec<CString> = args.into_iter().map(|a| CString::new(a.into_vec()).unwrap_or_default()).collect();
    // (By its link's full path, so Python finds its folder from it.)
    cs[0] = CString::new(home.join("bin/python3").into_os_string().into_vec()).unwrap_or_default();
    let mut ptrs: Vec<*mut libc::c_char> = cs.iter().map(|c| c.as_ptr() as *mut libc::c_char).collect();
    ptrs.push(std::ptr::null_mut());
    let code = unsafe { Py_BytesMain((ptrs.len() - 1) as libc::c_int, ptrs.as_mut_ptr()) };
    std::process::exit(code);
}

/// Before Python starts: its home is this folder. (Put back once it has
/// started: programs tessh runs mustn't inherit it.)
pub fn prepare() -> Option<(PathBuf, Option<OsString>)> {
    let home = home()?;
    let old = std::env::var_os("PYTHONHOME");
    unsafe { std::env::set_var("PYTHONHOME", &home) };
    Python::attach(|py| -> PyResult<()> {
        let os = py.import("os")?;
        let environ = os.getattr("environ")?;
        match &old {
            Some(v) => {
                unsafe { std::env::set_var("PYTHONHOME", v) };
                environ.set_item("PYTHONHOME", v.to_string_lossy().to_string())?;
            }
            None => {
                unsafe { std::env::remove_var("PYTHONHOME") };
                if environ.contains("PYTHONHOME")? {
                    environ.del_item("PYTHONHOME")?;
                }
            }
        }
        // (sys.executable: this Python, by its python3 link.)
        let sys = py.import("sys")?;
        let exe = home.join("bin").join("python3").to_string_lossy().to_string();
        sys.setattr("executable", &exe)?;
        sys.setattr("_base_executable", &exe)?;
        Ok(())
    })
    .ok()?;
    Some((home, old))
}

#[cfg(test)]
mod tests {
    #[test]
    fn python_names() {
        for (n, ok) in [("python3", true), ("/x/bin/python3.14", true), ("python", true), ("tessh", false), ("python-foo", false), ("python3.", false)] {
            assert_eq!(super::is_python_name(&n.into()), ok, "{n}");
        }
    }
}
