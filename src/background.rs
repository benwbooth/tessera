//! Work that would hold up the UI thread (tesserad requests, files, git,
//! subprocesses) runs here instead, and answers with a signal.
//!
//! `spawn` is for work that stands alone (a query). `in_order_on(lane, ..)`
//! is for work whose order matters, one job at a time per lane: "kernel"
//! (a kernel's sync before its execute, a notebook's write before the next
//! read, sessions), "files" (a save before the next save, a table's edits).
//! `drain` waits for the lanes at quit, so the last writes still land.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

type Job = Box<dyn FnOnce() + Send>;

fn lanes() -> &'static Mutex<HashMap<&'static str, Sender<Job>>> {
    static L: OnceLock<Mutex<HashMap<&'static str, Sender<Job>>>> = OnceLock::new();
    L.get_or_init(Default::default)
}

/// Run `f` on its own thread.
pub fn spawn(f: impl FnOnce() + Send + 'static) {
    std::thread::spawn(f);
}

/// Run `f` after everything sent to `lane` before it.
pub fn in_order_on(lane: &'static str, f: impl FnOnce() + Send + 'static) {
    let mut l = lanes().lock().unwrap();
    let tx = l.entry(lane).or_insert_with(|| {
        let (tx, rx) = channel::<Job>();
        std::thread::Builder::new()
            .name(format!("tessera-{lane}"))
            .spawn(move || {
                for job in rx {
                    job();
                }
            })
            .expect("ordered worker");
        tx
    });
    let _ = tx.send(Box::new(f));
}

/// The "kernel" lane.
pub fn in_order(f: impl FnOnce() + Send + 'static) {
    in_order_on("kernel", f)
}

/// Wait (at most `limit` in all) until every lane has run what was sent so far.
pub fn drain(limit: std::time::Duration) {
    let names: Vec<&'static str> = lanes().lock().unwrap().keys().copied().collect();
    let end = std::time::Instant::now() + limit;
    for lane in names {
        let (tx, rx) = channel();
        in_order_on(lane, move || {
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(end.saturating_duration_since(std::time::Instant::now()));
    }
}
