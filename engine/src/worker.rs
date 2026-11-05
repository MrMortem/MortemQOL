//! Runs one search at a time on a background thread. A newer search replaces an older one.
use crate::search;
use crate::snapshot;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PollStatus {
    Running,
    Finished,
    Error,
    Unknown,
}

#[derive(Default)]
struct Latest {
    id: u64,
    data: Option<Vec<u8>>,
    unread: bool,
    finished: bool,
    error: bool,
}

static NEXT_ID: AtomicU64 = AtomicU64::new(1);
static CURRENT: AtomicU64 = AtomicU64::new(0);
static LATEST: Mutex<Option<Latest>> = Mutex::new(None);
type Job = (u64, Vec<u8>);

static JOBS: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();

fn latest() -> MutexGuard<'static, Option<Latest>> {
    LATEST.lock().unwrap_or_else(|e| e.into_inner())
}

fn store(id: u64, data: Vec<u8>, finished: bool, error: bool) {
    let mut guard = latest();
    if let Some(l) = guard.as_mut().filter(|l| l.id == id) {
        l.data = Some(data);
        l.unread = true;
        l.finished = finished;
        l.error = error;
    }
}

fn process(id: u64, json: &[u8]) {
    let s = match snapshot::parse(json) {
        Ok(s) => s,
        Err(message) => return store(id, message.into_bytes(), false, true),
    };
    let cancel = || CURRENT.load(Ordering::Relaxed) != id;
    search::run(&s, &cancel, &mut |result| {
        let done = result.done;
        match serde_json::to_vec(&result.to_json()) {
            Ok(bytes) => store(id, bytes, done, false),
            Err(e) => store(id, e.to_string().into_bytes(), false, true),
        }
    });
}

fn worker_loop(jobs: std::sync::mpsc::Receiver<Job>) {
    while let Ok((mut id, mut json)) = jobs.recv() {
        // Only the newest queued request matters.
        while let Ok((next_id, next_json)) = jobs.try_recv() {
            id = next_id;
            json = next_json;
        }
        if catch_unwind(AssertUnwindSafe(|| process(id, &json))).is_err() {
            store(id, b"engine panic".to_vec(), false, true);
        }
    }
}

/// Starts a search for a snapshot in JSON and returns its id.
pub fn start(json: Vec<u8>) -> u64 {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    *latest() = Some(Latest { id, ..Latest::default() });
    CURRENT.store(id, Ordering::Relaxed);
    let sender = JOBS.get_or_init(|| {
        let (tx, rx) = channel();
        std::thread::spawn(move || worker_loop(rx));
        Mutex::new(tx)
    });
    let queued = sender.lock().unwrap_or_else(|e| e.into_inner()).send((id, json));
    if queued.is_err() {
        store(id, b"engine worker stopped".to_vec(), false, true);
    }
    id
}

/// Returns the search's status, plus any result not yet returned.
pub fn poll(id: u64) -> (PollStatus, Option<Vec<u8>>) {
    let mut guard = latest();
    let Some(l) = guard.as_mut().filter(|l| l.id == id) else {
        return (PollStatus::Unknown, None);
    };
    let data = if l.unread { l.data.clone() } else { None };
    l.unread = false;
    let status = if l.error {
        PollStatus::Error
    } else if l.finished {
        PollStatus::Finished
    } else {
        PollStatus::Running
    };
    (status, data)
}

/// Makes the last result available again, after a caller's buffer was too small.
pub fn mark_unread(id: u64) {
    if let Some(l) = latest().as_mut().filter(|l| l.id == id) {
        l.unread = true;
    }
}
