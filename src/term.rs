use libc;
use std::io::Stdin;
use std::os::unix::io::RawFd;
use termios;

// re-export
pub use termios::{
    // c_iflag
    BRKINT,
    ICRNL,
    IGNBRK,
    IGNCR,
    IGNPAR,
    INLCR,
    INPCK,
    ISTRIP,
    IXANY,
    IXOFF,
    IXON,
    PARMRK,
};
pub use termios::{
    // c_cflag
    CLOCAL,
    CREAD,
    CS5,
    CS6,
    CS7,
    CS8,
    CSIZE,
    CSTOPB,
    HUPCL,
    PARENB,
    PARODD,
};
pub use termios::{
    // c_lflag
    ECHO,
    ECHOE,
    ECHOK,
    ECHONL,
    ICANON,
    IEXTEN,
    ISIG,
    NOFLSH,
    TOSTOP,
};
pub use termios::{
    // c_oflag
    OCRNL,
    ONLCR,
    ONLRET,
    ONOCR,
    OPOST,
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
        let stat = termios::Termios::from_fd(fd).unwrap_or_else(|_| panic!("invalid fd {:?}", fd));
        Self {
            fd,
            default: stat,
            custom: stat,
        }
    }

    pub fn iflag(
        &mut self,
        set_flags: termios::tcflag_t,
        clear_flags: termios::tcflag_t,
    ) -> &mut Self {
        self.custom.c_iflag |= set_flags;
        self.custom.c_iflag &= !clear_flags;
        self
    }

    pub fn oflag(
        &mut self,
        set_flags: termios::tcflag_t,
        clear_flags: termios::tcflag_t,
    ) -> &mut Self {
        self.custom.c_oflag |= set_flags;
        self.custom.c_oflag &= !clear_flags;
        self
    }

    pub fn cflag(
        &mut self,
        set_flags: termios::tcflag_t,
        clear_flags: termios::tcflag_t,
    ) -> &mut Self {
        self.custom.c_cflag |= set_flags;
        self.custom.c_cflag &= !clear_flags;
        self
    }

    pub fn lflag(
        &mut self,
        set_flags: termios::tcflag_t,
        clear_flags: termios::tcflag_t,
    ) -> &mut Self {
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
