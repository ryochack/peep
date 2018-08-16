#![allow(dead_code)]
/// CSI(Control Sequence Introducer) of Escapse sequence
extern crate termios;

use escape_seq::{echo_off, echo_on};
use std::io::{self, Read, Write};

fn _flush(w: &mut Write) {
    w.flush().unwrap();
}

fn _write(w: &mut Write, s: &str) {
    write!(w, "{}", s).unwrap();
}

// avoid zero
fn _nz(n: u32) -> u32 {
    if n == 0 {
        1
    } else {
        n
    }
}

/// CUU: cursor up
pub fn cuu(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}A", _nz(n)));
}
/// CUD: cursor down
pub fn cud(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}B", _nz(n)));
}
/// CUF: cursor forward
pub fn cuf(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}C", _nz(n)));
}
/// CUB: cursor back
pub fn cub(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}D", _nz(n)));
}
/// CNL: cursor next line
pub fn cnl(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}E", _nz(n)));
}
/// CPL: cursor previous line
pub fn cpl(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}F", _nz(n)));
}
/// CHA: cursor horizontal absolute
pub fn cha(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}G", _nz(n)));
}

/// CUP: cursor position
pub fn cup(w: &mut Write, row: u32, col: u32) {
    _write(w, &format!("\x1b[{};{}H", _nz(row), _nz(col)));
}
/// ED: erase in display
/// If n is 0 (or missing), clear from cursor to end of screen.
/// If n is 1, clear from cursor to beginning of the screen.
/// If n {\displaystyle n} n is 2, clear entire screen (and moves cursor to upper left on DOS ANSI.SYS).
/// If n {\displaystyle n} n is 3, clear entire screen and delete all lines saved in the scrollback buffer (this feature was added for xterm and is supported by other terminal applications).
pub fn ed(w: &mut Write, n: u32) {
    if n <= 3 {
        _write(w, &format!("\x1b[{}J", n));
    }
}
/// EL: erase in line
/// If n is 0 (or missing), clear from cursor to the end of the line.
/// If n is 1, clear from cursor to beginning of the line.
/// If n is 2, clear entire line. Cursor position does not change.
pub fn el(w: &mut Write, n: u32) {
    if n <= 2 {
        _write(w, &format!("\x1b[{}K", n));
    }
}
/// SU: scroll up
pub fn su(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}S", _nz(n)));
}
/// SD: scroll down
pub fn sd(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}T", _nz(n)));
}
/// SGR: select graphic rendition
/// SGR parameters: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters
pub fn sgr(w: &mut Write, n: u32) {
    if n <= 107 {
        _write(w, &format!("\x1b[{}m", n));
    }
}
/// DSR: device status report
/// return (row, col)
pub fn dsr(w: &mut Write) -> Option<(u32, u32)> {
    let oldstat: Box<termios::Termios> = Box::new(echo_off());
    _write(w, &format!("\x1b[6n"));
    _flush(w);
    let (mut row, mut col, mut tmp) = (0u32, 0u32, 0u32);
    let s = io::stdin();
    // => "[${row};${col}R"
    for b in s.lock().bytes().filter_map(|v| v.ok()) {
        match b {
            // '0' ... '9'
            0x30...0x39 => {
                tmp = tmp * 10 + (b - 0x30) as u32;
            }
            // ';'
            0x3b => {
                row = tmp;
                tmp = 0;
            }
            // 'R'
            0x52 => {
                col = tmp;
                break;
            }
            _ => {}
        }
    }
    echo_on(&*oldstat);
    Some((row, col))
}
/// SCP: save cursor position
pub fn scp(w: &mut Write) {
    _write(w, &format!("\x1b[s"));
}
/// RCP: restore cursor position
pub fn rcp(w: &mut Write) {
    _write(w, &format!("\x1b[u"));
}
/// SM: set mode
/// mode: http://ttssh2.osdn.jp/manual/ja/about/ctrlseq.html#mode
pub fn sm(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}h", n));
}
/// RM: reset mode
pub fn rm(w: &mut Write, n: u32) {
    _write(w, &format!("\x1b[{}l", n));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{self, Write};

    fn setup(w: &mut Write) {
        scp(w);
    }

    fn teardown(w: &mut Write) {
        rcp(w);
    }

    #[test]
    fn test_cup() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        // terminal cordinate start from (1,1)
        cup(w, 0, 0);
        assert_eq!(dsr(w), Some((1, 1)));
        // cup(1, 1) -> (1, 1)
        cup(w, 1, 1);
        assert_eq!(dsr(w), Some((1, 1)));
        // cup(3, 5) -> (3, 5)
        cup(w, 3, 5);
        assert_eq!(dsr(w), Some((3, 5)));
        teardown(w);
    }

    #[test]
    fn test_cuu() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 5, 3);
        cuu(w, 1);
        assert_eq!(dsr(w), Some((4, 3)));
        cuu(w, 2);
        assert_eq!(dsr(w), Some((2, 3)));
        teardown(w);
    }

    #[test]
    fn test_cud() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 1, 5);
        cud(w, 1);
        assert_eq!(dsr(w), Some((2, 5)));
        cud(w, 2);
        assert_eq!(dsr(w), Some((4, 5)));
        teardown(w);
    }

    #[test]
    fn test_cuf() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 1);
        cuf(w, 1);
        assert_eq!(dsr(w), Some((3, 2)));
        cuf(w, 2);
        assert_eq!(dsr(w), Some((3, 4)));
        teardown(w);
    }

    #[test]
    fn test_cub() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        cub(w, 1);
        assert_eq!(dsr(w), Some((3, 4)));
        cub(w, 2);
        assert_eq!(dsr(w), Some((3, 2)));
        teardown(w);
    }

    #[test]
    fn test_cnl() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        cnl(w, 1);
        assert_eq!(dsr(w), Some((4, 1)));
        cup(w, 3, 5);
        cnl(w, 2);
        assert_eq!(dsr(w), Some((5, 1)));
        teardown(w);
    }

    #[test]
    fn test_cpl() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        cpl(w, 1);
        assert_eq!(dsr(w), Some((2, 1)));
        cup(w, 3, 5);
        cpl(w, 2);
        assert_eq!(dsr(w), Some((1, 1)));
        teardown(w);
    }

    #[test]
    fn test_cha() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        cha(w, 1);
        assert_eq!(dsr(w), Some((3, 1)));
        cha(w, 7);
        assert_eq!(dsr(w), Some((3, 7)));
        teardown(w);
    }

    // #[test]
    fn test_su() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        su(w, 1);
        assert_eq!(dsr(w), Some((3, 5)));
        teardown(w);
    }

    // #[test]
    fn test_sd() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        sd(w, 1);
        assert_eq!(dsr(w), Some((3, 5)));
        teardown(w);
    }
}
