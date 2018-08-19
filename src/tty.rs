use termios;

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

/// switch stdin to tty
/// after leave function, stdin is set as tty.
pub fn force_set_to_stdin() {
    use std::fs::File;
    use nix::unistd;
    use std::io;
    use std::os::unix::io::AsRawFd;
    let file = File::open("/dev/tty").unwrap();
    let _ = unistd::dup2(file.as_raw_fd(), io::stdin().as_raw_fd());
}

