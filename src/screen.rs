#![allow(unused_variables)]
#![allow(dead_code)]
/// Screen
extern crate termion;

use self::termion::terminal_size;
use escape_seq::cis;
use std::io::{self, Write};
use std::cmp;

// start position is (1, 1)
#[derive(Clone, Copy)]
struct Position {
    row: usize,
    col: usize,
}

// start point is (0, 0)
#[derive(Clone, Copy)]
struct Point {
    x: usize,
    y: usize,
}

pub enum ScreenCall {
    MoveDown(usize),
    MoveUp(usize),
    MoveLeft(usize),
    MoveRight(usize),
    MoveDownHalfPages(usize),
    MoveUpHalfPages(usize),
    MoveDownPages(usize),
    MoveUpPages(usize),
    MoveToHeadOfLine,
    MoveToEndOfLine,
    MoveToTopOfLines,
    MoveToBottomOfLines,
    MoveToLineNumber(usize),

    ShowLineNumber(bool),
    ShowNoPrinting(bool),
    HighLightWord(Option<String>),

    IncrementLines(usize),
    DecrementLines(usize),
    SetNumOfLines(usize),
    Quit,
}

pub struct Screen<'a> {
    linebuf: &'a [String],
    ostream: &'a mut Write,
    specified_numof_lines: usize,
    home_pos: Position,   // terminal home position
    specified_pt: Point,  // buffer point

    show_nonprinting: bool,
    show_line_number: bool,
    show_highlight: bool,
    highlight_word: String,
    dirty: bool,
}

impl<'a> Screen<'a> {
    pub fn new(buf: &'a [String], ostream: &'a mut Write, nlines: usize) -> Screen<'a> {
        // TODO: validate arguments or use builder
        let mut scr = Screen {
            linebuf: buf,
            ostream: ostream,
            specified_numof_lines: nlines,
            home_pos: Position { row: 1, col: 1 },
            specified_pt: Point { x: 0, y: 0 },
            show_nonprinting: false,
            show_line_number: false,
            show_highlight: false,
            highlight_word: "".to_owned(),
            dirty: true,
        };
        scr.sweep_window(nlines);
        scr.flush();
        scr.home_pos = match cis::dsr().unwrap_or((1, 1)) {
            (y, x) => Position { row: y, col: x },
        };
        cis::cnl(nlines);
        scr
    }

    fn flush(&mut self) {
        self.ostream.flush().unwrap();
    }

    fn sweep_window(&mut self, nlines: usize) {
        for _ in 0..nlines {
            cis::el(2);
            writeln!(self.ostream).unwrap();
        }
        // cis::el(2);
        cis::cpl(nlines);
    }

    fn move_to_home_position(&self) {
        cis::cup(self.home_pos.row, self.home_pos.col);
    }

    /// return (width, height)
    fn window_size(&self) -> io::Result<(usize, usize)> {
        terminal_size().map(|(w, h)| {
            (
                w as usize,
                if self.specified_numof_lines > h as usize {
                    h as usize
                } else {
                    self.specified_numof_lines
                },
            )
        })
    }

    fn line_index(&self, wrows: usize) -> (usize, usize) {
        let lbrows = self.linebuf.len();
        let y = self.specified_pt.y;
        if wrows > lbrows {
            // buflines length is less than win-rows.
            (0, lbrows)
        } else if y + wrows >= lbrows {
            // buflines length is not enough at current pos.row. scroll down to fit.
            (lbrows - wrows, lbrows)
        } else {
            (y, y + wrows)
        }
    }

    fn max_line_length(&self, (begin, end): (usize, usize)) -> usize {
        // TODO: validation
        self.linebuf[begin..end]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x))
    }

    fn fit_offset(&self, offset: usize, lnlen: usize, winwidth: usize) -> usize {
        if winwidth >= lnlen {
            0
        } else if offset + winwidth <= lnlen {
            offset
        } else {
            // offset + winwidth > lnlen
            lnlen - winwidth
        }
    }

    fn decorate(&self, raw: &str) -> String {
        // TODO: implement
        let (ww, _) = self.window_size().unwrap();
        let x = self.specified_pt.x;
        format!("{}", raw.get(x..cmp::min(raw.len(), x+ww)).unwrap_or(""))
    }

    fn refresh(&mut self) {
        if !self.dirty { return; }

        let (ww, wh) = self.window_size().unwrap();
        let (begin, end) = self.line_index(wh);

        self.move_to_home_position();
        self.sweep_window(wh);

        for (_i, ln) in self.linebuf[begin..end].iter().enumerate() {
            let dl = self.decorate(&ln);
            writeln!(self.ostream, "{}", dl);
        }

        self.flush();
        self.dirty = false;
    }

    fn scrcall_move_down(&mut self, n: usize) {
        let y = if self.specified_pt.y + n >= self.linebuf.len() {
            self.linebuf.len() - 1
        } else {
            self.specified_pt.y + n
        };
        if y == self.specified_pt.y { return; }
        self.specified_pt.y = y;
        self.dirty = true;
    }

    fn scrcall_move_up(&mut self, n: usize) {
        let y = if self.specified_pt.y <= n {
            0
        } else {
            self.specified_pt.y - n
        };
        if y == self.specified_pt.y { return; }
        self.specified_pt.y = y;
        self.dirty = true;
    }

    fn scrcall_move_left(&mut self, n: usize) {
        let x = if self.specified_pt.x <= n {
            0
        } else {
            self.specified_pt.x - n
        };
        if x == self.specified_pt.x { return; }
        self.specified_pt.x = x;
        self.dirty = true;
    }

    fn scrcall_move_right(&mut self, n: usize) {
        let (ww, wh) = self.window_size().unwrap();
        let max_lnlen = self.max_line_length(self.line_index(wh));
        let x = self.fit_offset(self.specified_pt.x + n, max_lnlen, ww);
        if x == self.specified_pt.x { return; }
        self.specified_pt.x = x;
        self.dirty = true;
    }

    fn scrcall_move_down_halfpages(&mut self, n: usize) {
        let (_, wh) = self.window_size().unwrap();
        let hpages = (wh * n) / 2;
        self.scrcall_move_down(hpages);
    }

    fn scrcall_move_up_halfpages(&mut self, n: usize) {
        let (_, wh) = self.window_size().unwrap();
        let hpages = (wh * n) / 2;
        self.scrcall_move_up(hpages);
    }

    fn scrcall_move_down_pages(&mut self, n: usize) {
        let (_, wh) = self.window_size().unwrap();
        let pages = wh * n;
        self.scrcall_move_down(pages);
    }

    fn scrcall_move_up_pages(&mut self, n: usize) {
        let (_, wh) = self.window_size().unwrap();
        let pages = wh * n;
        self.scrcall_move_up(pages);
    }

    fn scrcall_move_to_head_of_line(&mut self) {
        if self.specified_pt.x == 0 { return; }
        self.specified_pt.x = 0;
        self.dirty = true;
    }

    fn scrcall_move_to_end_of_line(&mut self) {
        let (ww, wh) = self.window_size().unwrap();
        let max_lnlen = self.max_line_length(self.line_index(wh));
        let x = self.fit_offset(max_lnlen, max_lnlen, ww);
        if x == self.specified_pt.x { return; }
        self.specified_pt.x = x;
        self.dirty = true;
    }

    fn scrcall_move_to_top_of_lines(&mut self) {
        if self.specified_pt.y == 0 { return; }
        self.specified_pt = Point { x: 0, y: 0 };
        self.dirty = true;
    }

    fn scrcall_move_to_bottom_of_lines(&mut self) {
        let y = self.linebuf.len() - 1;
        if self.specified_pt.y == y { return; }
        self.specified_pt = Point { x: 0, y: y };
        self.dirty = true;
    }

    fn scrcall_move_to_line_number(&mut self, n: usize) {
        let y = if n >= self.linebuf.len() {
            self.linebuf.len() - 1
        } else {
            n
        };
        if self.specified_pt.y == n { return; }
        self.specified_pt.y = y;
        self.dirty = true;
    }

    fn scrcall_show_line_number(&mut self, b: bool) {
        if b == self.show_line_number { return };
        self.show_line_number = b;
        self.dirty = true;
    }

    fn scrcall_show_noprinting(&mut self, b: bool) {
        unimplemented!();
    }

    fn scrcall_highlight_word(&mut self, hlword: Option<String>) {
        match hlword {
            Some(w) => {
                if w.is_empty() {
                    if !self.show_highlight { return; }
                    self.show_highlight = false;
                    self.dirty = true;
                } else {
                    if self.show_highlight && w == self.highlight_word { return; }
                    self.show_highlight = true;
                    self.highlight_word = w;
                    self.dirty = true;
                }
            }
            None => if self.show_highlight {
                self.show_highlight = false;
                self.dirty = true;
            },
        }
    }

    fn scrcall_increment_lines(&mut self, n: usize) {
        let nl = self.specified_numof_lines + n;
        self.scrcall_set_numof_lines(nl);
    }

    fn scrcall_decrement_lines(&mut self, n: usize) {
        let nl = self.specified_numof_lines - n;
        if nl <= 0 { return; }
        self.scrcall_set_numof_lines(nl);
    }

    fn scrcall_set_numof_lines(&mut self, n: usize) {
        if n == 0 || n == self.specified_numof_lines { return; }
        if n > self.specified_numof_lines {
            // TODO: requre changing home_position?
        }
        self.specified_numof_lines = n;
        self.dirty = true;
    }

    fn scrcall_quit(&mut self) {
        let (_, height) = self.window_size().unwrap();
        cis::cup(self.home_pos.row + height, self.home_pos.col);
        self.flush();
    }

    pub fn call(&mut self, scrcall: ScreenCall) {
        match scrcall {
            ScreenCall::MoveDown(n)           => self.scrcall_move_down(n),
            ScreenCall::MoveUp(n)             => self.scrcall_move_up(n),
            ScreenCall::MoveLeft(n)           => self.scrcall_move_left(n),
            ScreenCall::MoveRight(n)          => self.scrcall_move_right(n),
            ScreenCall::MoveDownHalfPages(n)  => self.scrcall_move_down_halfpages(n),
            ScreenCall::MoveUpHalfPages(n)    => self.scrcall_move_up_halfpages(n),
            ScreenCall::MoveDownPages(n)      => self.scrcall_move_down_pages(n),
            ScreenCall::MoveUpPages(n)        => self.scrcall_move_up_pages(n),
            ScreenCall::MoveToHeadOfLine      => self.scrcall_move_to_head_of_line(),
            ScreenCall::MoveToEndOfLine       => self.scrcall_move_to_end_of_line(),
            ScreenCall::MoveToTopOfLines      => self.scrcall_move_to_top_of_lines(),
            ScreenCall::MoveToBottomOfLines   => self.scrcall_move_to_bottom_of_lines(),
            ScreenCall::MoveToLineNumber(n)   => self.scrcall_move_to_line_number(n),
            ScreenCall::ShowLineNumber(b)     => self.scrcall_show_line_number(b),
            ScreenCall::ShowNoPrinting(b)     => self.scrcall_show_noprinting(b),
            ScreenCall::HighLightWord(hlword) => self.scrcall_highlight_word(hlword),
            ScreenCall::IncrementLines(n)     => self.scrcall_increment_lines(n),
            ScreenCall::DecrementLines(n)     => self.scrcall_decrement_lines(n),
            ScreenCall::SetNumOfLines(n)      => self.scrcall_set_numof_lines(n),
            ScreenCall::Quit                  => self.scrcall_quit(),
        }
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {
    }

    fn teardown() {
    }

    #[test]
    fn test_move() {
        use std::{thread, time};
        use std::io;

        let buf = [
            "[1]: aa<".to_owned(),
            "[2]: bbbb<".to_owned(),
            "[3]: cccccccc<".to_owned(),
            "[4]: dddddddddddddddd<".to_owned(),
            "[5]: eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee<".to_owned(),
            "[6]: ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff<".to_owned(),
            "[7]: gggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggggg<".to_owned(),
            "[8]: hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh<".to_owned(),
        ];
        // Bufwriter doesn't work. use Stdout or StdoutLock.
        let out = io::stdout();
        let mut outlock = out.lock();
        let nlines = 4;

        let mut scr = Screen::new(&buf, &mut outlock, nlines);
        scr.call(ScreenCall::MoveToTopOfLines);

        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveDown(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveDown(2));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveRight(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveRight(2));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveToEndOfLine);
        thread::sleep(time::Duration::from_millis(1000));
        scr.call(ScreenCall::MoveLeft(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveLeft(2));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveToHeadOfLine);
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveToBottomOfLines);
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveToTopOfLines);
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::IncrementLines(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::IncrementLines(2));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::DecrementLines(3));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::Quit);
        thread::sleep(time::Duration::from_millis(500));
    }
}
