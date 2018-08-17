extern crate termios;

pub mod csi;

/// echo off. Return old termios state.
pub fn echo_off() -> termios::Termios {
    let oldstat = termios::Termios::from_fd(0).unwrap();
    let mut termstat = oldstat;
    termstat.c_lflag &= !(termios::ICANON | termios::ECHO);
    termios::tcsetattr(0, termios::TCSANOW, &mut termstat).unwrap();
    oldstat
}

/// echo on. Pass old termios state.
pub fn echo_on(termstat: &termios::Termios) {
    termios::tcsetattr(0, termios::TCSANOW, &termstat).unwrap();
}
