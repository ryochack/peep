extern crate inotify;

use event::PeepEvent;
use std::sync::mpsc;
use std::thread::{spawn, sleep};
use std::io;
use std::time::Duration;
use std::os::unix::io::{AsRawFd, RawFd};
#[cfg(target_os = "linux")]
use std::fs::File;
#[cfg(target_os = "linux")]
use std::io::{Seek, SeekFrom};
#[cfg(target_os = "linux")]
use mio::unix::EventedFd;


use logger;

pub trait FileWatch {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()>;
}

#[cfg(target_os = "linux")]
use mio;

#[cfg(target_os = "linux")]
pub struct FileWatcher {
    file: File,
    poll: mio::Poll,
    events: mio::Events,
}

const NONE_WAIT_SEC: u64 = 60;

#[cfg(target_os = "linux")]
impl FileWatcher {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let mut file = File::open(file_path).unwrap();
        file.seek(SeekFrom::End(0))?;

        let poll = mio::Poll::new()?;
        let events = mio::Events::with_capacity(1024);

        poll.register(
            &EventedFd(&file.as_raw_fd()),
            mio::Token(0),
            mio::Ready::readable(),
            mio::PollOpt::edge(),
        )?;

        Ok( Self { file, poll, events, } )
    }
}

#[cfg(target_os = "linux")]
impl FileWatch for FileWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.poll.poll(&mut self.events, timeout)?;
        self.file.seek(SeekFrom::End(0))?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
pub struct StdinWatcher {
    poll: mio::Poll,
    events: mio::Events,
}

#[cfg(target_os = "linux")]
impl StdinWatcher {
    pub fn new(fd: RawFd) -> io::Result<Self> {
        let poll = mio::Poll::new()?;
        let events = mio::Events::with_capacity(1024);

        poll.register(
            &EventedFd(&fd),
            mio::Token(0),
            mio::Ready::readable(),
            mio::PollOpt::edge(),
        )?;
        Ok( Self{ poll, events, } )
    }
}

#[cfg(target_os = "linux")]
impl FileWatch for StdinWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub struct StdinWatcher;

#[cfg(target_os = "macos")]
impl StdinWatcher {
    pub fn new(_fd: RawFd) -> io::Result<Self> {
        Ok( Self{} )
    }
}

#[cfg(target_os = "macos")]
impl FileWatch for StdinWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        let timeout = timeout.unwrap_or(Duration::from_secs(NONE_WAIT_SEC));
        sleep(timeout);
        Ok(())
    }
}

#[cfg(target_os = "macos")]
type FileWatcher = Timeout;

#[cfg(target_os = "macos")]
impl FileWatcher {
    pub fn new(_file_path: &str) -> io::Result<Self> {
        Ok( Self{} )
    }
}

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

#[allow(dead_code)]
pub fn inotifier(file_path: &str, event_sender: mpsc::Sender<PeepEvent>) {
    let mut inotify = inotify::Inotify::init().expect("Failed to initialize inotify");

    if file_path == "-" {
        // from pipe
        // FIXME: how to watch stdin?
        let _ = spawn(move || loop {
            sleep(Duration::from_millis(500));
            event_sender.send(PeepEvent::FileUpdated).unwrap();
        });
    } else {
        // from file
        inotify
            .add_watch(file_path, inotify::WatchMask::MODIFY)
            .expect("Failed to add inotify watch");

        let _ = spawn(move || {
            let mut buffer = [0u8; 1024];
            loop {
                let events = inotify
                    .read_events_blocking(&mut buffer)
                    .expect("Failed to read inotify events");
                for event in events {
                    if event.mask.contains(inotify::EventMask::MODIFY) {
                        event_sender.send(PeepEvent::FileUpdated).unwrap();
                    }
                }
            }
        });
    }
}
