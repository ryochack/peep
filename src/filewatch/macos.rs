use std::fs::File;
use std::io::{self, Seek, SeekFrom};
use std::time::Duration;
use std::os::unix::io::{AsRawFd, RawFd};
use mio;
use mio::unix::{EventedFd, UnixReady};
use super::*;

pub struct FileWatcher {
    file: File,
    poll: mio::Poll,
    events: mio::Events,
}

impl FileWatcher {
    pub fn new(file_path: &str) -> io::Result<Self> {
        let mut file = File::open(file_path)?;
        file.seek(SeekFrom::End(0))?;

        let poll = mio::Poll::new()?;
        let events = mio::Events::with_capacity(1024);
        poll.register(
            &EventedFd(&file.as_raw_fd()),
            mio::Token(0),
            mio::Ready::readable(),
            mio::PollOpt::edge(),
        )?;

        Ok( Self { file, poll, events } )
    }
}

impl FileWatch for FileWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<Option<bool>> {
        self.poll.poll(&mut self.events, timeout)?;
        self.file.seek(SeekFrom::End(0))?;
        Ok(
            if self.events.is_empty() {
                None
            } else {
                Some(false)
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

