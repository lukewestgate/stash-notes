//! A self-test the Windows CI job runs (`STASH_SMOKE_TEST=1`): the main
//! window pins its note through the real command path, the pinned window
//! reports that it loaded and closes itself, and the app exits 0. If any
//! step hangs, a watchdog exits non-zero so CI fails instead of waiting.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static PIN_LOADED: AtomicBool = AtomicBool::new(false);

pub fn enabled() -> bool {
    std::env::var("STASH_SMOKE_TEST").is_ok_and(|v| v == "1")
}

/// Append to `%TEMP%/stash-smoke.log` (release builds have no console).
pub fn log(msg: &str) {
    eprintln!("smoke: {msg}");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(std::env::temp_dir().join("stash-smoke.log")) {
        let _ = writeln!(f, "{msg}");
    }
}

/// Exit with code 2 if the test hasn't finished in time. Uses the process
/// directly, since a hung event loop can't run `app.exit`.
pub fn start_watchdog() {
    log("started");
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(60));
        log("FAILED: timed out (a window or the event loop is stuck)");
        std::process::exit(2);
    });
}

pub fn step(name: &str) {
    log(name);
    if name == "pin-loaded" {
        PIN_LOADED.store(true, Ordering::SeqCst);
    }
}

/// Called after a pinned window closed.
pub fn pin_closed(app: &tauri::AppHandle) {
    if enabled() && PIN_LOADED.load(Ordering::SeqCst) {
        log("pin-closed: OK");
        app.exit(0);
    }
}
