#![allow(dead_code)]
/// CSI(Control Sequence Introducer) of Escapse sequence
extern crate termios;

use escape_seq::{echo_off, echo_on};
use std::io::{self, Read, Write};

fn _flush(w: &mut Write) {
    w.flush().unwrap();
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
    write!(w, "{}", format!("\x1b[{}A", _nz(n)));
}

/// CUD: cursor down
pub fn cud(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}B", _nz(n)));
}

/// CUF: cursor forward
pub fn cuf(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}C", _nz(n)));
}

/// CUB: cursor back
pub fn cub(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}D", _nz(n)));
}

/// CNL: cursor next line
pub fn cnl(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}E", _nz(n)));
}

/// CPL: cursor previous line
pub fn cpl(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}F", _nz(n)));
}

/// CHA: cursor horizontal absolute
pub fn cha(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}G", _nz(n)));
}

/// CUP: cursor position
pub fn cup(w: &mut Write, row: u32, col: u32) {
    write!(w, "{}", format!("\x1b[{};{}H", _nz(row), _nz(col)));
}

pub enum EdClear {
    FromCurToEos = 0,
    FromCurToBos = 1,
    EntireScreen = 2,
    EntireScreenAndDeleteAllScrollBuffer = 3,
}
/// ED: erase in display
/// If n is 0 (or missing), clear from cursor to end of screen.
/// If n is 1, clear from cursor to beginning of the screen.
/// If n {\displaystyle n} n is 2, clear entire screen (and moves cursor to upper left on DOS ANSI.SYS).
/// If n {\displaystyle n} n is 3, clear entire screen and delete all lines saved in the scrollback buffer (this feature was added for xterm and is supported by other terminal applications).
pub fn ed(w: &mut Write, n: u32) {
    if n <= 3 {
        write!(w, "{}", format!("\x1b[{}J", n));
    }
}

pub enum ElClear {
    FromCurToEol = 0,
    FromCurToBol = 1,
    EntireLine = 2,
}
/// EL: erase in line
/// If n is 0 (or missing), clear from cursor to the end of the line.
/// If n is 1, clear from cursor to beginning of the line.
/// If n is 2, clear entire line. Cursor position does not change.
pub fn el(w: &mut Write, n: u32) {
    if n <= 2 {
        write!(w, "{}", format!("\x1b[{}K", n));
    }
}

/// SU: scroll up
pub fn su(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}S", _nz(n)));
}

/// SD: scroll down
pub fn sd(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}T", _nz(n)));
}

pub enum SgrCode {
    Normal = 0,
    Bold = 1,
    Faint = 2,
    Italic = 3,
    Underline = 4,
    SlowBlink = 5,
    RapidBlink = 6,
    Inverse = 7,
    Invisible = 8,
    Strikethrough = 9,
    PrimaryFont = 10,
    AltFont1 = 11,
    AltFont2 = 12,
    AltFont3 = 13,
    AltFont4 = 14,
    AltFont5 = 15,
    AltFont6 = 16,
    AltFont7 = 17,
    AltFont8 = 18,
    AltFont9 = 19,
    DoubleUnderline = 21,
    BoldFaintOff = 22,
    ItalicOff = 23,
    UnderlineOff = 24,
    Steady = 25,   // not blinking
    Positive = 27, // not inverse
    Visible = 28,
    StrikethroughOff = 29,
    FgColorBlack = 30,
    FgColorRed = 31,
    FgColorGreen = 32,
    FgColorYellow = 33,
    FgColorBlue = 34,
    FgColorMagenta = 35,
    FgColorCyan = 36,
    FgColorWhite = 37,

    // FgColor8bit(u8),
    // FgColor24bit((u8, u8, u8)),
    FgColorDefault = 39,
    BgColorBlack = 40,
    BgColorRed = 41,
    BgColorGreen = 42,
    BgColorYellow = 43,
    BgColorBlue = 44,
    BgColorMagenta = 45,
    BgColorCyan = 46,
    BgColorWhite = 47,
    // BgColor8bit(u8),
    // BgColor24bit((u8, u8, u8)),
    BgColorDefault = 49,
    Frame = 51,
    Encircle = 52,
    Overline = 53,
    FrameEncircleOff = 54,
    OverlineOff = 55,
    RightSideLine = 60,
    RightSideDoublLine = 61,
    LeftSideLine = 62,
    LeftSideDoublLine = 63,
    DoubleStrikethrough = 64,
    LineOff = 65,
    FgColorBrightBlack = 90,
    FgColorBrightRed = 91,
    FgColorBrightGreen = 92,
    FgColorBrightYellow = 93,
    FgColorBrightBlue = 94,
    FgColorBrightMagenta = 95,
    FgColorBrightCyan = 96,
    FgColorBrightWhite = 97,
    BgColorBrightBlack = 100,
    BgColorBrightRed = 101,
    BgColorBrightGreen = 102,
    BgColorBrightYellow = 103,
    BgColorBrightBlue = 104,
    BgColorBrightMagenta = 105,
    BgColorBrightCyan = 106,
    BgColorBrightWhite = 107,
}
/// SGR: select graphic rendition
/// SGR parameters: https://en.wikipedia.org/wiki/ANSI_escape_code#SGR_(Select_Graphic_Rendition)_parameters
pub fn sgr(w: &mut Write, c: SgrCode) {
    write!(w, "\x1b[{}m", (c as i32));
}

pub enum SgrColor {
    FgColor8bit(u8),
    FgColor24bit((u8, u8, u8)),
    BgColor8bit(u8),
    BgColor24bit((u8, u8, u8)),
}
pub fn sgr_color(w: &mut Write, c: SgrColor) {
    write!(w, "\x1b[{}m",
        match c {
            SgrColor::FgColor8bit(color) => format!("38;5;{}", color),
            SgrColor::FgColor24bit((r, g, b)) => format!("38;2;{};{};{}", r, g, b),
            SgrColor::BgColor8bit(color) => format!("48;5;{}", color),
            SgrColor::BgColor24bit((r, g, b)) => format!("48;2;{};{};{}", r, g, b),
        }
    );
}

/// DSR: device status report
/// return (row, col)
pub fn dsr(w: &mut Write) -> Option<(u32, u32)> {
    let oldstat: Box<termios::Termios> = Box::new(echo_off());
    write!(w, "{}", format!("\x1b[6n"));
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
    write!(w, "{}", format!("\x1b[s"));
}

/// RCP: restore cursor position
pub fn rcp(w: &mut Write) {
    write!(w, "{}", format!("\x1b[u"));
}

/// SM: set mode
/// mode: http://ttssh2.osdn.jp/manual/ja/about/ctrlseq.html#mode
pub fn sm(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}h", n));
}

/// RM: reset mode
pub fn rm(w: &mut Write, n: u32) {
    write!(w, "{}", format!("\x1b[{}l", n));
}

pub enum DecscusrStyle {
    BlinkingBlock = 1,
    SteadyBlock = 2,
    BlinkingUnderline = 3,
    SteadyUnderline = 4,
    BlinkingBar = 5,
    SteadyBar = 6,
}
/// DECSCUSR: set cursor style
/// 0,1: blinking block
/// 2: steady block
/// 3: blinking underline
/// 4: steady underline
/// 5: blinking bar
/// 6: steady bar
pub fn decscusr(w: &mut Write, n: u32) {
    if n <= 6 {
        write!(w, "{}", format!("\x1b[{} q", n));
    }
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

    #[test]
    fn test_su() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        su(w, 1);
        assert_eq!(dsr(w), Some((3, 5)));
        teardown(w);
    }

    #[test]
    fn test_sd() {
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        cup(w, 3, 5);
        sd(w, 1);
        assert_eq!(dsr(w), Some((3, 5)));
        teardown(w);
    }

    #[test]
    fn test_sgr() {
        let mut w = io::stdout();
        let w = &mut w;
        // setup(w);

        sgr(w, SgrCode::Normal); // reset
        write!(w, "0");

        sgr(w, SgrCode::Bold); // bold on
        write!(w, "1");
        sgr(w, SgrCode::BoldFaintOff); // bold off

        sgr(w, SgrCode::Faint); // faint on
        write!(w, "2");
        sgr(w, SgrCode::BoldFaintOff); // faint off

        sgr(w, SgrCode::Italic); // italic on
        write!(w, "3");
        sgr(w, SgrCode::ItalicOff); // italic off

        sgr(w, SgrCode::Underline); // underline on
        write!(w, "4");
        sgr(w, SgrCode::UnderlineOff); // underline off

        sgr(w, SgrCode::SlowBlink); // blink on
        write!(w, "5");
        sgr(w, SgrCode::Steady); // blink off

        sgr(w, SgrCode::RapidBlink); // blink on
        write!(w, "6");
        sgr(w, SgrCode::Steady); // blink off

        sgr(w, SgrCode::Inverse); // inverse on
        write!(w, "7");
        sgr(w, SgrCode::Positive); // inverse off

        sgr(w, SgrCode::Strikethrough); // strikethrough on
        write!(w, "9");
        sgr(w, SgrCode::StrikethroughOff); // strikethrough off

        sgr(w, SgrCode::FgColorBlack); // fg: black
        sgr(w, SgrCode::BgColorRed); // bg: red
        write!(w, "30");
        sgr(w, SgrCode::BgColorDefault); // bg: default
        sgr(w, SgrCode::FgColorRed); // fg: red
        write!(w, "31");
        sgr(w, SgrCode::FgColorGreen); // fg: green
        write!(w, "32");
        sgr(w, SgrCode::FgColorYellow); // fg: yellow
        write!(w, "33");
        sgr(w, SgrCode::FgColorBlue); // fg: blue
        write!(w, "34");
        sgr(w, SgrCode::FgColorMagenta); // fg: magenta
        write!(w, "35");
        sgr(w, SgrCode::FgColorCyan); // fg: cyan
        write!(w, "36");
        sgr(w, SgrCode::FgColorWhite); // fg: white
        write!(w, "37");
        sgr(w, SgrCode::FgColorDefault); // fg: default

        sgr(w, SgrCode::BgColorBlack); // bg: black
        write!(w, "40");
        sgr(w, SgrCode::BgColorRed); // bg: red
        write!(w, "41");
        sgr(w, SgrCode::BgColorGreen); // bg: green
        write!(w, "42");
        sgr(w, SgrCode::BgColorYellow); // bg: yellow
        write!(w, "43");
        sgr(w, SgrCode::BgColorBlue); // bg: blue
        write!(w, "44");
        sgr(w, SgrCode::BgColorMagenta); // bg: magenta
        write!(w, "45");
        sgr(w, SgrCode::BgColorCyan); // bg: cyan
        write!(w, "46");
        sgr(w, SgrCode::FgColorRed); // fg: red
        sgr(w, SgrCode::BgColorWhite); // bg: white
        write!(w, "47");
        sgr(w, SgrCode::FgColorWhite); // fg: reset
        sgr(w, SgrCode::BgColorWhite); // bg: default

        sgr(w, SgrCode::Bold); // bold on
        sgr(w, SgrCode::Underline); // underline on
        sgr(w, SgrCode::SlowBlink); // blink on
        sgr(w, SgrCode::Normal); // reset
        write!(w, "x");

        writeln!(w);
        _flush(w);

        // teardown(w);
    }

    #[test]
    fn test_sgr() {
        let mut w = io::stdout();
        let w = &mut w;

        sgr(w, SgrCode::Normal); // reset
        write!(w, "0");

        sgr_color(w, SgrColor::FgColor8bit(0));
        write!(w, "A");
        sgr_color(w, SgrColor::FgColor8bit(1));
        write!(w, "B");
        sgr_color(w, SgrColor::FgColor8bit(0));
        write!(w, "A");
        sgr_color(w, SgrColor::FgColor8bit(1));
        write!(w, "B");
    }

    #[test]
    fn test_decscusr() {
        use std::{thread, time};
        let mut w = io::stdout();
        let w = &mut w;
        setup(w);
        decscusr(w, 1);
        _flush(w);
        thread::sleep(time::Duration::from_millis(1000));
        decscusr(w, 2);
        _flush(w);
        thread::sleep(time::Duration::from_millis(1000));
        decscusr(w, 3);
        _flush(w);
        thread::sleep(time::Duration::from_millis(1000));
        decscusr(w, 4);
        _flush(w);
        thread::sleep(time::Duration::from_millis(1000));
        decscusr(w, 5);
        _flush(w);
        thread::sleep(time::Duration::from_millis(1000));
        decscusr(w, 6);
        _flush(w);
        thread::sleep(time::Duration::from_millis(1000));
        decscusr(w, 2);
        _flush(w);
        teardown(w);
    }
}
