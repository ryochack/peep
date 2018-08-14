#![allow(dead_code)]
/// CSI(Control Sequence Introducer) of Escapse sequence
extern crate termios;

use escape_seq::{echo_off, echo_on};
use std::io::{self, Read, Write};

/// CUU: cursor up
pub fn cuu(n: usize) {
    write!(io::stdout(), "\x1b[{}A", n).unwrap();
}
/// CUD: cursor down
pub fn cud(n: usize) {
    write!(io::stdout(), "\x1b[{}B", n).unwrap();
}
/// CUF: cursor forward
pub fn cuf(n: usize) {
    write!(io::stdout(), "\x1b[{}C", n).unwrap();
}
/// CUB: cursor back
pub fn cub(n: usize) {
    write!(io::stdout(), "\x1b[{}D", n).unwrap();
}
/// CNL: cursor next line
pub fn cnl(n: usize) {
    write!(io::stdout(), "\x1b[{}E", n).unwrap();
}
/// CPL: cursor previous line
pub fn cpl(n: usize) {
    write!(io::stdout(), "\x1b[{}F", n).unwrap();
}
/// CHA: cursor horizontal absolute
pub fn cha(n: usize) {
    write!(io::stdout(), "\x1b[{}G", n).unwrap();
}

/// CUP: cursor position
pub fn cup(row: usize, col: usize) {
    write!(io::stdout(), "\x1b[{};{}H", row, col).unwrap();
}
/// ED: erase in display
/// If n is 0 (or missing), clear from cursor to end of screen.
/// If n is 1, clear from cursor to beginning of the screen.
/// If n {\displaystyle n} n is 2, clear entire screen (and moves cursor to upper left on DOS ANSI.SYS).
/// If n {\displaystyle n} n is 3, clear entire screen and delete all lines saved in the scrollback buffer (this feature was added for xterm and is supported by other terminal applications).
pub fn ed(n: usize) {
    if n <= 3 {
        write!(io::stdout(), "\x1b[{}J", n).unwrap();
    }
}
/// EL: erase in line
/// If n is 0 (or missing), clear from cursor to the end of the line.
/// If n is 1, clear from cursor to beginning of the line.
/// If n is 2, clear entire line. Cursor position does not change.
pub fn el(n: usize) {
    if n <= 2 {
        write!(io::stdout(), "\x1b[{}K", n).unwrap();
    }
}
/// SU: scroll up
pub fn su(n: usize) {
    write!(io::stdout(), "\x1b[{}S", n).unwrap();
}
/// SD: scroll down
pub fn sd(n: usize) {
    write!(io::stdout(), "\x1b[{}T", n).unwrap();
}
/// HVP: horizontal vertical position (same as CUP)
pub fn hvp(row: usize, col: usize) {
    write!(io::stdout(), "\x1b[{};{}f", row, col).unwrap();
}
/// SGR: select graphic rendition
/// SGR parameters: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters
pub fn sgr(n: usize) {
    if n <= 107 {
        write!(io::stdout(), "\x1b[{}m", n).unwrap();
    }
}
/// DSR: device status report
/// return (row, col)
pub fn dsr() -> Option<(usize, usize)> {
    let oldstat: Box<termios::Termios> = Box::new(echo_off());
    write!(io::stdout(), "\x1b[6n").unwrap();
    io::stdout().flush().unwrap();
    let (mut row, mut col, mut tmp) = (0usize, 0usize, 0usize);
    let s = io::stdin();
    // => "[${row};${col}R"
    for b in s.lock().bytes().filter_map(|v| v.ok()) {
        match b {
            // '0' ... '9'
            0x30...0x39 => {
                tmp = tmp * 10 + (b - 0x30) as usize;
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
pub fn scp() {
    write!(io::stdout(), "\x1b[s").unwrap();
}
/// RCP: restore cursor position
pub fn rcp() {
    write!(io::stdout(), "\x1b[u").unwrap();
}
/// SM: set mode
/// mode: http://ttssh2.osdn.jp/manual/ja/about/ctrlseq.html#mode
pub fn sm(n: usize) {
    write!(io::stdout(), "\x1b[{}h", n).unwrap();
}
/// RM: reset mode
pub fn rm(n: usize) {
    write!(io::stdout(), "\x1b[{}l", n).unwrap();
}

