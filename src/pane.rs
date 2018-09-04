//! Pane module

use csi::cursor_ext;
use search::{NullSearcher, Search};
use std::cell::RefCell;
use std::cmp;
use std::io::{self, Write};
use std::ops;
use std::rc::Rc;
use termion;

const DEFAULT_PANE_HEIGHT: u16 = 1;

use std::fmt;
pub struct ExtendMark(pub char);
impl fmt::Display for ExtendMark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            termion::style::Invert,
            self.0,
            termion::style::Reset
        )
    }
}

pub struct Pane<'a> {
    linebuf: Rc<RefCell<Vec<String>>>,
    writer: Box<RefCell<'a + Write>>,
    height: u16,
    numof_flushed_lines: u16,
    // cur_pos: (x, y)
    cur_pos: (u16, u16),
    show_linenumber: bool,
    show_highlight: bool,
    hlsearcher: Rc<RefCell<Search>>,
    message: String,
    termsize_getter: Box<Fn() -> io::Result<(u16, u16)>>,
}

#[derive(Debug)]
pub enum ScrollStep {
    Char(u16),
    HalfPage(u16),
    Page(u16),
}

impl ScrollStep {
    fn to_numof_chars(&self, page_size: u16) -> u16 {
        match *self {
            ScrollStep::Char(n) => n,
            ScrollStep::HalfPage(n) => (page_size * n) / 2,
            ScrollStep::Page(n) => page_size * n,
        }
    }
}

impl<'a> Pane<'a> {
    const MARGIN_RIGHT_WIDTH: u16 = 2;
    const MESSAGE_BAR_HEIGHT: u16 = 1;

    pub fn new<W: 'a + Write>(w: Box<RefCell<W>>) -> Self {
        let mut pane = Pane {
            linebuf: Rc::new(RefCell::new(Vec::new())),
            writer: w,
            height: DEFAULT_PANE_HEIGHT,
            numof_flushed_lines: DEFAULT_PANE_HEIGHT,
            cur_pos: (0, 0),
            show_linenumber: false,
            show_highlight: false,
            hlsearcher: Rc::new(RefCell::new(NullSearcher::new())),
            message: "".to_owned(),
            termsize_getter: if cfg!(test) {
                Box::new(move || Ok((10, 10)))
            } else {
                Box::new(termion::terminal_size)
            },
        };

        // limit pane height if terminal height is less than pane height.
        pane.set_height(DEFAULT_PANE_HEIGHT).expect("terminal_size get error");
        pane.numof_flushed_lines = pane.height;

        pane.sweep(pane.height);
        pane.move_to_message_line();
        pane.flush();
        pane
    }

    #[cfg(test)]
    fn replace_termsize_getter(&mut self, getter: Box<Fn() -> io::Result<(u16, u16)>>) {
        self.termsize_getter = getter;
    }

    /// Load text buffer and reset position.
    /// After this function is called, current pane postion is set to (0, 0).
    pub fn load(&mut self, buf: Rc<RefCell<Vec<String>>>) {
        self.linebuf = buf;
        self.cur_pos = (0, 0);
    }

    fn flush(&self) {
        self.writer.borrow_mut().flush().unwrap();
    }

    fn sweep(&self, n: u16) {
        let mut s = String::new();
        s.push_str(&format!("{}", cursor_ext::HorizontalAbsolute(1)));
        for _ in 0..n {
            s.push_str(&format!("{}\n", termion::clear::CurrentLine));
        }
        if n > 0 {
            s.push_str(&format!("{}{}",
                                termion::clear::CurrentLine,
                                cursor_ext::PreviousLine(n)));
        } else {
            // n == 0
            s.push_str(&format!("{}", termion::clear::CurrentLine));
        }
        self.writer.borrow_mut().write_all(s.as_bytes()).unwrap();
    }

    /// Return the range that matches the highlight word.
    fn hl_match_ranges(&self, raw: &str) -> Vec<(usize, usize)> {
        let mut v: Vec<(usize, usize)> = vec![];
        if self.hlsearcher.borrow().as_str().is_empty() {
            return v;
        }
        for m in self.hlsearcher.borrow().find_iter(raw) {
            v.push((m.start(), m.end()));
        }
        v
    }

    fn hl_words_for_trimed(trimed: &str, trimrange: &(usize, usize), hlranges: &[(usize, usize)]) -> String {
        let mut hlline = String::new();
        let mut copied = 0;
        let offset = trimrange.0;
        let end = trimrange.1 - offset;

        for &(hl_s, hl_e) in hlranges.iter() {
            if hl_e < trimrange.0 {
                continue;
            }
            else if hl_s <= trimrange.0 && hl_e >= trimrange.1 {
                // highlight whole line
                // _[____]_
                hlline.push_str(&format!("{}{}{}",
                                        termion::style::Invert,
                                        trimed,
                                        termion::style::Reset));
                copied = end;
                break;
            }
            else if hl_s <= trimrange.0 && hl_e > trimrange.0 {
                // _[_   ]
                hlline.push_str(&format!("{}{}{}",
                                        termion::style::Invert,
                                        trimed.get(..hl_e - offset).unwrap(),
                                        termion::style::Reset));
                copied = hl_e - offset;
            }
            else if hl_s >= trimrange.0 && hl_e <= trimrange.1 {
                //  [ __ ]
                hlline.push_str(&format!("{}{}{}{}",
                                        trimed.get(copied..hl_s - offset).unwrap(),
                                        termion::style::Invert,
                                        trimed.get(hl_s - offset..hl_e - offset).unwrap(),
                                        termion::style::Reset));
                copied = hl_e - offset;
            }
            else if hl_s < trimrange.1 && hl_e >= trimrange.1 {
                //  [   _]_
                hlline.push_str(&format!("{}{}{}{}",
                                        trimed.get(copied..hl_s - offset).unwrap(),
                                        termion::style::Invert,
                                        trimed.get(hl_s - offset..).unwrap(),
                                        termion::style::Reset));
                copied = end;
                break;
            }
            else if hl_s > trimrange.1 {
                //  [    ]_
                hlline.push_str(&format!("{}", trimed.get(copied..).unwrap()));
                copied = end;
                break;
            }
        }

        if copied < end {
            hlline.push_str(&format!("{}", trimed.get(copied..).unwrap()));
        }

        hlline
    }

    /// Get ranges that is considered unicode width
    fn unicode_range(raw: &str, start: usize, end: usize) -> (usize, usize) {
        use unicode_width::UnicodeWidthChar;

        if start >= raw.len() {
            return (raw.len(), raw.len())
        }

        let mut search_end = false;
        let mut width_from_head = 0;
        let limit_width = end - start;
        let mut awidth = 0;
        let mut us = 0;
        let mut ue = raw.len();
        for (i, c) in raw.char_indices() {
            if let Some(n) = c.width_cjk() {
                if !search_end {
                    if width_from_head >= start {
                        us = i;
                        search_end = true;
                    }
                    width_from_head += n;
                } else {
                    width_from_head += n;
                    awidth += n;
                    if awidth >= limit_width {
                        // overflow, use previous ue
                        break;
                    }
                    ue = i;
                    if width_from_head >= end {
                        break;
                    }
                }
            }
        }
        if ue < raw.len() {
            ue += 1;
            while !raw.is_char_boundary(ue) { ue += 1; }
        }
        (us, ue)
    }

    /// Decorate line
    fn decorate(&self, raw: &str, line_number: u16) -> String {
        let extend_mark_space: usize = 2;

        let pane_width = self.pane_size().unwrap().0;

        // visble raw trimming range
        let mut raw_range = (
            self.cur_pos.0 as usize,
            (self.cur_pos.0 + pane_width) as usize - extend_mark_space
        );

        // subtract line number space from raw_range
        if self.show_linenumber {
            let lnum_space = 4;
            raw_range.1 = if raw_range.1 - lnum_space > raw_range.0 {
                raw_range.1 - lnum_space
            } else {
                raw_range.0
            };
        }

        // get range that considered to unicode width
        let uc_range = Pane::unicode_range(raw, raw_range.0, raw_range.1);

        // trimed line
        let trimed = raw.get(uc_range.0..uc_range.1).unwrap();

        // highlight line
        let hl_ranges = self.hl_match_ranges(raw);
        let hlline = Pane::hl_words_for_trimed(&trimed, &uc_range, &hl_ranges);

        // add line number
        let lnum = if self.show_linenumber {
            format!("{:>4}", line_number + 1)
        } else {
            String::new()
        };

        // add extend marks
        let sol = if uc_range.0 > 0 {
            format!("{}", ExtendMark('+'))
        } else {
            " ".to_owned()
        };

        // add extend marks
        let eol = if raw.len() > uc_range.1 {
            format!("{}{}", cursor_ext::HorizontalAbsolute(pane_width), ExtendMark('+'))
        } else {
            format!("{}", termion::style::Reset)
        };

        format!("{}{}{}{}", lnum, sol, hlline, eol)
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        // decorate content lines
        let pane_height = self.pane_size()?.1;
        let buf_range = self.range_of_visible_lines()?;
        let mut block = String::new();
        for (i, line) in self.linebuf.borrow()[buf_range.start..buf_range.end]
            .iter()
            .enumerate()
        {
            block.push_str(&format!(
                "{}\n",
                self.decorate(&line, (buf_range.start + i) as u16)
            ));
        }

        // move down to message bar position
        let numof_lines_to_message_bar = pane_height - buf_range.len() as u16;
        if numof_lines_to_message_bar > 0 {
            block.push_str(&format!(
                    "{}",
                    cursor_ext::NextLine(numof_lines_to_message_bar)
            ));
        }

        // message line
        if self.message.is_empty() && buf_range.end == self.linebuf.borrow().len() {
            block.push_str(&format!(
                "{}(END){}",
                termion::style::Invert,
                termion::style::Reset
            ));
        } else {
            block.push_str(&self.message);
        };

        self.return_home();
        self.sweep(cmp::max(self.numof_flushed_lines, pane_height));
        self.writer
            .borrow_mut()
            .write_all(block.as_bytes())
            .unwrap();
        self.flush();
        self.numof_flushed_lines = pane_height;
        Ok(())
    }

    pub fn quit(&self) {
        write!(
            self.writer.borrow_mut(),
            "{}{}",
            cursor_ext::HorizontalAbsolute(1),
            termion::clear::CurrentLine
        ).unwrap();
        self.flush();
    }

    pub fn show_line_number(&mut self, b: bool) {
        self.show_linenumber = b;
    }

    pub fn show_highlight(&mut self, b: bool) {
        self.show_highlight = b;
    }

    pub fn set_highlight_searcher(&mut self, searcher: Rc<RefCell<Search>>) {
        self.hlsearcher = searcher;
    }

    pub fn set_message(&mut self, msg: Option<&str>) {
        if let Some(m) = msg {
            self.message = m.to_owned();
        } else {
            self.message.clear();
        }
    }

    fn move_to_message_line(&self) {
        let ph = self.pane_size().unwrap_or((1, 1)).1;
        write!(self.writer.borrow_mut(), "{}", cursor_ext::NextLine(ph)).unwrap();
    }

    fn return_home(&self) {
        if self.numof_flushed_lines > 0 {
            write!(
                self.writer.borrow_mut(),
                "{}",
                cursor_ext::PreviousLine(self.numof_flushed_lines)
            ).unwrap();
        }
    }

    /// Return pane size (width, height)
    pub fn pane_size(&self) -> io::Result<(u16, u16)> {
        (*self.termsize_getter)().map(|(tw, th)| (tw, cmp::min(th, self.height)))
    }

    /// Return (x, y)
    pub fn position(&self) -> (u16, u16) {
        self.cur_pos
    }

    /// Return the end of y that is considered buffer lines and window size
    fn limit_bottom_y(&self) -> io::Result<u16> {
        let linebuf_height = self.linebuf.borrow().len() as u16;
        let pane_height = self.pane_size()?.1;

        Ok(if linebuf_height > pane_height {
            linebuf_height - pane_height
        } else {
            0
        })
    }

    /// Return range of visible lines from current line to buffer line end or bottom of pane.
    fn range_of_visible_lines(&self) -> io::Result<ops::Range<usize>> {
        let pane_height = self.pane_size()?.1 as usize;
        let buf_height = self.linebuf.borrow().len();
        let y = self.cur_pos.1 as usize;

        Ok(y..
           if (buf_height - y) < pane_height {
               buf_height
           } else {
               y + pane_height
           }
        )
    }

    /// Return the horizontal offset that is considered pane size and string length
    fn limit_right_x(&self, next_x: u16, max_len: u16) -> io::Result<u16> {
        let margined_len = max_len + Pane::MARGIN_RIGHT_WIDTH;
        let pane_width = self.pane_size()?.0;

        Ok(if pane_width >= margined_len {
            0
        } else if next_x + pane_width <= margined_len {
            next_x
        } else {
            margined_len - pane_width
        })
    }

    // return actual scroll distance
    pub fn scroll_up(&mut self, ss: &ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.pane_size()?.1);
        let astep = if self.cur_pos.1 > step {
            step
        } else {
            self.cur_pos.1
        };
        self.cur_pos.1 -= astep;
        Ok(astep)
    }

    // return actual scroll distance
    pub fn scroll_down(&mut self, ss: &ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.pane_size()?.1);
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
    pub fn scroll_left(&mut self, ss: &ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.pane_size()?.0);
        let astep = if self.cur_pos.0 > step {
            step
        } else {
            self.cur_pos.0
        };
        self.cur_pos.0 -= astep;
        Ok(astep)
    }

    // return actual scroll distance
    pub fn scroll_right(&mut self, ss: &ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.pane_size()?.0);
        let max_visible_line_len = self.linebuf.borrow()[self.range_of_visible_lines()?]
            .iter()
            .map(|s| s.len())
            .fold(0, cmp::max) as u16;
        let x = self.limit_right_x(self.cur_pos.0 + step, max_visible_line_len)?;
        assert!(
            x >= self.cur_pos.0,
            format!("{} > {} is not pass!", x, self.cur_pos.0)
        );
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

    /// Go to head of current line.
    pub fn goto_head_of_line(&mut self) -> io::Result<(u16, u16)> {
        self.cur_pos.0 = 0;
        Ok(self.cur_pos)
    }

    /// Go to tail of current line.
    pub fn goto_tail_of_line(&mut self) -> io::Result<(u16, u16)> {
        let max_visible_line_len = self.linebuf.borrow()[self.range_of_visible_lines().unwrap()]
            .iter()
            .map(|s| s.len())
            .fold(0, cmp::max) as u16;
        self.cur_pos.0 = self
            .limit_right_x(max_visible_line_len, max_visible_line_len)
            .unwrap();
        Ok(self.cur_pos)
    }

    /// Go to specified absolute line number.
    /// Scroll so that the specified line appears at the top of the pane.
    pub fn goto_absolute_line(&mut self, line: u16) -> io::Result<u16> {
        let buf_height = self.linebuf.borrow().len() as u16;
        self.cur_pos.1 = if line >= buf_height {
            buf_height - 1
        } else {
            line
        };
        Ok(self.cur_pos.1)
    }

    pub fn goto_absolute_horizontal_offset(&mut self, offset: u16) -> io::Result<u16> {
        let max_visible_line_len = self.linebuf.borrow()[self.range_of_visible_lines()?]
            .iter()
            .map(|s| s.len())
            .fold(0, cmp::max) as u16;
        self.cur_pos.0 = self.limit_right_x(offset, max_visible_line_len)?;
        Ok(self.cur_pos.0)
    }

    /// Set pane height.
    /// Pane height is limited by the actual terminal height.
    /// Return acutually set pane height.
    pub fn set_height(&mut self, n: u16) -> io::Result<u16> {
        let max = (*self.termsize_getter)()?.1 - Pane::MESSAGE_BAR_HEIGHT;
        self.height = if n == 0 {
            1
        } else if n > max {
            max
        } else {
            n
        };
        Ok(self.height)
    }

    /// Increment pane height.
    /// Return acutually set pane height.
    pub fn increment_height(&mut self, n: u16) -> io::Result<u16> {
        let height = self.height + n;
        self.set_height(height)
    }

    /// Decrement pane height.
    /// Return acutually set pane height.
    pub fn decrement_height(&mut self, n: u16) -> io::Result<u16> {
        let height = if self.height > n { self.height - n } else { 1 };
        self.set_height(height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::BufWriter;

    macro_rules! gen_pane {
        ($w:expr) => {
            {
                let _w = BufWriter::new($w);
                Pane::new(Box::new(RefCell::new(_w)))
            }
        }
    }

    fn gen_texts(s: &[&str]) -> Rc<RefCell<Vec<String>>> {
        let mut v: Vec<String> = vec![];
        for t in s.iter() {
            v.push(t.to_string());
        }
        Rc::new(RefCell::new(v))
    }

    fn gen_sizer(w: u16, h: u16) -> ((u16, u16), Box<Fn() -> io::Result<(u16, u16)>>) {
        ((w, h), Box::new(move || Ok((w, h))))
    }

    #[test]
    fn test_scroll_up_down() {
        let t = [
            "", "", "", "", "", "", "", "", "", "",
            "", "", "", "", "", "", "", "", "", "",
        ];
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 5);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        let pane_height = 4;
        let _ = pane.set_height(pane_height);

        let stride_page  = pane_height;
        let stride_hpage = pane_height / 2;

        // in range
        {
            assert_eq!(pane.position(), (0, 0));
            assert_eq!(pane.scroll_down(&ScrollStep::Char(1)).unwrap(), 1);
            assert_eq!(pane.scroll_down(&ScrollStep::Char(2)).unwrap(), 2);
            assert_eq!(pane.position(), (0, 3));

            assert_eq!(pane.scroll_up(&ScrollStep::Char(1)).unwrap(), 1);
            assert_eq!(pane.scroll_up(&ScrollStep::Char(2)).unwrap(), 2);
            assert_eq!(pane.position(), (0, 0));

            assert_eq!(pane.scroll_down(&ScrollStep::HalfPage(1)).unwrap(), stride_hpage);
            assert_eq!(pane.scroll_down(&ScrollStep::HalfPage(2)).unwrap(), stride_hpage * 2);
            assert_eq!(pane.position(), (0, stride_hpage * 3));

            assert_eq!(pane.scroll_up(&ScrollStep::HalfPage(1)).unwrap(), stride_hpage);
            assert_eq!(pane.scroll_up(&ScrollStep::HalfPage(2)).unwrap(), stride_hpage * 2);
            assert_eq!(pane.position(), (0, 0));

            assert_eq!(pane.scroll_down(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(pane.scroll_down(&ScrollStep::Page(2)).unwrap(), stride_page * 2);
            assert_eq!(pane.position(), (0, stride_page * 3));

            assert_eq!(pane.scroll_up(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(pane.scroll_up(&ScrollStep::Page(2)).unwrap(), stride_page * 2);
            assert_eq!(pane.position(), (0, 0));
        }

        // out of range
        {
            assert_eq!(
                pane.scroll_down(&ScrollStep::Page(10)).unwrap(),
                texts.borrow().len() as u16 - pane_height
            );
            assert_eq!(
                pane.position(),
                (0, texts.borrow().len() as u16 - pane_height)
            );

            assert_eq!(
                pane.scroll_up(&ScrollStep::Page(10)).unwrap(),
                texts.borrow().len() as u16 - pane_height
            );
            assert_eq!(pane.position(), (0, 0));
        }
    }

    #[test]
    fn test_scroll_left_right() {
        let t = [
            "1234567890123456789012345678901234567890"
        ];
        let texts = gen_texts(&t);
        let (size, sizer) = gen_sizer(4, 2);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        let stride_page  = size.0;
        let stride_hpage = size.0 / 2;

        // in range
        {
            assert_eq!(pane.position(), (0, 0));
            assert_eq!(pane.scroll_right(&ScrollStep::Char(1)).unwrap(), 1);
            assert_eq!(pane.scroll_right(&ScrollStep::Char(2)).unwrap(), 2);
            assert_eq!(pane.position(), (3, 0));

            assert_eq!(pane.scroll_left(&ScrollStep::Char(1)).unwrap(), 1);
            assert_eq!(pane.scroll_left(&ScrollStep::Char(2)).unwrap(), 2);
            assert_eq!(pane.position(), (0, 0));

            assert_eq!(pane.scroll_right(&ScrollStep::HalfPage(1)).unwrap(), stride_hpage);
            assert_eq!(pane.scroll_right(&ScrollStep::HalfPage(2)).unwrap(), stride_hpage * 2);
            assert_eq!(pane.position(), (stride_hpage * 3, 0));

            assert_eq!(pane.scroll_left(&ScrollStep::HalfPage(1)).unwrap(), stride_hpage);
            assert_eq!(pane.scroll_left(&ScrollStep::HalfPage(2)).unwrap(), stride_hpage * 2);
            assert_eq!(pane.position(), (0, 0));

            assert_eq!(pane.scroll_right(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(pane.scroll_right(&ScrollStep::Page(2)).unwrap(), stride_page * 2);
            assert_eq!(pane.position(), (stride_page * 3, 0));

            assert_eq!(pane.scroll_left(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(pane.scroll_left(&ScrollStep::Page(2)).unwrap(), stride_page * 2);
            assert_eq!(pane.position(), (0, 0));
        }

        // out of range
        {
            // need to consider right margin
            assert_eq!(
                pane.scroll_right(&ScrollStep::Page(10)).unwrap(),
                texts.borrow()[0].len() as u16 - size.0 + Pane::MARGIN_RIGHT_WIDTH
            );
            assert_eq!(
                pane.position(),
                (texts.borrow()[0].len() as u16 - size.0 + Pane::MARGIN_RIGHT_WIDTH, 0)
            );

            let (w, _) = pane.position();
            assert_eq!(
                pane.scroll_left(&ScrollStep::Page(10)).unwrap(),
                w,
            );
            assert_eq!(pane.position(), (0, 0));
        }

        let t = [
            "1234567890123456789012345678901234567890",
            ""
        ];
        let texts = gen_texts(&t);
        pane.load(texts.clone());
        assert_eq!(pane.goto_absolute_line(1).unwrap(), 1);
        // now, draw "" only in terminal.
        assert_eq!(
            pane.scroll_right(&ScrollStep::Char(10)).unwrap(),
            0
        );
    }

    #[test]
    fn test_goto_vertical_lines() {
        let t = [
            "", "", "", "", "", "", "", "", "", "",
            "", "", "", "", "", "", "", "", "", "",
        ];
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 5);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        let pane_height = 4;
        let _ = pane.set_height(pane_height);

        pane.scroll_right(&ScrollStep::Char(1)).unwrap();
        assert_eq!(
            pane.goto_bottom_of_lines().unwrap(),
            (0, texts.borrow().len() as u16 - pane_height)
        );
        assert_eq!(
            pane.position(),
            (0, texts.borrow().len() as u16 - pane_height)
        );

        pane.scroll_right(&ScrollStep::Char(1)).unwrap();
        assert_eq!(pane.goto_top_of_lines().unwrap(), (0, 0));
        assert_eq!(pane.position(), (0, 0));

        assert_eq!(pane.goto_absolute_line(4).unwrap(), 4);
        assert_eq!(pane.position(), (0, 4));
        assert_eq!(pane.goto_absolute_line(0).unwrap(), 0);
        assert_eq!(pane.position(), (0, 0));
        assert_eq!(
            pane.goto_absolute_line(100).unwrap(),
            texts.borrow().len() as u16 - 1
        );
        assert_eq!(
            pane.position(),
            (0, texts.borrow().len() as u16 - 1)
        );

        // case: buffer height is less than pane height
        let (_, sizer) = gen_sizer(2, 10);
        let t = ["", "", "", ""];
        let texts = gen_texts(&t);
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        let pane_height = 8;
        let _ = pane.set_height(pane_height);
        assert_eq!(
            pane.goto_bottom_of_lines().unwrap(),
            (0, 0)
        );
        assert_eq!(
            pane.position(),
            (0, 0)
        );
        let t = ["", "", "", "", "", "", "", "", "", ""];
        let texts = gen_texts(&t);
        pane.load(texts.clone());
        assert_eq!(
            pane.goto_bottom_of_lines().unwrap(),
            (0, 2)
        );
        assert_eq!(
            pane.position(),
            (0, 2)
        );
    }

    #[test]
    fn test_goto_horizontal_line() {
        let t = [
            "1234567890123456789012345678901234567890"
        ];
        let texts = gen_texts(&t);
        let (size, sizer) = gen_sizer(4, 2);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        pane.scroll_right(&ScrollStep::Char(1)).unwrap();
        assert_eq!(pane.goto_head_of_line().unwrap(), (0, 0));
        assert_eq!(
            pane.goto_tail_of_line().unwrap(),
            (
                texts.borrow()[0].len() as u16 - size.0 + Pane::MARGIN_RIGHT_WIDTH,
                0
            )
        );

        assert_eq!(pane.goto_absolute_horizontal_offset(4).unwrap(), 4);
        assert_eq!(pane.position(), (4, 0));
        assert_eq!(pane.goto_absolute_horizontal_offset(0).unwrap(), 0);
        assert_eq!(pane.position(), (0, 0));
        assert_eq!(
            pane.goto_absolute_horizontal_offset(100).unwrap(),
            texts.borrow()[0].len() as u16 - size.0 + Pane::MARGIN_RIGHT_WIDTH
        );
    }

    #[test]
    fn test_set_height() {
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        let (size, sizer) = gen_sizer(1, 10);
        pane.replace_termsize_getter(sizer);

        assert_eq!(pane.set_height(5).unwrap(), 5);
        assert_eq!(pane.pane_size().unwrap(), (1, 5));
        assert_eq!(pane.set_height(0).unwrap(), 1);
        assert_eq!(pane.pane_size().unwrap(), (1, 1));
        assert_eq!(pane.set_height(size.1).unwrap(), size.1 - Pane::MESSAGE_BAR_HEIGHT);
        assert_eq!(pane.pane_size().unwrap(), (1, size.1 - Pane::MESSAGE_BAR_HEIGHT));
        assert_eq!(pane.set_height(size.1 + 1).unwrap(), size.1 - Pane::MESSAGE_BAR_HEIGHT);
        assert_eq!(pane.pane_size().unwrap(), (1, size.1 - Pane::MESSAGE_BAR_HEIGHT));

        assert_eq!(pane.set_height(5).unwrap(), 5);
        assert_eq!(pane.pane_size().unwrap(), (1, 5));
        assert_eq!(pane.decrement_height(1).unwrap(), 4);
        assert_eq!(pane.pane_size().unwrap(), (1, 4));
        assert_eq!(pane.decrement_height(3).unwrap(), 1);
        assert_eq!(pane.pane_size().unwrap(), (1, 1));
        assert_eq!(pane.decrement_height(100).unwrap(), 1);
        assert_eq!(pane.pane_size().unwrap(), (1, 1));

        assert_eq!(pane.set_height(5).unwrap(), 5);
        assert_eq!(pane.pane_size().unwrap(), (1, 5));
        assert_eq!(pane.increment_height(1).unwrap(), 6);
        assert_eq!(pane.pane_size().unwrap(), (1, 6));
        assert_eq!(pane.increment_height(3).unwrap(), 9);
        assert_eq!(pane.pane_size().unwrap(), (1, 9));
        assert_eq!(pane.increment_height(100).unwrap(), size.1 - Pane::MESSAGE_BAR_HEIGHT);
        assert_eq!(pane.pane_size().unwrap(), (1, size.1 - Pane::MESSAGE_BAR_HEIGHT));
    }

    #[test]
    fn test_set_message() {
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.set_message(None);
        assert!(pane.message.is_empty());
        pane.set_message(Some("ThisIsTest"));
        assert_eq!(pane.message, "ThisIsTest");
    }

    #[test]
    fn test_set_highlight_searcher() {
    }

    #[test]
    fn test_show_highlight() {
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.show_highlight(true);
        assert_eq!(pane.show_highlight, true);
        pane.show_highlight(false);
        assert_eq!(pane.show_highlight, false);
    }

    #[test]
    fn test_show_line_number() {
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.show_line_number(true);
        assert_eq!(pane.show_linenumber, true);
        pane.show_line_number(false);
        assert_eq!(pane.show_linenumber, false);
    }

    #[test]
    fn test_load() {
        let a = [
            "1234567890123456789012345678901234567890"
        ];
        let b = [
            "ABCD", "EFGH", "IJKL", "MNOP", "QRST", "UVWX", "YZ"
        ];
        let atxt = gen_texts(&a);
        let btxt = gen_texts(&b);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        let (_, sizer) = gen_sizer(2, 5);
        pane.replace_termsize_getter(sizer);

        pane.load(atxt.clone());
        assert_eq!(pane.position(), (0, 0));
        assert_eq!(pane.linebuf.borrow().len(), atxt.borrow().len());
        pane.scroll_right(&ScrollStep::Char(1)).unwrap();
        assert_eq!(pane.position(), (1, 0));

        pane.load(btxt.clone());
        assert_eq!(pane.position(), (0, 0));
        assert_eq!(pane.linebuf.borrow().len(), btxt.borrow().len());
    }

    #[allow(dead_code)]
    fn test_refresh() {
        unimplemented!();
    }
    #[allow(dead_code)]
    fn test_quit() {
        unimplemented!();
    }

    #[test]
    fn test_limit_bottom_y() {
        let t = [
            "", "", "", "", "", "", "", "", "", "",
            "", "", "", "", "", "", "", "", "", "",
        ];
        let nbuflines = t.len() as u16;
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 10);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        assert_eq!(pane.set_height(1).unwrap(), 1);
        assert_eq!(
            pane.limit_bottom_y().unwrap(),
            nbuflines - 1
        );
        assert_eq!(pane.set_height(4).unwrap(), 4);
        assert_eq!(
            pane.limit_bottom_y().unwrap(),
            nbuflines - 4
        );
    }

    #[test]
    fn test_range_of_visible_lines() {
        let t = [
            "", "", "", "", "", "", "", "", "", "",
            "", "", "", "", "", "", "", "", "", "",
        ];
        let nbuflines = t.len() as u16;
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 10);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        assert_eq!(pane.set_height(5).unwrap(), 5);

        assert_eq!(pane.goto_absolute_line(0).unwrap(), 0);
        assert_eq!(
            pane.range_of_visible_lines().unwrap(),
            0..5
        );
        assert_eq!(pane.goto_absolute_line(10).unwrap(), 10);
        assert_eq!(
            pane.range_of_visible_lines().unwrap(),
            10..15
        );
        assert_eq!(pane.goto_absolute_line(nbuflines - 1).unwrap(), nbuflines - 1);
        assert_eq!(
            pane.range_of_visible_lines().unwrap(),
            (nbuflines as usize - 1)..(nbuflines as usize)
        );
    }

    #[test]
    fn test_limit_right_x() {
        let (size, sizer) = gen_sizer(20, 0);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.replace_termsize_getter(sizer);

        let max_text_length = 10;
        assert_eq!(pane.limit_right_x(0, max_text_length).unwrap(), 0);
        assert_eq!(pane.limit_right_x(5, max_text_length).unwrap(), 0);
        assert_eq!(pane.limit_right_x(20, max_text_length).unwrap(), 0);

        let max_text_length = 50;
        assert_eq!(pane.limit_right_x(0, max_text_length).unwrap(), 0);
        assert_eq!(pane.limit_right_x(20, max_text_length).unwrap(), 20);
        assert_eq!(
            pane.limit_right_x(40, max_text_length).unwrap(),
            // remain 10
            max_text_length - size.0 + Pane::MARGIN_RIGHT_WIDTH
        );
    }
}
