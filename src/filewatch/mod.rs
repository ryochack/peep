use event::PeepEvent;
use std::sync::mpsc;
use std::thread::sleep;
use std::io;
use std::time::Duration;
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "linux")]
pub use self::linux::*;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use self::macos::*;

use logger;

pub trait FileWatch {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()>;
}

const NONE_WAIT_SEC: u64 = 60;

pub struct Timeout;

impl FileWatch for Timeout {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        let timeout = timeout.unwrap_or(Duration::from_secs(NONE_WAIT_SEC));
        sleep(timeout);
        Ok(())
    }
}

pub fn file_watcher(file_path: &str, event_sender: mpsc::Sender<PeepEvent>) {
    let mut fw: FileWatcher;
    let mut tm = Timeout;
    let mut sw: StdinWatcher;
    let stdin_fd = io::stdin().as_raw_fd();
    let filewatcher: &mut FileWatch = if file_path == "-" {
        if let Ok(v) = StdinWatcher::new(stdin_fd) {
            sw = v;
            &mut sw
        } else {
            &mut tm
        }
    } else {
        if let Ok(v) = FileWatcher::new(file_path) {
            fw = v;
            &mut fw
        } else {
            &mut tm
        }
    };

    let default_timeout = Duration::from_millis(500);

    loop {
        filewatcher.block(Some(default_timeout)).unwrap();
        event_sender.send(PeepEvent::FileUpdated).unwrap();
        logger::log("file updated");
    }
}

