//! Pane module

use termion;
use std::cmp;
use std::io::{self, Write};
use std::ops;
use csi::cursor_ext;

const DEFAULT_PANE_HEIGHT: u16 = 5;

pub struct Pane<'a> {
    linebuf: &'a [String],
    writer: &'a mut Write,
    height: u16,
    numof_flushed_lines: u16,
    // cur_pos: (x, y)
    cur_pos: (u16, u16),
    fullscreen: bool,
    show_linenumber: bool,
    show_highlight: bool,
    highlight_word: String,
    message: String,
    termsize_getter: Box<Fn() -> io::Result<(u16, u16)>>,
}

#[derive(Debug)]
pub enum ScrollStep {
    Char(u16),
    Halfpage(u16),
    Page(u16),
}

impl ScrollStep {
    fn to_numof_chars(&self, page_size: u16) -> u16 {
        match *self {
            ScrollStep::Char(n) => n,
            ScrollStep::Halfpage(n) => (page_size * n) / 2,
            ScrollStep::Page(n) => page_size * n,
        }
    }
}

impl<'a> Pane<'a> {
    pub fn new(w: &'a mut Write) -> Self {
        let mut pane = Pane {
            linebuf: &[],
            writer: w,
            height: DEFAULT_PANE_HEIGHT,
            numof_flushed_lines: DEFAULT_PANE_HEIGHT,
            cur_pos: (0, 0),
            fullscreen: false,
            show_linenumber: false,
            show_highlight: false,
            highlight_word: "".to_owned(),
            message: "".to_owned(),
            termsize_getter: Box::new(|| termion::terminal_size()),
        };
        pane.sweep();
        pane.move_to_message_line();
        pane.flush();
        pane
    }

    #[cfg(test)]
    fn replace_termsize_getter(&mut self, getter: Box<Fn() -> io::Result<(u16, u16)>>) {
        self.termsize_getter = getter;
    }

    pub fn load(&mut self, buf: &'a [String]) {
        self.linebuf = buf;
        self.cur_pos = (0, 0);
    }

    fn flush(&mut self) {
        self.writer.flush().unwrap();
    }

    fn sweep(&mut self) {
        let mut s = String::new();
        s += &format!("{}", cursor_ext::HorizontalAbsolute(1));
        s += &format!("{}", termion::clear::AfterCursor);
        for _ in 0..self.height {
            s += "\n";
        }
        s += &format!("{}", cursor_ext::PreviousLine(self.height as u16));
        self.writer.write(s.as_bytes()).unwrap();
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        let buf_range = self.range_of_visible_lines()?;
        self.return_home();
        self.sweep();
        for (i, line) in self.linebuf[buf_range.start..buf_range.end].iter().enumerate() {
            writeln!(self.writer, "{}", line);
        }
        write!(self.writer, ":{}", self.message);
        self.flush();
        self.numof_flushed_lines = (buf_range.end - buf_range.start) as u16;
        Ok(())
    }

    pub fn quit(&mut self) {
        write!(self.writer, "{}", termion::clear::CurrentLine);
        writeln!(self.writer);
        self.flush();
    }

    pub fn show_line_number(&mut self, b: bool) {
        self.show_linenumber = b;
    }

    pub fn set_highlight_word(&mut self, hlword: Option<&str>) {
        if let Some(w) = hlword {
            if w.is_empty() {
                self.show_highlight = false;
            } else {
                self.show_highlight = true;
                self.highlight_word = w.to_owned();
            }
        } else {
            self.show_highlight = false;
        }
    }

    pub fn set_message(&mut self, msg: Option<&str>) {
        if let Some(m) = msg {
            self.message = m.to_owned();
        } else {
            self.message.clear();
        }
    }

    fn move_to_message_line(&mut self) {
        let ph = self.size().unwrap_or((1, 1)).1;
        write!(self.writer, "{}", cursor_ext::NextLine(ph));
    }

    fn return_home(&mut self) {
        write!(self.writer, "{}", cursor_ext::PreviousLine(self.numof_flushed_lines));
    }

    /// return (width, height)
    pub fn size(&self) -> io::Result<(u16, u16)> {
        (*self.termsize_getter)().map(|(tw, th)| {
            (tw, cmp::min(th, self.height))
        })
    }

    /// return (x, y)
    pub fn position(&self) -> (u16, u16) {
        self.cur_pos
    }

    pub fn highlight_word(&self) -> Option<&str> {
        if self.highlight_word.is_empty() {
            None
        } else {
            Some(&self.highlight_word)
        }
    }

    /// return the end of y that is considered buffer lines and window size
    fn limit_bottom_y(&self) -> io::Result<u16> {
        Ok(self.linebuf.len() as u16 - self.size()?.1)
    }

    /// return range of visible lines
    fn range_of_visible_lines(&self) -> io::Result<ops::Range<usize>> {
        let pane_height = self.size()?.1 as usize;
        let buf_height = self.linebuf.len();
        let y = self.cur_pos.1 as usize;

        Ok(
            if buf_height < pane_height {
                // buffer lines does not fill pane height
                0..buf_height
            } else if buf_height <= y + pane_height {
                // buffer lines is not enough at current pos. scroll up to fit.
                (buf_height - pane_height)..buf_height
            } else {
                y..(y + pane_height)
            }
        )
    }

    /// return the horizontal offset that is considered pane size and string length
    fn limit_right_x(&self, next_x: u16, max_len: u16) -> io::Result<u16> {
        // FIXME: magic number for right margin
        let margin_right = 8;
        let margined_len = max_len + margin_right;
        let pane_width = self.size()?.0;

        Ok(
            if pane_width >= margined_len {
                0
            } else if next_x + pane_width <= margined_len {
                next_x
            } else {
                margined_len - pane_width
            }
        )
    }

    // return actual scroll distance
    pub fn scroll_up(&mut self, ss: ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.size()?.1);
        let astep = if self.cur_pos.1 > step {
            step
        } else {
            self.cur_pos.1
        };
        self.cur_pos.1 -= astep;
        Ok(astep)
    }

    // return actual scroll distance
    pub fn scroll_down(&mut self, ss: ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.size()?.1);
        let end_y = self.limit_bottom_y()?;
        let astep = if self.cur_pos.1 + step < end_y {
            step
        } else {
            end_y - self.cur_pos.1
        };
        self.cur_pos.1 += astep;
        Ok(astep)
    }

    // return actual scroll distance
    pub fn scroll_left(&mut self, ss: ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.size()?.0);
        let astep = if self.cur_pos.0 > step {
            step
        } else {
            self.cur_pos.0
        };
        self.cur_pos.0 -= astep;
        Ok(astep)
    }

    // return actual scroll distance
    pub fn scroll_right(&mut self, ss: ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.size()?.0);
        let max_visible_line_len = self.linebuf[self.range_of_visible_lines()?]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x)) as u16;
        let x = self.limit_right_x(self.cur_pos.0 + step, max_visible_line_len)?;
        assert!(x > self.cur_pos.0, format!("{} > {} is not pass!", x, self.cur_pos.0));
        let astep = x - self.cur_pos.0;
        self.cur_pos.0 = x;
        Ok(astep)
    }

    pub fn goto_top_of_lines(&mut self) -> io::Result<(u16, u16)> {
        self.cur_pos = (0, 0);
        Ok(self.cur_pos)
    }

    pub fn goto_bottom_of_lines(&mut self) -> io::Result<(u16, u16)> {
        let y = self.limit_bottom_y().unwrap();
        self.cur_pos = (0, y);
        Ok(self.cur_pos)
    }

    pub fn goto_head_of_line(&mut self) -> io::Result<(u16, u16)> {
        self.cur_pos.0 = 0;
        Ok(self.cur_pos)
    }

    pub fn goto_tail_of_line(&mut self) -> io::Result<(u16, u16)> {
        let max_visible_line_len = self.linebuf[self.range_of_visible_lines().unwrap()]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x)) as u16;
        self.cur_pos.0 = self.limit_right_x(max_visible_line_len, max_visible_line_len).unwrap();
        Ok(self.cur_pos)
    }

    pub fn goto_absolute_line(&mut self, line: u16) -> io::Result<u16> {
        let buf_height = self.linebuf.len() as u16;
        self.cur_pos.1 = if line >= buf_height {
            buf_height - 1
        } else {
            line
        };
        Ok(self.cur_pos.1)
    }

    pub fn goto_absolute_horizontal_offset(&mut self, offset: u16) -> io::Result<u16> {
        let max_visible_line_len = self.linebuf[self.range_of_visible_lines()?]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x)) as u16;
        self.cur_pos.0 = self.limit_right_x(offset, max_visible_line_len)?;
        Ok(self.cur_pos.0)
    }

    pub fn set_height(&mut self, n: u16) -> io::Result<u16> {
        if n == 0 {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "Require non-zero value"))
        } else {
            self.height = n;
            Ok(self.height)
        }
    }

    pub fn increment_height(&mut self, n: u16) -> io::Result<u16> {
        let max = (*self.termsize_getter)()?.1;
        let height = if self.height + n < max { self.height + n } else { max };
        self.set_height(height)
    }

    pub fn decrement_height(&mut self, n: u16) -> io::Result<u16> {
        let height = if self.height > n { self.height - n } else { 0 };
        self.set_height(height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const static_texts: &'static [&'static str] = &[
        "11111111",
        "22222222",
        "33333333",
        "44444444",
        "5555555555555555",
        "6666666666666666",
        "7777777777777777",
        "8888888888888888",
        "99999999999999999999999999999999",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
        "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
        "EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE",
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        "GGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGGG",
        ];

    fn setup() {
    }

    fn teardown() {
    }

    fn texts() -> Vec<String> {
        let mut v: Vec<String> = vec![];
        for t in static_texts.iter() {
            v.push(t.to_string());
        }
        v
    }

    #[test]
    fn test_pane_scroll() {
        use std::io;
        use std::{thread, time};
        use std::fs::OpenOptions;
        use std::io::BufWriter;

        // let w = io::stdout();
        // let mut w = w.lock();
        let f = OpenOptions::new().write(true).open("/dev/null").unwrap();
        let mut w = BufWriter::new(f);

        let texts = texts();
        let mut pane = Pane::new(&mut w);
        pane.load(&texts);

        let size_getter = || Ok((10, 4));
        let size = size_getter().unwrap();
        let mut pos = pane.position();
        pane.replace_termsize_getter(Box::new(size_getter));

        assert!(pane.refresh().is_ok());

        // scroll down
        assert_eq!(pane.scroll_down(ScrollStep::Char(1)).unwrap(), 1);
        pos.1 += 1;
        assert_eq!(pane.position(), pos);
        assert_eq!(pane.scroll_down(ScrollStep::Char(3)).unwrap(), 3);
        pos.1 += 3;
        assert_eq!(pane.position(), pos);
        assert_eq!(pane.scroll_down(ScrollStep::Halfpage(1)).unwrap(), size.1/2);
        pos.1 += size.1/2;
        assert_eq!(pane.position(), pos);
        assert_eq!(pane.scroll_down(ScrollStep::Page(1)).unwrap(), size.1);
        pos.1 += size.1;
        assert_eq!(pane.position(), pos);
        // bottom limit
        let bottom = texts.len() as u16 - size.1;
        let remain = bottom - pos.1;
        assert_eq!(pane.scroll_down(ScrollStep::Page(10)).unwrap(), remain);
        pos.1 = bottom;
        assert_eq!(pane.position(), pos);

        // scroll up
        assert_eq!(pane.scroll_up(ScrollStep::Char(1)).unwrap(), 1);
        pos.1 -= 1;
        assert_eq!(pane.position(), pos);
        assert_eq!(pane.scroll_up(ScrollStep::Char(2)).unwrap(), 2);
        pos.1 -= 2;
        assert_eq!(pane.position(), pos);
        assert_eq!(pane.scroll_up(ScrollStep::Halfpage(2)).unwrap(), (size.1 * 2)/2);
        pos.1 -= (size.1 * 2)/2;
        assert_eq!(pane.position(), pos);
        assert_eq!(pane.scroll_up(ScrollStep::Page(1)).unwrap(), size.1);
        pos.1 -= size.1;
        assert_eq!(pane.position(), pos);
        // top limit
        assert_eq!(pane.scroll_up(ScrollStep::Page(10)).unwrap(), pos.1);
        pos.1 = 0;
        assert_eq!(pane.position(), pos);
    }

    #[test]
    fn test_pane() {
        use std::io;
        use std::{thread, time};

        let w = io::stdout();
        let mut w = w.lock();
        let texts = texts();
        let mut pane = Pane::new(&mut w);
        pane.replace_termsize_getter(Box::new(|| Ok((10, 5))));
        pane.load(&texts);
        pane.refresh();
        thread::sleep(time::Duration::from_millis(200));
        pane.scroll_down(ScrollStep::Char(1));
        thread::sleep(time::Duration::from_millis(200));
        pane.scroll_down(ScrollStep::Char(1));
        thread::sleep(time::Duration::from_millis(200));
        pane.scroll_down(ScrollStep::Char(1));

        pane.set_height(10);
        pane.refresh();

        pane.quit();
    }
}

