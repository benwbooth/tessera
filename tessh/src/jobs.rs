//! Job control: the job table (`cmd &`, a program stopped with Ctrl+Z, a
//! Python call run in the background), job specs (`%1`, `%%`, `%+`, `%-`,
//! `%name`, `%?text`), and the builtins that use them: jobs, fg, bg, wait,
//! disown, kill, suspend and logout. When an interactive tessh ends, or the
//! terminal hangs up, its jobs get SIGHUP (and SIGCONT, if stopped), as
//! zsh's do; `disown` keeps one going.

use crate::runtime::{exit_code, output, py_print, set_status, tty_ok, INTERACTIVE};
use pyo3::exceptions::{PyIndexError, PyOSError, PyTypeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};
use std::process::Child;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

pub struct JobInfo {
    /// Its own, never reused (a Job value holds it).
    id: usize,
    /// Its number, [n] and %n: one more than the highest still listed, as
    /// bash numbers them, so the first job of a fresh list is 1 again.
    pub n: usize,
    pgid: i32,
    pids: Vec<i32>,
    words: String,
    stopped: bool,
    thread: Option<Py<PyAny>>,
    children: Vec<Child>,
    /// When it last stopped or went to the background: the latest is the
    /// current job (`%+`), the one before the previous (`%-`).
    seq: u64,
    /// `disown -h`: kept on the list, but not sent SIGHUP when tessh ends.
    nohup: bool,
    /// Its exit status, once it has ended.
    status: Option<i32>,
    /// Said to be done (by `jobs`, or before a prompt): no longer listed, but
    /// `wait %n` and `$job.wait` still find its status.
    reported: bool,
}

static JOBS: Mutex<Vec<JobInfo>> = Mutex::new(Vec::new());
static JOB_N: AtomicUsize = AtomicUsize::new(0);
static SEQ: AtomicU64 = AtomicU64::new(0);

fn next_seq() -> u64 {
    SEQ.fetch_add(1, Ordering::SeqCst) + 1
}

fn next_n(jobs: &[JobInfo]) -> usize {
    jobs.iter().filter(|j| !j.reported).map(|j| j.n).max().unwrap_or(0) + 1
}

/// A job for programs (running in the background, or stopped): (its id,
/// its number). `again`: the id and number it had before (brought back to
/// the foreground, then stopped again).
pub fn add_job(children: Vec<Child>, pgid: i32, words: String, stopped: bool, again: Option<(usize, usize)>) -> (usize, usize) {
    let pids = children.iter().map(|c| c.id() as i32).collect();
    let mut jobs = JOBS.lock().unwrap();
    let (id, n) = again.unwrap_or_else(|| (JOB_N.fetch_add(1, Ordering::SeqCst) + 1, next_n(&jobs)));
    // (Old finished jobs nobody asked about: let go of, past a hundred.)
    while jobs.iter().filter(|j| j.reported).count() > 100 {
        let i = jobs.iter().position(|j| j.reported).unwrap();
        jobs.remove(i);
    }
    jobs.push(JobInfo { id, n, pgid, pids, words, stopped, thread: None, children, seq: next_seq(), nohup: false, status: None, reported: false });
    (id, n)
}

/// Whether it has ended (its status kept, the processes reaped).
fn job_done(j: &mut JobInfo, py: Python<'_>) -> bool {
    if j.status.is_some() {
        return true;
    }
    if let Some(t) = &j.thread {
        let alive = t.call_method0(py, "is_alive").and_then(|v| v.extract::<bool>(py)).unwrap_or(false);
        if !alive {
            j.status = Some(0);
        }
        return !alive;
    }
    if j.stopped {
        return false;
    }
    let mut code = None;
    for c in j.children.iter_mut() {
        match c.try_wait() {
            Ok(Some(s)) => code = Some(exit_code(s)),
            Ok(None) => return false,
            Err(_) => code = code.or(Some(0)),
        }
    }
    j.status = Some(code.unwrap_or(0));
    true
}

fn state(j: &mut JobInfo, py: Python<'_>) -> String {
    if job_done(j, py) {
        match j.status {
            Some(0) | None => "done".into(),
            Some(c) => format!("exit {c}"),
        }
    } else if j.stopped {
        "stopped".into()
    } else {
        "running".into()
    }
}

/// A background job's value: `$job.wait`, `wait $job`, `kill $job` (it reads
/// as `%n` in a command's words), `$job.pid`.
#[pyclass(module = "__tessh__", name = "Job", frozen)]
pub struct JobRef {
    id: usize,
    /// Just started, and "[n] pid" said so: the REPL needn't show it too.
    fresh: std::sync::atomic::AtomicBool,
    /// Its programs' process IDs, kept: still there once it's disowned.
    pids: Vec<i32>,
}

impl JobRef {
    fn with<T>(&self, f: impl FnOnce(&mut JobInfo) -> T) -> Option<T> {
        JOBS.lock().unwrap().iter_mut().find(|j| j.id == self.id).map(f)
    }
}

#[pymethods]
impl JobRef {
    /// Wait for it, in the background (the terminal stays tessh's): its
    /// result (a Python job) or its exit status, as `wait %n`.
    fn wait(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        wait_id(py, self.id)
    }

    /// Its number: [n], %n.
    #[getter]
    fn n(&self) -> Option<usize> {
        self.with(|j| j.n)
    }

    /// Bring it to the foreground, as `fg %n`.
    fn fg(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        fg(py, Some(&self.__str__()))
    }

    /// The process ID of its first program (bash's `$!` for it); None for a
    /// Python job.
    #[getter]
    fn pid(&self) -> Option<i32> {
        self.pids.first().copied()
    }

    /// The process IDs of its programs (a pipe has several).
    #[getter]
    fn pids(&self) -> Vec<i32> {
        self.pids.clone()
    }

    /// Its exit status once it has ended; None while it runs.
    #[getter]
    fn status(&self, py: Python<'_>) -> Option<i32> {
        self.with(|j| {
            job_done(j, py);
            j.status
        })
        .flatten()
    }

    /// In a command's words: %n while it's listed, else its process ID
    /// (its number may be another job's by then).
    fn __str__(&self) -> String {
        self.with(|j| if j.reported && !j.pids.is_empty() { j.pids[0].to_string() } else { format!("%{}", j.n) }).or_else(|| self.pids.first().map(|p| p.to_string())).unwrap_or_else(|| "%0".into())
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        let mut jobs = JOBS.lock().unwrap();
        match jobs.iter_mut().find(|j| j.id == self.id) {
            Some(j) => {
                let what = state(j, py);
                format!("[{}] {what} {}", j.n, j.words)
            }
            None => "[?] done".into(),
        }
    }
}

pub fn bg_call(py: Python<'_>, thunk: Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
    let threading = py.import("threading")?;
    let holder = PyList::empty(py);
    let code = std::ffi::CString::new("def _run(f, out):\n    out.append(f())\n").unwrap();
    let helper = PyModule::from_code(py, &code, c"<tessh bg>", c"__tessh_bg__")?.getattr("_run")?;
    let kw = PyDict::new(py);
    kw.set_item("target", helper)?;
    kw.set_item("args", (thunk, &holder))?;
    kw.set_item("daemon", true)?;
    let th = threading.getattr("Thread")?.call((), Some(&kw))?;
    th.call_method0("start")?;
    let id = JOB_N.fetch_add(1, Ordering::SeqCst) + 1;
    th.setattr("_tessh_result", holder)?;
    let mut jobs = JOBS.lock().unwrap();
    let n = next_n(&jobs);
    jobs.push(JobInfo { id, n, pgid: 0, pids: vec![], words: "(python)".into(), stopped: false, thread: Some(th.unbind()), children: vec![], seq: next_seq(), nohup: false, status: None, reported: false });
    drop(jobs);
    Ok(Py::new(py, JobRef { id, pids: vec![], fresh: false.into() })?.into_any())
}

pub fn job_ref(py: Python<'_>, id: usize) -> PyResult<Py<PyAny>> {
    let pids = JOBS.lock().unwrap().iter().find(|j| j.id == id).map(|j| j.pids.clone()).unwrap_or_default();
    Ok(Py::new(py, JobRef { id, pids, fresh: INTERACTIVE.load(Ordering::SeqCst).into() })?.into_any())
}

/// Whether v is a job just started at the prompt (once: shown after that).
pub fn announced(v: &Bound<'_, PyAny>) -> bool {
    v.cast::<JobRef>().is_ok_and(|j| j.get().fresh.swap(false, Ordering::SeqCst))
}

// ---- job specs -------------------------------------------------------------------------

/// The job a spec names, as an index into the table: `%n` (or `n`, for fg
/// and bg), `%%`, `%+` or `%` (the current job: the latest stopped or sent to
/// the background), `%-` (the one before), `%text` (its command starts with
/// text), `%?text` (its command has text in it). None: the current job.
fn find(jobs: &mut [JobInfo], py: Python<'_>, spec: Option<&str>, bare_number: bool) -> PyResult<usize> {
    let live: Vec<usize> = {
        let mut v: Vec<usize> = (0..jobs.len()).filter(|&i| !jobs[i].reported).collect();
        v.sort_by_key(|&i| std::cmp::Reverse(jobs[i].seq));
        v
    };
    let current = |k: usize| -> PyResult<usize> {
        live.get(k).or(live.first()).copied().ok_or_else(|| PyIndexError::new_err(if spec.is_none() { "no current job".to_string() } else { format!("{}: no such job", spec.unwrap()) }))
    };
    let Some(s) = spec else { return current(0) };
    let body = match s.strip_prefix('%') {
        Some(b) => b,
        None if bare_number && !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()) => s,
        None => return Err(PyValueError::new_err(format!("{s}: no such job (a job is %n, %%, %+, %-, %name or %?text)"))),
    };
    if let Some(id) = body.strip_prefix('@').and_then(|d| d.parse::<usize>().ok()) {
        return jobs.iter().position(|j| j.id == id).ok_or_else(|| PyIndexError::new_err(format!("{s}: no such job")));
    }
    match body {
        "" | "%" | "+" => current(0),
        "-" => current(1),
        _ if body.bytes().all(|c| c.is_ascii_digit()) => {
            let k: usize = body.parse().map_err(|_| PyValueError::new_err(format!("{s}: no such job")))?;
            live.iter().copied().find(|&i| jobs[i].n == k).or_else(|| (0..jobs.len()).rev().find(|&i| jobs[i].n == k)).ok_or_else(|| PyIndexError::new_err(format!("{s}: no such job")))
        }
        _ => {
            let (inside, text) = match body.strip_prefix('?') {
                Some(t) => (true, t),
                None => (false, body),
            };
            let hits: Vec<usize> = live.iter().copied().filter(|&i| if inside { jobs[i].words.contains(text) } else { jobs[i].words.starts_with(text) }).collect();
            let _ = py;
            match hits.len() {
                0 => Err(PyIndexError::new_err(format!("{s}: no such job"))),
                1 => Ok(hits[0]),
                _ => Err(PyValueError::new_err(format!("{s}: ambiguous job spec"))),
            }
        }
    }
}

// ---- fg, bg, jobs ----------------------------------------------------------------------

pub fn fg(py: Python<'_>, spec: Option<&str>) -> PyResult<Py<PyAny>> {
    let job = {
        let mut jobs = JOBS.lock().unwrap();
        let i = find(&mut jobs, py, spec, true)?;
        if jobs[i].reported || (jobs[i].thread.is_none() && job_done(&mut jobs[i], py)) {
            // (Ended already: its status, as wait gives it.)
            let id = jobs[i].id;
            drop(jobs);
            return wait_id(py, id);
        }
        jobs.remove(i)
    };
    if let Some(th) = &job.thread {
        let th = th.bind(py);
        th.call_method0("join")?;
        let res = th.getattr("_tessh_result")?;
        let v = if res.len()? > 0 { res.get_item(0)?.unbind() } else { py.None() };
        return Ok(v);
    }
    if INTERACTIVE.load(Ordering::SeqCst) {
        let _ = py_print(py, &job.words);
    }
    unsafe { libc::killpg(job.pgid, libc::SIGCONT) };
    let words = job.words.clone();
    let again = (job.id, job.n);
    let code = if tty_ok() {
        py.detach(|| wait_foreground(job.children, job.pgid, words, Some(again)))
    } else {
        let mut children = job.children;
        py.detach(|| {
            let mut code = 0;
            for c in children.iter_mut() {
                code = c.wait().map(exit_code).unwrap_or(0);
            }
            code
        })
    };
    set_status(code);
    output(py, "", code, true)
}

pub fn bg(py: Python<'_>, specs: &[String]) -> PyResult<()> {
    let specs: Vec<Option<&str>> = if specs.is_empty() { vec![None] } else { specs.iter().map(|s| Some(s.as_str())).collect() };
    for spec in specs {
        let line = {
            let mut jobs = JOBS.lock().unwrap();
            let i = find(&mut jobs, py, spec, true)?;
            let j = &mut jobs[i];
            if j.thread.is_some() {
                return Err(PyValueError::new_err(format!("%{}: a Python job runs in the background already", j.n)));
            }
            if job_done(j, py) {
                return Err(PyValueError::new_err(format!("%{}: the job has ended", j.n)));
            }
            j.stopped = false;
            j.seq = next_seq();
            unsafe { libc::killpg(j.pgid, libc::SIGCONT) };
            format!("[{}] {} &", j.n, j.words)
        };
        if INTERACTIVE.load(Ordering::SeqCst) {
            py_print(py, &line)?;
        }
    }
    Ok(())
}

/// jobs [-lprs] [spec…]: [n] running, stopped, done or exit N, then the
/// command; `+` marks the current job and `-` the previous one. -l adds the
/// process IDs, -p prints only them, -r only running jobs, -s only stopped.
pub fn jobs(py: Python<'_>, args: &[String]) -> PyResult<()> {
    let (mut long, mut pids_only, mut running, mut stopped) = (false, false, false, false);
    let mut specs = vec![];
    for a in args {
        if a.starts_with('-') && a.len() > 1 && specs.is_empty() {
            for c in a[1..].chars() {
                match c {
                    'l' => long = true,
                    'p' => pids_only = true,
                    'r' => running = true,
                    's' => stopped = true,
                    'n' => {}
                    c => return Err(PyValueError::new_err(format!("jobs: -{c}: invalid option\njobs: usage: jobs [-lprs] [jobspec …]"))),
                }
            }
        } else {
            specs.push(a.clone());
        }
    }
    let lines = {
        let mut jobs = JOBS.lock().unwrap();
        for j in jobs.iter_mut() {
            job_done(j, py);
        }
        let mut order: Vec<usize> = (0..jobs.len()).filter(|&i| !jobs[i].reported).collect();
        order.sort_by_key(|&i| std::cmp::Reverse(jobs[i].seq));
        let (cur, prev) = (order.first().map(|&i| jobs[i].n), order.get(1).map(|&i| jobs[i].n));
        let mut pick: Vec<usize> = if specs.is_empty() {
            (0..jobs.len()).filter(|&i| !jobs[i].reported).collect()
        } else {
            specs.iter().map(|s| find(&mut jobs, py, Some(s), true)).collect::<PyResult<_>>()?
        };
        pick.sort_by_key(|&i| jobs[i].n);
        let mut lines = vec![];
        for i in pick {
            let j = &mut jobs[i];
            let done = j.status.is_some();
            if (running && (done || j.stopped)) || (stopped && !j.stopped) {
                continue;
            }
            if pids_only {
                lines.extend(j.pids.iter().map(|p| p.to_string()));
            } else {
                let mark = if Some(j.n) == cur { "+" } else if Some(j.n) == prev { "-" } else { "" };
                let what = state(j, py);
                let pid = if long { j.pids.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(",") + " " } else { String::new() };
                // ("[1] running sleep 5", as before, with the marks after the number.)
                lines.push(format!("[{}]{mark} {pid}{what} {}", j.n, j.words));
            }
            if done {
                j.reported = true;
            }
        }
        lines
    };
    for l in lines {
        py_print(py, &l)?;
    }
    Ok(())
}

/// Before a prompt: the jobs that have ended since, said once, as bash's
/// "[1]+ Done".
#[pyfunction]
pub fn job_notices(py: Python<'_>) -> PyResult<()> {
    if !INTERACTIVE.load(Ordering::SeqCst) {
        return Ok(());
    }
    let lines = {
        let mut jobs = JOBS.lock().unwrap();
        let mut lines = vec![];
        for j in jobs.iter_mut().filter(|j| !j.reported) {
            if job_done(j, py) {
                j.reported = true;
                let what = state(j, py);
                lines.push(format!("[{}] {what} {}", j.n, j.words));
            }
        }
        lines
    };
    if !lines.is_empty() {
        let err = py.import("sys")?.getattr("stderr")?;
        err.call_method1("write", (lines.join("\n") + "\n",))?;
        err.call_method0("flush")?;
    }
    Ok(())
}

// ---- wait ------------------------------------------------------------------------------

/// Wait for one job, in the background: a Python job's result, or the exit
/// status (as an Output) of a program's. Ctrl+C stops the waiting, not the job.
fn wait_one(py: Python<'_>, spec: &str) -> PyResult<Py<PyAny>> {
    let id = {
        let mut jobs = JOBS.lock().unwrap();
        let i = find(&mut jobs, py, Some(spec), false)?;
        jobs[i].id
    };
    wait_id(py, id)
}

fn wait_id(py: Python<'_>, id: usize) -> PyResult<Py<PyAny>> {
    let thread = {
        let mut jobs = JOBS.lock().unwrap();
        let Some(j) = jobs.iter_mut().find(|j| j.id == id) else {
            set_status(127);
            return output(py, "", 127, true);
        };
        if j.stopped {
            return Err(PyValueError::new_err(format!("%{}: the job is stopped (bg %{} lets it go on)", j.n, j.n)));
        }
        j.thread.as_ref().map(|t| t.clone_ref(py))
    };
    if let Some(th) = thread {
        let th = th.bind(py);
        loop {
            th.call_method1("join", (0.05,))?;
            if !th.call_method0("is_alive")?.extract::<bool>()? {
                break;
            }
            py.check_signals()?;
        }
        if let Some(j) = JOBS.lock().unwrap().iter_mut().find(|j| j.id == id) {
            j.status = Some(0);
            j.reported = true;
        }
        let res = th.getattr("_tessh_result")?;
        return Ok(if res.len()? > 0 { res.get_item(0)?.unbind() } else { py.None() });
    }
    loop {
        let done = {
            let mut jobs = JOBS.lock().unwrap();
            match jobs.iter_mut().find(|j| j.id == id) {
                Some(j) => {
                    if job_done(j, py) {
                        j.reported = true;
                        Some(j.status.unwrap_or(0))
                    } else if j.stopped {
                        Some(148)
                    } else {
                        None
                    }
                }
                None => Some(127),
            }
        };
        if let Some(code) = done {
            set_status(code);
            return output(py, "", code, true);
        }
        py.detach(|| std::thread::sleep(std::time::Duration::from_millis(20)));
        py.check_signals()?;
    }
}

/// wait [-n] [spec | pid …]: each named job (or a job's process ID), or all
/// of them; the status is the last one's (0 when it waited for all). -n
/// waits for whichever ends first.
pub fn wait(py: Python<'_>, args: &[String]) -> PyResult<Py<PyAny>> {
    let mut any = false;
    let mut specs: Vec<String> = vec![];
    for a in args {
        match a.as_str() {
            "-n" => any = true,
            "-f" | "--" => {}
            _ => specs.push(a.clone()),
        }
    }
    // (Process IDs: the job they're in.)
    let mut named: Vec<String> = vec![];
    for s in &specs {
        if s.starts_with('%') {
            named.push(s.clone());
            continue;
        }
        let pid: i32 = s.parse().map_err(|_| PyValueError::new_err(format!("{s}: not a process ID or job spec")))?;
        let id = JOBS.lock().unwrap().iter().rev().find(|j| j.pids.contains(&pid)).map(|j| j.id);
        match id {
            Some(id) => named.push(format!("%@{id}")),
            None => {
                let msg = format!("tessh: wait: pid {pid} is not a child of this shell\n");
                py.import("sys")?.getattr("stderr")?.call_method1("write", (msg,))?;
                set_status(127);
                return output(py, "", 127, true);
            }
        }
    }
    if any {
        let pool: Vec<usize> = if named.is_empty() {
            let mut jobs = JOBS.lock().unwrap();
            jobs.iter_mut().filter(|j| !j.reported && !j.stopped).map(|j| j.id).collect()
        } else {
            let mut jobs = JOBS.lock().unwrap();
            named.iter().map(|s| find(&mut jobs, py, Some(s), false).map(|i| jobs[i].id)).collect::<PyResult<_>>()?
        };
        if pool.is_empty() {
            set_status(127);
            return output(py, "", 127, true);
        }
        loop {
            let hit = {
                let mut jobs = JOBS.lock().unwrap();
                let mut hit = None;
                for j in jobs.iter_mut() {
                    if pool.contains(&j.id) && job_done(j, py) {
                        hit = Some(j.id);
                        break;
                    }
                }
                hit
            };
            if let Some(id) = hit {
                return wait_id(py, id);
            }
            py.detach(|| std::thread::sleep(std::time::Duration::from_millis(20)));
            py.check_signals()?;
        }
    }
    if named.is_empty() {
        // (All of them: the running ones; a stopped job would never end.)
        let all: Vec<usize> = JOBS.lock().unwrap().iter().filter(|j| !j.reported && !j.stopped).map(|j| j.id).collect();
        for id in all {
            wait_id(py, id)?;
        }
        set_status(0);
        return output(py, "", 0, true);
    }
    let mut last = py.None();
    for s in &named {
        last = wait_one(py, s)?;
    }
    Ok(last)
}

// ---- disown ----------------------------------------------------------------------------

/// disown [-ahr] [spec…]: off the job list, so it isn't sent SIGHUP when
/// tessh ends (-h: kept on the list, but not sent it). -a: every job, -r:
/// every running one.
pub fn disown(py: Python<'_>, args: &[String]) -> PyResult<()> {
    let (mut keep, mut all, mut running) = (false, false, false);
    let mut specs = vec![];
    for a in args {
        if a.starts_with('-') && a.len() > 1 {
            for c in a[1..].chars() {
                match c {
                    'h' => keep = true,
                    'a' => all = true,
                    'r' => running = true,
                    c => return Err(PyValueError::new_err(format!("disown: -{c}: invalid option\ndisown: usage: disown [-h] [-ar] [jobspec … | pid …]"))),
                }
            }
        } else {
            specs.push(a.clone());
        }
    }
    let mut jobs = JOBS.lock().unwrap();
    let mut pick: Vec<usize> = vec![];
    if all || running {
        pick = (0..jobs.len()).filter(|&i| !jobs[i].reported && (!running || !jobs[i].stopped)).collect();
    } else if specs.is_empty() {
        pick.push(find(&mut jobs, py, None, false)?);
    } else {
        for s in &specs {
            let i = match s.parse::<i32>() {
                Ok(pid) if !s.starts_with('%') => jobs.iter().position(|j| j.pids.contains(&pid)).ok_or_else(|| PyIndexError::new_err(format!("{s}: no such job")))?,
                _ => find(&mut jobs, py, Some(s), false)?,
            };
            pick.push(i);
        }
    }
    if keep {
        for i in pick {
            jobs[i].nohup = true;
        }
        return Ok(());
    }
    pick.sort();
    pick.dedup();
    for i in pick.into_iter().rev() {
        let mut j = jobs.remove(i);
        // (Still tessh's children: reaped when they end, so none is left a zombie.)
        let children = std::mem::take(&mut j.children);
        if !children.is_empty() {
            std::thread::spawn(move || {
                for mut c in children {
                    let _ = c.wait();
                }
            });
        }
    }
    Ok(())
}

// ---- kill ------------------------------------------------------------------------------

const SIGNALS: &[(&str, i32)] = &[
    ("HUP", libc::SIGHUP),
    ("INT", libc::SIGINT),
    ("QUIT", libc::SIGQUIT),
    ("ILL", libc::SIGILL),
    ("TRAP", libc::SIGTRAP),
    ("ABRT", libc::SIGABRT),
    ("BUS", libc::SIGBUS),
    ("FPE", libc::SIGFPE),
    ("KILL", libc::SIGKILL),
    ("USR1", libc::SIGUSR1),
    ("SEGV", libc::SIGSEGV),
    ("USR2", libc::SIGUSR2),
    ("PIPE", libc::SIGPIPE),
    ("ALRM", libc::SIGALRM),
    ("TERM", libc::SIGTERM),
    ("STKFLT", libc::SIGSTKFLT),
    ("CHLD", libc::SIGCHLD),
    ("CONT", libc::SIGCONT),
    ("STOP", libc::SIGSTOP),
    ("TSTP", libc::SIGTSTP),
    ("TTIN", libc::SIGTTIN),
    ("TTOU", libc::SIGTTOU),
    ("URG", libc::SIGURG),
    ("XCPU", libc::SIGXCPU),
    ("XFSZ", libc::SIGXFSZ),
    ("VTALRM", libc::SIGVTALRM),
    ("PROF", libc::SIGPROF),
    ("WINCH", libc::SIGWINCH),
    ("IO", libc::SIGIO),
    ("PWR", libc::SIGPWR),
    ("SYS", libc::SIGSYS),
];

/// A signal by name (TERM, SIGTERM, sigterm, IOT…) or number.
pub fn signal_number(s: &str) -> Option<i32> {
    if let Ok(n) = s.parse::<i32>() {
        return (0..=64).contains(&n).then_some(n);
    }
    let u = s.to_ascii_uppercase();
    let u = u.strip_prefix("SIG").unwrap_or(&u);
    match u {
        "IOT" => return Some(libc::SIGABRT),
        "POLL" => return Some(libc::SIGIO),
        "CLD" => return Some(libc::SIGCHLD),
        _ => {}
    }
    if let Some(k) = u.strip_prefix("RTMIN+").and_then(|k| k.parse::<i32>().ok()) {
        return Some(libc::SIGRTMIN() + k);
    }
    if let Some(k) = u.strip_prefix("RTMAX-").and_then(|k| k.parse::<i32>().ok()) {
        return Some(libc::SIGRTMAX() - k);
    }
    match u {
        "RTMIN" => return Some(libc::SIGRTMIN()),
        "RTMAX" => return Some(libc::SIGRTMAX()),
        _ => {}
    }
    SIGNALS.iter().find(|(n, _)| *n == u).map(|(_, v)| *v)
}

pub fn signal_name(n: i32) -> Option<String> {
    SIGNALS.iter().find(|(_, v)| *v == n).map(|(k, _)| k.to_string())
}

#[pyfunction]
#[pyo3(name = "signal_number")]
fn py_signal_number(s: &str) -> Option<i32> {
    signal_number(s)
}

#[pyfunction]
#[pyo3(name = "signal_names")]
fn py_signal_names() -> Vec<(String, i32)> {
    SIGNALS.iter().map(|(n, v)| (n.to_string(), *v)).collect()
}

/// kill [-s SIG | -n NUM | -SIG] spec|pid …, kill -l [n …]: a signal (TERM
/// without one) to jobs (each of its programs) and processes. A stopped job
/// sent TERM or HUP gets CONT too, so it can act on it.
pub fn kill(py: Python<'_>, args: &[String]) -> PyResult<Py<PyAny>> {
    let usage = "kill: usage: kill [-s sigspec | -n signum | -sigspec] pid | jobspec … or kill -l [sigspec]";
    let mut sig = libc::SIGTERM;
    let mut rest: &[String] = args;
    match rest.first().map(String::as_str) {
        None => return Err(PyTypeError::new_err(usage)),
        Some("-l") | Some("-L") | Some("--list") => {
            if rest.len() == 1 {
                let all: Vec<String> = SIGNALS.iter().map(|(n, v)| format!("{v:2}) SIG{n}")).collect();
                for row in all.chunks(5) {
                    py_print(py, &row.join("\t"))?;
                }
                return Ok(py.None());
            }
            for a in &rest[1..] {
                // (A number: its name; a status above 128, the signal it died of; a name: its number.)
                match a.parse::<i32>() {
                    Ok(n) => py_print(py, &signal_name(if n > 128 { n - 128 } else { n }).ok_or_else(|| PyValueError::new_err(format!("kill: {a}: invalid signal specification")))?)?,
                    Err(_) => py_print(py, &signal_number(a).ok_or_else(|| PyValueError::new_err(format!("kill: {a}: invalid signal specification")))?.to_string())?,
                }
            }
            return Ok(py.None());
        }
        Some("-s") | Some("-n") => {
            let s = rest.get(1).ok_or_else(|| PyTypeError::new_err(usage))?;
            sig = signal_number(s).ok_or_else(|| PyValueError::new_err(format!("kill: {s}: invalid signal specification")))?;
            rest = &rest[2..];
        }
        Some("--") => rest = &rest[1..],
        Some(a) if a.starts_with('-') && a.len() > 1 && !a[1..].starts_with(|c: char| c == '-') => {
            sig = signal_number(&a[1..]).ok_or_else(|| PyValueError::new_err(format!("kill: {}: invalid signal specification", &a[1..])))?;
            rest = &rest[1..];
            if rest.first().map(String::as_str) == Some("--") {
                rest = &rest[1..];
            }
        }
        _ => {}
    }
    if rest.is_empty() {
        return Err(PyTypeError::new_err(usage));
    }
    let mut code = 0;
    let err = py.import("sys")?.getattr("stderr")?;
    for t in rest {
        let r: Result<(), String> = if t.starts_with('%') {
            let mut jobs = JOBS.lock().unwrap();
            match find(&mut jobs, py, Some(t), false) {
                Err(e) => Err(e.value(py).to_string()),
                Ok(i) => {
                    let j = &mut jobs[i];
                    if j.thread.is_some() {
                        Err(format!("{t}: a Python job (a thread) can't be sent a signal"))
                    } else if job_done(j, py) {
                        Err(format!("{t}: the job has ended"))
                    } else if unsafe { libc::killpg(j.pgid, sig) } != 0 {
                        Err(format!("{t}: {}", std::io::Error::last_os_error()))
                    } else {
                        if j.stopped && (sig == libc::SIGTERM || sig == libc::SIGHUP) {
                            unsafe { libc::killpg(j.pgid, libc::SIGCONT) };
                        }
                        if sig == libc::SIGCONT {
                            j.stopped = false;
                        }
                        Ok(())
                    }
                }
            }
        } else {
            match t.parse::<i32>() {
                Ok(pid) => {
                    if unsafe { libc::kill(pid, sig) } != 0 {
                        Err(format!("({pid}) - {}", std::io::Error::last_os_error()))
                    } else {
                        Ok(())
                    }
                }
                Err(_) => Err(format!("{t}: arguments must be process or job IDs")),
            }
        };
        if let Err(m) = r {
            err.call_method1("write", (format!("tessh: kill: {m}\n"),))?;
            code = 1;
        }
    }
    let _ = err.call_method0("flush");
    set_status(code);
    output(py, "", code, true)
}

// ---- suspend, logout, the end ----------------------------------------------------------

/// suspend [-f]: stop tessh, as Ctrl+Z stops a program, until the shell
/// that started it continues it (fg). A login shell only with -f.
pub fn suspend(args: &[String]) -> PyResult<()> {
    let force = args.iter().any(|a| a == "-f");
    if !INTERACTIVE.load(Ordering::SeqCst) {
        return Err(PyOSError::new_err("suspend: only at the prompt: a script has no shell to come back to"));
    }
    if crate::LOGIN.load(Ordering::SeqCst) && !force {
        return Err(PyOSError::new_err("suspend: a login shell can't be suspended (suspend -f does it anyway)"));
    }
    unsafe { libc::kill(libc::getpid(), libc::SIGSTOP) };
    Ok(())
}

/// What `exit` checks first: with jobs left (an interactive shell), it says
/// so once and stays; `exit` again ends them (SIGHUP) and leaves.
static WARNED: Mutex<Vec<usize>> = Mutex::new(Vec::new());

pub fn exit_check(py: Python<'_>) -> PyResult<bool> {
    if !INTERACTIVE.load(Ordering::SeqCst) {
        return Ok(true);
    }
    let (left, stopped): (Vec<usize>, usize) = {
        let mut jobs = JOBS.lock().unwrap();
        let mut v = vec![];
        let mut s = 0;
        for j in jobs.iter_mut() {
            if !j.nohup && j.thread.is_none() && !job_done(j, py) {
                v.push(j.n);
                if j.stopped {
                    s += 1;
                }
            }
        }
        (v, s)
    };
    if left.is_empty() {
        return Ok(true);
    }
    let mut w = WARNED.lock().unwrap();
    if *w == left {
        return Ok(true);
    }
    *w = left.clone();
    let running = left.len() - stopped;
    let what = match (running, stopped) {
        (0, s) => format!("{s} stopped job{}", if s == 1 { "" } else { "s" }),
        (r, 0) => format!("{r} running job{}", if r == 1 { "" } else { "s" }),
        (r, s) => format!("{r} running and {s} stopped jobs"),
    };
    let msg = format!("tessh: you have {what} (jobs lists them): exit again to end them, or disown one to keep it going\n");
    py.import("sys")?.getattr("stderr")?.call_method1("write", (msg,))?;
    Ok(false)
}

/// When an interactive tessh ends (atexit) or its terminal hangs up: its
/// jobs get SIGHUP, and SIGCONT if stopped, except those disowned.
#[pyfunction]
pub fn hup_jobs() {
    if !INTERACTIVE.load(Ordering::SeqCst) {
        return;
    }
    // (try_lock: from a signal handler, a lock the shell holds is let be.)
    if let Ok(jobs) = JOBS.try_lock() {
        for j in jobs.iter().filter(|j| !j.nohup && j.pgid > 0 && j.status.is_none()) {
            unsafe {
                libc::killpg(j.pgid, libc::SIGHUP);
                if j.stopped {
                    libc::killpg(j.pgid, libc::SIGCONT);
                }
            }
        }
    }
}

/// Whether `trap … EXIT` is set: then a hangup lets tessh end its own way
/// (the terminal gone, its input ends), so the trap runs.
static EXIT_TRAP: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[pyfunction]
fn set_exit_trap(on: bool) {
    EXIT_TRAP.store(on, Ordering::SeqCst);
}

/// The write end of the pipe that wakes the hangup thread.
static HUP_PIPE: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(-1);

extern "C" fn on_hup(_: libc::c_int) {
    hup_jobs();
    unsafe {
        let w = HUP_PIPE.load(Ordering::SeqCst);
        if EXIT_TRAP.load(Ordering::SeqCst) && w >= 0 {
            // (The trap is Python, which a signal handler can't run: the
            // hangup thread runs it and ends tessh. The terminal's hangup
            // can come more than once: let it be. Ended anyway in 5 s.)
            libc::signal(libc::SIGHUP, libc::SIG_IGN);
            libc::write(w, b"h".as_ptr() as *const libc::c_void, 1);
            libc::alarm(5);
            return;
        }
        libc::signal(libc::SIGHUP, libc::SIG_DFL);
        libc::raise(libc::SIGHUP);
    }
}

/// An interactive tessh: SIGHUP (the terminal closed) passes it on to the
/// jobs, then ends tessh as it would have.
pub fn on_hangup() {
    static THREAD: std::sync::Once = std::sync::Once::new();
    THREAD.call_once(|| {
        let mut fds = [0; 2];
        if unsafe { libc::pipe2(fds.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            return;
        }
        // (Out of the way, high: the low numbers are for redirections and <(…).)
        for f in fds.iter_mut() {
            let high = unsafe { libc::fcntl(*f, libc::F_DUPFD_CLOEXEC, 200) };
            if high >= 0 {
                unsafe { libc::close(*f) };
                *f = high;
            }
        }
        HUP_PIPE.store(fds[1], Ordering::SeqCst);
        let r = fds[0];
        std::thread::spawn(move || {
            let mut b = [0u8; 1];
            if unsafe { libc::read(r, b.as_mut_ptr() as *mut libc::c_void, 1) } == 1 {
                Python::attach(|py| {
                    if let Ok(a) = py.import("atexit") {
                        let _ = a.call_method0("_run_exitfuncs");
                    }
                });
                unsafe { libc::_exit(128 + libc::SIGHUP) };
            }
        });
    });
    unsafe {
        libc::signal(libc::SIGHUP, on_hup as *const () as libc::sighandler_t);
    }
}

/// Give the terminal to the programs, wait (or see them stopped: a job,
/// keeping the id and number it had, if it had them), take it back.
pub fn wait_foreground(children: Vec<Child>, pgid: i32, words: String, again: Option<(usize, usize)>) -> i32 {
    unsafe {
        let old = libc::signal(libc::SIGTTOU, libc::SIG_IGN);
        libc::tcsetpgrp(0, pgid);
        let pids: Vec<i32> = children.iter().map(|c| c.id() as i32).collect();
        let last = *pids.last().unwrap_or(&0);
        let mut left: std::collections::HashSet<i32> = pids.iter().copied().collect();
        let mut code = 0;
        let mut stopped = false;
        while !left.is_empty() {
            let mut st = 0;
            let pid = libc::waitpid(-pgid, &mut st, libc::WUNTRACED);
            if pid < 0 {
                if std::io::Error::last_os_error().raw_os_error() == Some(libc::EINTR) {
                    continue;
                }
                break;
            }
            if libc::WIFSTOPPED(st) {
                stopped = true;
                break;
            }
            left.remove(&pid);
            if pid == last {
                code = if libc::WIFEXITED(st) { libc::WEXITSTATUS(st) } else { 128 + libc::WTERMSIG(st) };
            }
        }
        libc::tcsetpgrp(0, libc::getpgrp());
        libc::signal(libc::SIGTTOU, old);
        if stopped {
            let (_, n) = add_job(children, pgid, words.clone(), true, again);
            eprintln!("\n[{n}] stopped {words}");
            return 148;
        }
        // (Reaped already: the Child handles must not wait again.)
        std::mem::forget(children);
        code
    }
}

pub fn add(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<JobRef>()?;
    m.add_function(wrap_pyfunction!(job_notices, m)?)?;
    m.add_function(wrap_pyfunction!(hup_jobs, m)?)?;
    m.add_function(wrap_pyfunction!(set_exit_trap, m)?)?;
    m.add_function(wrap_pyfunction!(py_signal_number, m)?)?;
    m.add_function(wrap_pyfunction!(py_signal_names, m)?)?;
    Ok(())
}
