use super::*;
use mio;
use std::io;
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Duration;

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

        poll.registry().register(
            &mut mio::unix::SourceFd(&inotify.as_raw_fd()),
            mio::Token(0),
            mio::Interest::READABLE
        )?;

        Ok(Self {
            inotify,
            poll,
            events,
            buffer: [0u8; 1024],
        })
    }
}

impl FileWatch for FileWatcher {
    fn watch(&mut self, timeout: Option<Duration>) -> io::Result<Option<bool>> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(if self.events.is_empty() {
            None
        } else {
            let evt = &self.events.iter().next();
            self.inotify.read_events(&mut self.buffer)?;
            if let Some(e) = evt {
                Some(e.is_readable())
            } else {
                None
            }
        })
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
        poll.registry().register(
            &mut mio::unix::SourceFd(&fd),
            mio::Token(0),
            mio::Interest::READABLE
        )?;

        Ok(Self { poll, events })
    }
}

impl FileWatch for StdinWatcher {
    fn watch(&mut self, timeout: Option<Duration>) -> io::Result<Option<bool>> {
        self.poll.poll(&mut self.events, timeout)?;
        Ok(if self.events.is_empty() {
            None
        } else {
            let evt = &self.events.iter().next();
            if let Some(e) = evt {
                Some(e.is_readable())
            } else {
                None
            }
        })
    }
}
