use event::PeepEvent;
use std::io;
use std::os::unix::io::AsRawFd;
use std::sync::mpsc;
use std::thread::sleep;
use std::time::Duration;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

use logger;

/// Returns one of the following values as io::Result<Option<(is_hup)>>.
/// - Err() : Error
/// - Ok(None) : Timeout
/// - Ok(Some(false)) : Get event without hung up.
/// - Ok(Some(true)) : Get event with hung up.  It is necessary to quit after read.
pub trait FileWatch {
    fn watch(&mut self, timeout: Option<Duration>) -> io::Result<Option<(bool)>>;
}

const NONE_WAIT_SEC: u64 = 60;

pub struct Timeout;

impl FileWatch for Timeout {
    fn watch(&mut self, timeout: Option<Duration>) -> io::Result<Option<(bool)>> {
        let timeout = timeout.unwrap_or(Duration::from_secs(NONE_WAIT_SEC));
        sleep(timeout);
        Ok(None)
    }
}

pub fn file_watcher(file_path: &str, event_sender: &mpsc::Sender<PeepEvent>) {
    let mut fw: FileWatcher;
    let mut tm = Timeout;
    let mut sw: StdinWatcher;
    let stdin_fd = io::stdin().as_raw_fd();
    let filewatcher: &mut FileWatch = if file_path == "-" {
        if let Ok(v) = StdinWatcher::new(stdin_fd) {
            sw = v;
            logger::log("get stdin watcher");
            &mut sw
        } else {
            logger::log("get timeout1");
            &mut tm
        }
    } else if let Ok(v) = FileWatcher::new(file_path) {
        fw = v;
        logger::log("file watcher");
        &mut fw
    } else {
        logger::log("get timeout2");
        &mut tm
    };

    let default_timeout = Duration::from_secs(NONE_WAIT_SEC);

    loop {
        if filewatcher.watch(Some(default_timeout)).unwrap().is_some() {
            event_sender.send(PeepEvent::FileUpdated).unwrap();
            logger::log("file updated");
        }
    }
}
