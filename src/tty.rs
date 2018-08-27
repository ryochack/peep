use termion;
use termios;
use std::io;
use nix::unistd;
use std::fs::File;
use std::os::unix::io::AsRawFd;

/// echo off. Return old termios state.
pub fn echo_off() -> Option<termios::Termios> {
    let file;
    let fd = if !termion::is_tty(&io::stdin()) {
        // from pipe
        file = File::open("/dev/tty").unwrap();
        file.as_raw_fd()
    } else {
        // from file
        0
    };
    let oldstat = termios::Termios::from_fd(fd).unwrap();
    let mut termstat = oldstat;
    termstat.c_lflag &= !(termios::ICANON | termios::ECHO);
    termios::tcsetattr(fd, termios::TCSANOW, &mut termstat).unwrap();
    Some(oldstat)
}

/// echo on. Pass old termios state.
pub fn echo_on(termstat: &Option<termios::Termios>) {
    if let Some(stat) = termstat {
        let file;
        let fd = if !termion::is_tty(&io::stdin()) {
            // from pipe
            file = File::open("/dev/tty").unwrap();
            file.as_raw_fd()
        } else {
            // from file
            0
        };
        termios::tcsetattr(fd, termios::TCSANOW, &stat).unwrap();
    }
}

#[allow(dead_code)]
/// switch stdin to tty
/// after leave function, stdin is set as tty.
pub fn switch_stdin_to_tty() {
    let file = File::open("/dev/tty").unwrap();
    let _ = unistd::dup2(file.as_raw_fd(), io::stdin().as_raw_fd());
}

