use std::time::Duration;
use std::thread::sleep;
use std::io;
use std::os::unix::io::RawFd;
use super::*;

pub struct StdinWatcher;

impl StdinWatcher {
    pub fn new(_fd: RawFd) -> io::Result<Self> {
        Ok( Self{} )
    }
}

impl FileWatch for StdinWatcher {
    fn block(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        let timeout = timeout.unwrap_or(Duration::from_secs(NONE_WAIT_SEC));
        sleep(timeout);
        Ok(())
    }
}

type FileWatcher = Timeout;

impl FileWatcher {
    pub fn new(_file_path: &str) -> io::Result<Self> {
        Ok( Self{} )
    }
}

