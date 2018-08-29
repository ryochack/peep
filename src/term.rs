use termios;
use std::os::unix::io::RawFd;
use libc;
use std::io::Stdin;

// re-export
pub use termios::{
    // c_iflag
    IGNBRK, BRKINT, IGNPAR, PARMRK, INPCK, ISTRIP,
    INLCR, IGNCR, ICRNL, IXON, IXANY, IXOFF,
    // c_oflag
    OPOST, ONLCR, OCRNL, ONOCR, ONLRET,
    // c_cflag
    CSIZE, CS5, CS6, CS7, CS8,
    CSTOPB, CREAD, PARENB, PARODD,
    HUPCL, CLOCAL,
    // c_lflag
    ISIG, ICANON, ECHO, ECHOE, ECHOK, ECHONL,
    IEXTEN, NOFLSH, TOSTOP,
};

pub struct TermAttrSetter {
    fd: RawFd,
    default: termios::Termios,
    custom: termios::Termios,
}

pub struct TermAttrRestorer {
    default: termios::Termios,
}

pub enum CcSymbol {
    VEOF = termios::VEOF as isize,
    VEOL = termios::VEOL as isize,
    VERASE = termios::VERASE as isize,
    VINTR = termios::VINTR as isize,
    VKILL = termios::VKILL as isize,
    VMIN = termios::VMIN as isize,
    VQUIT = termios::VQUIT as isize,
    VSTART = termios::VSTART as isize,
    VSTOP = termios::VSTOP as isize,
    VSUSP = termios::VSUSP as isize,
    VTIME = termios::VTIME as isize,
}

impl TermAttrSetter {
    pub fn new(fd: RawFd) -> TermAttrSetter {
        let stat = termios::Termios::from_fd(fd).expect(&format!("invalid fd {:?}", fd));
        Self {
            fd: fd,
            default: stat,
            custom: stat,
        }
    }

    pub fn iflag(&mut self, set_flags: termios::tcflag_t, clear_flags: termios::tcflag_t) -> &mut Self {
        self.custom.c_iflag |= set_flags;
        self.custom.c_iflag &= !clear_flags;
        self
    }

    pub fn oflag(&mut self, set_flags: termios::tcflag_t, clear_flags: termios::tcflag_t) -> &mut Self {
        self.custom.c_oflag |= set_flags;
        self.custom.c_oflag &= !clear_flags;
        self
    }

    pub fn cflag(&mut self, set_flags: termios::tcflag_t, clear_flags: termios::tcflag_t) -> &mut Self {
        self.custom.c_cflag |= set_flags;
        self.custom.c_cflag &= !clear_flags;
        self
    }

    pub fn lflag(&mut self, set_flags: termios::tcflag_t, clear_flags: termios::tcflag_t) -> &mut Self {
        self.custom.c_lflag |= set_flags;
        self.custom.c_lflag &= !clear_flags;
        self
    }

    pub fn cc(&mut self, sym: CcSymbol, value: u8) -> &mut Self {
        self.custom.c_cc[sym as usize] = value;
        self
    }

    pub fn set(&self) -> TermAttrRestorer {
        termios::tcsetattr(self.fd, termios::TCSANOW, &self.custom).unwrap();

        TermAttrRestorer {
            default: self.default,
        }
    }
}

impl TermAttrRestorer {
    pub fn restore(&self, fd: RawFd) {
        termios::tcsetattr(fd, termios::TCSANOW, &self.default).unwrap();
    }
}

pub trait Block {
    fn nonblocking(&self);
    fn blocking(&self);
}

impl Block for Stdin {
    fn nonblocking(&self) {
        unsafe {
            let mut nonblocking = 1 as libc::c_ulong;
            libc::ioctl(0, libc::FIONBIO, &mut nonblocking);
        }
    }

    fn blocking(&self) {
        unsafe {
            let mut nonblocking = 0 as libc::c_ulong;
            libc::ioctl(0, libc::FIONBIO, &mut nonblocking);
        }
    }
}
