//! Pane module

use termion;
use std::cmp;
use std::io::{self, Write};
use std::ops;
use csi::cursor_ext;

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
}

#[derive(Debug)]
pub enum ScrollStep {
    Char(u16),
    Halfpage(u16),
    Page(u16),
}

impl ScrollStep {
    pub fn to_numof_chars(&self, page_size: u16) -> u16 {
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
            height: 5,
            numof_flushed_lines: 5,
            cur_pos: (0, 0),
            fullscreen: false,
            show_linenumber: false,
            show_highlight: false,
            highlight_word: "".to_owned(),
            message: "".to_owned(),
        };
        pane.sweep();
        pane.move_to_message_row();
        pane.flush();
        pane
    }

    pub fn load(&mut self, buf: &'a [String]) {
        self.linebuf = buf;
    }

    fn flush(&mut self) {
        self.writer.flush().unwrap();
    }

    fn sweep(&mut self) {
        write!(self.writer, "{}", cursor_ext::HorizontalAbsolute(1));
        write!(self.writer, "{}", termion::clear::AfterCursor);
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

    fn move_to_message_row(&mut self) {
        let ph = self.size().unwrap_or((1, 1)).1;
        write!(self.writer, "{}", cursor_ext::NextLine(ph));
    }

    fn return_home(&mut self) {
        write!(self.writer, "{}", cursor_ext::PreviousLine(self.numof_flushed_lines));
    }

    /// return (width, height)
    pub fn size(&self) -> io::Result<(u16, u16)> {
        termion::terminal_size().map(|(tw, th)| {
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

    /// return the end of y that is considered buffer rows and window size
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
                // buffer rows does not fill pane rows
                0..buf_height
            } else if buf_height <= y + pane_height {
                // buffer rows is not enough at current pos. scroll up to fit.
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

    pub fn goto_top_of_rows(&mut self) -> io::Result<(u16, u16)> {
        self.cur_pos = (0, 0);
        Ok(self.cur_pos)
    }

    pub fn goto_bottom_of_rows(&mut self) -> io::Result<(u16, u16)> {
        let y = self.limit_bottom_y().unwrap();
        self.cur_pos = (0, y);
        Ok(self.cur_pos)
    }

    pub fn goto_head_of_row(&mut self) -> io::Result<(u16, u16)> {
        self.cur_pos.0 = 0;
        Ok(self.cur_pos)
    }

    pub fn goto_tail_of_row(&mut self) -> io::Result<(u16, u16)> {
        let max_visible_line_len = self.linebuf[self.range_of_visible_lines().unwrap()]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x)) as u16;
        self.cur_pos.0 = self.limit_right_x(max_visible_line_len, max_visible_line_len).unwrap();
        Ok(self.cur_pos)
    }

    pub fn goto_absolute_row(&mut self, row: u16) -> io::Result<u16> {
        let buf_height = self.linebuf.len() as u16;
        self.cur_pos.1 = if row >= buf_height {
            buf_height - 1
        } else {
            row
        };
        Ok(self.cur_pos.1)
    }

    pub fn goto_absolute_column(&mut self, col: u16) -> io::Result<u16> {
        let max_visible_line_len = self.linebuf[self.range_of_visible_lines()?]
            .iter()
            .map(|s| s.len())
            .fold(0, |acc, x| cmp::max(acc, x)) as u16;
        self.cur_pos.0 = self.limit_right_x(col, max_visible_line_len)?;
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
        let max = termion::terminal_size()?.1;
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

    fn setup() {}

    fn teardown() {}

    #[test]
    fn test_pane_hoge() {
    }
}
