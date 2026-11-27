//! `tessh --dap`: tessh's debugger, a Debug Adapter Protocol server for
//! editors (Turbine, VS Code). The protocol is on stdin and stdout; the
//! program gets /dev/null for input and its output (fd 1 and 2, its
//! commands' too) goes to a pipe the debugger sends on as output events.
//! The debugger itself is Python (dap.py): tessh's code keeps the script's
//! file name and line numbers, so Python's tracing sees tessh lines.

use crate::runner;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};

const CODE: &str = include_str!("dap.py");

pub fn run(py: Python<'_>) -> i32 {
    // The protocol keeps the real stdin and stdout; the program gets the rest.
    let (dap_in, dap_out, out_read) = unsafe {
        let dap_in = libc::dup(0);
        let dap_out = libc::dup(1);
        let mut p = [0i32; 2];
        if dap_in < 0 || dap_out < 0 || libc::pipe(p.as_mut_ptr()) != 0 {
            eprintln!("tessh --dap: can't set up its pipes");
            return 1;
        }
        libc::dup2(p[1], 1);
        libc::dup2(p[1], 2);
        libc::close(p[1]);
        let null = libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY);
        if null >= 0 {
            libc::dup2(null, 0);
            libc::close(null);
        }
        // (The protocol's own descriptors aren't for the program's children.)
        libc::fcntl(dap_in, libc::F_SETFD, libc::FD_CLOEXEC);
        libc::fcntl(dap_out, libc::F_SETFD, libc::FD_CLOEXEC);
        libc::fcntl(p[0], libc::F_SETFD, libc::FD_CLOEXEC);
        (dap_in, dap_out, p[0])
    };
    match serve(py, dap_in, dap_out, out_read) {
        Ok(code) => code,
        Err(e) => {
            // (Nothing to show it on but the protocol: an output event would
            // need the debugger; stderr is the pipe. Its exit code says.)
            e.print(py);
            1
        }
    }
}

fn serve(py: Python<'_>, dap_in: i32, dap_out: i32, out_read: i32) -> PyResult<i32> {
    let m = PyModule::from_code(py, &std::ffi::CString::new(CODE).unwrap(), c"<tessh dap>", c"__tessh_dap__")?;
    // run(program, args) -> exit code: the script, as `tessh FILE ARGS`.
    let run = pyo3::types::PyCFunction::new_closure(py, Some(c"run"), None, move |args: &Bound<'_, PyTuple>, _kw: Option<&Bound<'_, PyDict>>| -> PyResult<i32> {
        let py = args.py();
        let program: String = args.get_item(0)?.extract()?;
        let rest: Vec<String> = args.get_item(1)?.extract()?;
        let mut argv = vec![program.clone()];
        argv.extend(rest);
        let ns = runner::new_namespace(py, "__main__", argv)?;
        if let Some(d) = std::path::Path::new(&program).parent() {
            let _ = py.import("sys")?.getattr("path")?.call_method1("insert", (0, d.to_string_lossy().to_string()));
        }
        Ok(crate::finish(py, runner::run_file(py, &program, &ns), &program))
    })?;
    // tessh_eval(text, namespace): tessh code's value, for watches and the
    // debug console.
    let eval = pyo3::types::PyCFunction::new_closure(py, Some(c"tessh_eval"), None, move |args: &Bound<'_, PyTuple>, _kw: Option<&Bound<'_, PyDict>>| -> PyResult<Py<PyAny>> {
        let py = args.py();
        let text: String = args.get_item(0)?.extract()?;
        let ns = args.get_item(1)?.cast_into::<PyDict>()?;
        runner::eval_source(py, &text, &ns, "<debug>").map_err(|e| e.into_pyerr(py))
    })?;
    m.call_method1("serve", (dap_in, dap_out, out_read, run, eval))?.extract()
}
