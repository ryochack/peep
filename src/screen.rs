#![allow(dead_code)]
/// Screen
extern crate termion;

use self::termion::terminal_size;
use escape_seq::cis;
use std::cmp;
use std::io::{self, Write};

// start point is (0, 0)
#[derive(Clone, Copy)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Debug)]
pub enum ScreenCall<'a> {
    MoveDown(u32),
    MoveUp(u32),
    MoveLeft(u32),
    MoveRight(u32),
    MoveDownHalfPages(u32),
    MoveUpHalfPages(u32),
    MoveDownPages(u32),
    MoveUpPages(u32),
    MoveToHeadOfLine,
    MoveToEndOfLine,
    MoveToTopOfLines,
    MoveToBottomOfLines,
    MoveToLineNumber(u32),

    ShowLineNumber(bool),
    ShowNonPrinting(bool),
    HighLightWord(Option<&'a str>),

    IncrementLines(u32),
    DecrementLines(u32),
    SetNumOfLines(u32),
    Message(Option<&'a str>),
    Refresh,
    Quit,
}

pub struct Screen<'a> {
    linebuf: &'a [String],
    ostream: &'a mut Write,
    specified_numof_lines: u32,
    flushed_numof_lines: u32,
    specified_pt: Point, // buffer point

    show_nonprinting: bool,
    show_line_number: bool,
    show_highlight: bool,
    highlight_word: String,
    dirty: bool,
    message: String,
}

impl<'a> Screen<'a> {
    pub fn new(buf: &'a [String], ostream: &'a mut Write, nlines: u32) -> Self {
        // TODO: validate arguments or use builder
        let mut scr = Screen {
            linebuf: buf,
            ostream: ostream,
            specified_numof_lines: nlines,
            flushed_numof_lines: nlines,
            specified_pt: Point { x: 0, y: 0 },
            show_nonprinting: false,
            show_line_number: false,
            show_highlight: false,
            highlight_word: "".to_owned(),
            dirty: true,
            message: "".to_owned(),
        };
        scr.sweep_window(nlines);
        scr.flush();
        cis::cnl(&mut scr.ostream, nlines);
        scr
    }

    fn flush(&mut self) {
        self.ostream.flush().unwrap();
    }

    fn sweep_window(&mut self, nlines: u32) {
        for _ in 0..nlines {
            cis::el(&mut self.ostream, 2);
            writeln!(self.ostream).unwrap();
        }
        // cis::el(&mut self.ostream, 2);
        cis::cpl(&mut self.ostream, nlines);
    }

    fn move_to_home_position(&mut self) {
        cis::cpl(&mut self.ostream, self.flushed_numof_lines);
    }

    /// return (width, height)
    fn window_size(&self) -> io::Result<(u32, u32)> {
        terminal_size().map(|(w, h)| {
            (
                w as u32,
                if self.specified_numof_lines > h as u32 {
                    h as u32
                } else {
                    self.specified_numof_lines
                },
            )
        })
    }

    /// return range of visible lines
    fn lines_range(&self, wrows: u32) -> (usize, usize) {
        let wr = wrows as usize;
        let lbrows = self.linebuf.len();
        let y = self.specified_pt.y as usize;
        if wr > lbrows {
            // buflines length is less than win-rows.
            (0, lbrows)
        } else if y + wr >= lbrows {
            // buflines length is not enough at current pos.row. scroll down to fit.
            (lbrows - wr, lbrows)
        } else {
            (y, y + wr)
        }
    }

    /// return max line length of specified lines
    fn max_line_length(&self, (begin, end): (usize, usize)) -> usize {
        // TODO: validation
        self.linebuf[begin..end]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x))
    }

    /// return the end of y that is considered window size.
    fn limit_bottom_y(&self) -> u32 {
        let (_, wh) = self.window_size().unwrap();
        self.linebuf.len() as u32 - wh
    }

    /// return the cursor offset that is considered window size and string length.
    fn fit_offset(&self, offset: u32, lnlen: u32, winwidth: u32) -> u32 {
        let margin_right = 8;
        let mrgined_lnlen = lnlen + margin_right;

        if winwidth >= mrgined_lnlen {
            0
        } else if offset + winwidth <= mrgined_lnlen {
            offset
        } else {
            // offset + winwidth > lnlen
            mrgined_lnlen - winwidth
        }
    }

    fn decorate(&self, raw: &str, line_number: usize) -> String {
        // TODO: implement
        let (ww, _) = self.window_size().unwrap();
        let x = self.specified_pt.x as usize;

        let (mut begin, end): (usize, usize) = (0, ww as usize);
        let mut line = String::new();

        if self.show_line_number {
            line.push_str(format!("{:>4} ", line_number).as_str());
            begin += 5;
        }
        line.push_str(format!("{}", raw.get(x..cmp::min(raw.len(), x + (end - begin))).unwrap_or("")).as_str());
        line
    }

    fn refresh(&mut self) {
        if !self.dirty {
            return;
        }

        let (_, wh) = self.window_size().unwrap();
        let (begin, end) = self.lines_range(wh);

        self.move_to_home_position();
        let nlines = self.flushed_numof_lines;
        self.sweep_window(nlines + 1);

        for (i, ln) in self.linebuf[begin..end].iter().enumerate() {
            let dl = self.decorate(&ln, begin + i + 1);
            writeln!(self.ostream, "{}", dl);
        }

        write!(self.ostream, ":{}", self.message);
        self.flush();
        self.dirty = false;

        self.flushed_numof_lines = (end - begin) as u32;
    }

    fn scrcall_move_down(&mut self, n: u32) {
        let end_y = self.limit_bottom_y();
        let y = if self.specified_pt.y + n > end_y {
            end_y
        } else {
            self.specified_pt.y + n
        };
        if y == self.specified_pt.y {
            return;
        }
        self.specified_pt.y = y;
        self.dirty = true;
    }

    fn scrcall_move_up(&mut self, n: u32) {
        let y = if self.specified_pt.y <= n {
            0
        } else {
            self.specified_pt.y - n
        };
        if y == self.specified_pt.y {
            return;
        }
        self.specified_pt.y = y;
        self.dirty = true;
    }

    fn scrcall_move_left(&mut self, n: u32) {
        let x = if self.specified_pt.x <= n {
            0
        } else {
            self.specified_pt.x - n
        };
        if x == self.specified_pt.x {
            return;
        }
        self.specified_pt.x = x;
        self.dirty = true;
    }

    fn scrcall_move_right(&mut self, n: u32) {
        let (ww, wh) = self.window_size().unwrap();
        let max_lnlen = self.max_line_length(self.lines_range(wh)) as u32;
        let x = self.fit_offset(self.specified_pt.x + n, max_lnlen, ww);
        if x == self.specified_pt.x {
            return;
        }
        self.specified_pt.x = x;
        self.dirty = true;
    }

    fn scrcall_move_down_halfpages(&mut self, n: u32) {
        let (_, wh) = self.window_size().unwrap();
        let hpages = (wh * n) / 2;
        self.scrcall_move_down(hpages);
    }

    fn scrcall_move_up_halfpages(&mut self, n: u32) {
        let (_, wh) = self.window_size().unwrap();
        let hpages = (wh * n) / 2;
        self.scrcall_move_up(hpages);
    }

    fn scrcall_move_down_pages(&mut self, n: u32) {
        let (_, wh) = self.window_size().unwrap();
        let pages = wh * n;
        self.scrcall_move_down(pages);
    }

    fn scrcall_move_up_pages(&mut self, n: u32) {
        let (_, wh) = self.window_size().unwrap();
        let pages = wh * n;
        self.scrcall_move_up(pages);
    }

    fn scrcall_move_to_head_of_line(&mut self) {
        if self.specified_pt.x == 0 {
            return;
        }
        self.specified_pt.x = 0;
        self.dirty = true;
    }

    fn scrcall_move_to_end_of_line(&mut self) {
        let (ww, wh) = self.window_size().unwrap();
        let max_lnlen = self.max_line_length(self.lines_range(wh)) as u32;
        let x = self.fit_offset(max_lnlen, max_lnlen, ww);
        if x == self.specified_pt.x {
            return;
        }
        self.specified_pt.x = x;
        self.dirty = true;
    }

    fn scrcall_move_to_top_of_lines(&mut self) {
        if self.specified_pt.y == 0 {
            return;
        }
        self.specified_pt = Point { x: 0, y: 0 };
        self.dirty = true;
    }

    fn scrcall_move_to_bottom_of_lines(&mut self) {
        let y = self.limit_bottom_y();
        if self.specified_pt.y == y {
            return;
        }
        self.specified_pt = Point { x: 0, y: y };
        self.dirty = true;
    }

    fn scrcall_move_to_line_number(&mut self, n: u32) {
        let y = if n >= self.linebuf.len() as u32 {
            self.linebuf.len() as u32 - 1
        } else {
            n
        };
        if self.specified_pt.y == n {
            return;
        }
        self.specified_pt.y = y;
        self.dirty = true;
    }

    fn scrcall_show_line_number(&mut self, b: bool) {
        if b == self.show_line_number {
            return;
        };
        self.show_line_number = b;
        // self.dirty = true;
    }

    fn scrcall_show_nonprinting(&mut self, b: bool) {
        if b == self.show_nonprinting {
            return;
        };
        self.show_nonprinting = b;
        // self.dirty = true;
    }

    fn scrcall_highlight_word(&mut self, hlword: Option<&str>) {
        match hlword {
            Some(w) => {
                if w.is_empty() {
                    if !self.show_highlight {
                        return;
                    }
                    self.show_highlight = false;
                    self.dirty = true;
                } else {
                    if self.show_highlight && w == self.highlight_word {
                        return;
                    }
                    self.show_highlight = true;
                    self.highlight_word = w.to_owned();
                    self.dirty = true;
                }
            }
            None => if self.show_highlight {
                self.show_highlight = false;
                self.dirty = true;
            },
        }
    }

    fn scrcall_increment_lines(&mut self, n: u32) {
        let nl = self.specified_numof_lines + n;
        self.scrcall_set_numof_lines(nl);
    }

    fn scrcall_decrement_lines(&mut self, n: u32) {
        let nl = self.specified_numof_lines - n;
        if nl <= 0 {
            return;
        }
        self.scrcall_set_numof_lines(nl);
    }

    fn scrcall_set_numof_lines(&mut self, n: u32) {
        if n == 0 || n == self.specified_numof_lines {
            return;
        }
        self.specified_numof_lines = n;
        self.dirty = true;
    }

    fn scrcall_message(&mut self, msg: Option<&str>) {
        match msg {
            Some(m) => {
                self.message = m.to_owned();
            }
            None => {
                self.message.clear();
            }
        }
        self.dirty = true;
    }

    fn scrcall_refresh(&mut self) {
        self.dirty = true;
    }

    fn scrcall_quit(&mut self) {
        cis::el(&mut self.ostream, 2);
        writeln!(self.ostream);
        self.flush();
    }

    pub fn call(&mut self, scrcall: ScreenCall) {
        match scrcall {
            ScreenCall::MoveDown(n) => self.scrcall_move_down(n),
            ScreenCall::MoveUp(n) => self.scrcall_move_up(n),
            ScreenCall::MoveLeft(n) => self.scrcall_move_left(n),
            ScreenCall::MoveRight(n) => self.scrcall_move_right(n),
            ScreenCall::MoveDownHalfPages(n) => self.scrcall_move_down_halfpages(n),
            ScreenCall::MoveUpHalfPages(n) => self.scrcall_move_up_halfpages(n),
            ScreenCall::MoveDownPages(n) => self.scrcall_move_down_pages(n),
            ScreenCall::MoveUpPages(n) => self.scrcall_move_up_pages(n),
            ScreenCall::MoveToHeadOfLine => self.scrcall_move_to_head_of_line(),
            ScreenCall::MoveToEndOfLine => self.scrcall_move_to_end_of_line(),
            ScreenCall::MoveToTopOfLines => self.scrcall_move_to_top_of_lines(),
            ScreenCall::MoveToBottomOfLines => self.scrcall_move_to_bottom_of_lines(),
            ScreenCall::MoveToLineNumber(n) => self.scrcall_move_to_line_number(n),
            ScreenCall::ShowLineNumber(b) => self.scrcall_show_line_number(b),
            ScreenCall::ShowNonPrinting(b) => self.scrcall_show_nonprinting(b),
            ScreenCall::HighLightWord(hlword) => self.scrcall_highlight_word(hlword),
            ScreenCall::IncrementLines(n) => self.scrcall_increment_lines(n),
            ScreenCall::DecrementLines(n) => self.scrcall_decrement_lines(n),
            ScreenCall::SetNumOfLines(n) => self.scrcall_set_numof_lines(n),
            ScreenCall::Message(msg) => self.scrcall_message(msg),
            ScreenCall::Refresh => self.scrcall_refresh(),
            ScreenCall::Quit => self.scrcall_quit(),
        }
        self.refresh();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() {}

    fn teardown() {}

    #[test]
    fn test_screen() {
        use std::io;
        use std::{thread, time};

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
        scr.call(ScreenCall::Message(Some("this is debug message")));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveDown(2));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveRight(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveRight(2));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveToEndOfLine);
        scr.call(ScreenCall::Message(Some("this message will be cleared soon.")));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveLeft(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveLeft(2));
        scr.call(ScreenCall::Message(None));
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
        scr.call(ScreenCall::MoveDown(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::MoveDown(1));
        thread::sleep(time::Duration::from_millis(500));
        scr.call(ScreenCall::Quit);
        thread::sleep(time::Duration::from_millis(500));

    }
}
