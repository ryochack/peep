use std::io;
use std::time::Duration;
use std::os::unix::io::{AsRawFd, RawFd};
use mio;
use mio::unix::{EventedFd, UnixReady};
use super::*;
use inotify;

pub struct FileWatcher {
    inotify: inotify::Inotify,
    poll: mio::Poll,
    events: mio::Events,
    buffer: [u8; 1024],
}

impl FileWatcher {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let mut inotify = inotify::Inotify::init()?;
        inotify.add_watch(file_path, inotify::WatchMask::MODIFY)?;
        let poll = mio::Poll::new()?;
        let events = mio::Events::with_capacity(1024);

        poll.register(
            &EventedFd(&inotify.as_raw_fd()),
            mio::Token(0),
            mio::Ready::readable(),
            mio::PollOpt::edge(),
        )?;

        Ok( Self { inotify, poll, events, buffer: [0u8; 1024] } )
    }
}

impl FileWatch for FileWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<Option<bool>> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(
            if self.events.is_empty() {
                None
            } else {
                let evt = &self.events.iter().next();
                self.inotify.read_events(&mut self.buffer)?;
                if let Some(e) = evt {
                    Some(UnixReady::from(e.readiness()).is_hup())
                } else {
                    None
                }
            }
        )
    }
}

pub struct StdinWatcher {
    poll: mio::Poll,
    events: mio::Events,
}

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

        Ok( Self{ poll, events } )
    }
}

impl FileWatch for StdinWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<Option<bool>> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(
            if self.events.is_empty() {
                None
            } else {
                let evt = &self.events.iter().next();
                if let Some(e) = evt {
                    Some(UnixReady::from(e.readiness()).is_hup())
                } else {
                    None
                }
            }
        )
    }
}

