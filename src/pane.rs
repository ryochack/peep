//! Pane module

use csi::cursor_ext;
use search::{NullSearcher, Search};
use std::cell::RefCell;
use std::cmp;
use std::io::{self, BufRead, BufReader, Write};
use std::io::{Seek, SeekFrom};
use std::ops;
use std::rc::Rc;
use tab::TabExpand;
use termion;
use unicode_divide::UnicodeStrDivider;
use unicode_width::UnicodeWidthStr;

const DEFAULT_PANE_HEIGHT: u16 = 1;
const DEFAULT_TAB_WIDTH: usize = 4;

use std::fmt;
pub struct ExtendMark(pub char);
impl fmt::Display for ExtendMark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            termion::color::Fg(termion::color::LightBlack),
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
    numof_semantic_flushed_lines: u16,
    // cur_pos: (x, y)
    cur_pos: (u16, u16),
    show_linenumber: bool,
    show_highlight: bool,
    hlsearcher: Rc<RefCell<Search>>,
    message: String,
    termsize_getter: Box<Fn() -> io::Result<(u16, u16)>>,
    tab_width: usize,
    wraps_line: bool,
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
    const MARGIN_RIGHT_WIDTH: u16 = 4;
    const MESSAGE_BAR_HEIGHT: u16 = 1;

    pub fn new<W: 'a + Write>(w: Box<RefCell<W>>) -> Self {
        let mut pane = Pane {
            linebuf: Rc::new(RefCell::new(Vec::new())),
            writer: w,
            height: DEFAULT_PANE_HEIGHT,
            numof_flushed_lines: DEFAULT_PANE_HEIGHT,
            numof_semantic_flushed_lines: DEFAULT_PANE_HEIGHT,
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
            tab_width: DEFAULT_TAB_WIDTH,
            wraps_line: false,
        };

        // limit pane height if terminal height is less than pane height.
        pane.set_height(DEFAULT_PANE_HEIGHT)
            .expect("terminal_size get error");
        pane.numof_flushed_lines = pane.height;
        pane.numof_semantic_flushed_lines = pane.height;

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
            s.push_str(&format!(
                "{}{}",
                termion::clear::CurrentLine,
                cursor_ext::PreviousLine(n)
            ));
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

    fn hl_words_for_trimed(
        trimed: &str,
        trimrange: &(usize, usize),
        hlranges: &[(usize, usize)],
    ) -> String {
        let mut hlline = String::new();
        let mut copied = 0;
        let offset = trimrange.0;
        let end = trimrange.1 - offset;

        for &(hl_s, hl_e) in hlranges.iter() {
            if hl_e < trimrange.0 {
                continue;
            } else if hl_s <= trimrange.0 && hl_e >= trimrange.1 {
                // highlight whole line
                // _[____]_
                hlline.push_str(&format!(
                    "{}{}{}",
                    termion::style::Invert,
                    trimed,
                    termion::style::Reset
                ));
                copied = end;
                break;
            } else if hl_s <= trimrange.0 && hl_e > trimrange.0 {
                // _[_   ]
                hlline.push_str(&format!(
                    "{}{}{}",
                    termion::style::Invert,
                    trimed.get(..hl_e - offset).unwrap(),
                    termion::style::Reset
                ));
                copied = hl_e - offset;
            } else if hl_s >= trimrange.0 && hl_e <= trimrange.1 {
                //  [ __ ]
                hlline.push_str(&format!(
                    "{}{}{}{}",
                    trimed.get(copied..hl_s - offset).unwrap(),
                    termion::style::Invert,
                    trimed.get(hl_s - offset..hl_e - offset).unwrap(),
                    termion::style::Reset
                ));
                copied = hl_e - offset;
            } else if hl_s < trimrange.1 && hl_e >= trimrange.1 {
                //  [   _]_
                hlline.push_str(&format!(
                    "{}{}{}{}",
                    trimed.get(copied..hl_s - offset).unwrap(),
                    termion::style::Invert,
                    trimed.get(hl_s - offset..).unwrap(),
                    termion::style::Reset
                ));
                copied = end;
                break;
            } else if hl_s > trimrange.1 {
                //  [    ]_
                hlline.push_str(&trimed.get(copied..).unwrap().to_owned());
                copied = end;
                break;
            }
        }

        if copied < end {
            hlline.push_str(&trimed.get(copied..).unwrap().to_owned());
        }

        hlline
    }

    /// Generate line number string
    /// | 100 ......
    /// | 101 ......
    fn gen_line_number_string(width: usize, line_number: u16) -> String {
        match width {
            0...2 => format!("{:>2}", line_number + 1),
            3 => format!("{:>3}", line_number + 1),
            4 => format!("{:>4}", line_number + 1),
            _ => format!("{:>5}", line_number + 1),
        }
    }

    /// Generate blank line number string
    /// | 100 ......
    /// |    +......
    /// | 101 ......
    fn gen_blank_line_number_string(width: usize) -> String {
        // from the second line
        match width {
            0...2 => "  ".to_owned(),
            3 => "   ".to_owned(),
            4 => "    ".to_owned(),
            _ => "     ".to_owned(),
        }
    }

    /// Decorate line
    ///
    /// | 12+xxxxxxxxxxxxxxxxxxxxxxxxxx+|
    /// | 13+xxxxxxx.                   |
    /// | 14+xxxxxxxxxxxx.              |
    ///
    fn decorate_trim(&self, raw: &str, line_number: u16) -> String {
        // subtract line number space from raw_range
        let lnpw = self.line_number_printing_width();

        // replace tabs with spaces
        let raw_notab = raw.expand_tab(self.tab_width);

        // trim unicode str considering visual unicode width
        let mut ucdiv = UnicodeStrDivider::new(&raw_notab, self.width_of_text_area());
        let _ = ucdiv.seek(SeekFrom::Start(self.cur_pos.0 as u64));
        let trimed = ucdiv.next().unwrap_or("");
        let uc_range = ucdiv.last_range();

        // highlight line
        let hlline;
        let decorated = if self.show_highlight {
            let hl_ranges = self.hl_match_ranges(&raw_notab);
            hlline = Self::hl_words_for_trimed(&trimed, &uc_range, &hl_ranges);
            &hlline
        } else {
            trimed
        };

        // add line number
        let lnum = if self.show_linenumber {
            Self::gen_line_number_string(lnpw, line_number)
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
        let eol = if raw_notab.len() > uc_range.1 {
            format!(
                "{}{}",
                cursor_ext::HorizontalAbsolute(self.pane_size().unwrap().0),
                ExtendMark('+')
            )
        } else {
            format!("{}", termion::style::Reset)
        };

        format!("{}{}{}{}", lnum, sol, decorated, eol)
    }

    /// Decorate line
    ///
    /// | 12 xxxxxxxxxxxxxxxxxxxxxxxxxxx|
    /// |   >xxxxxxx.                   |
    /// | 13 xxxxxxxxxxxx.              |
    ///
    fn decorate_wrap(&self, raw: &str, line_number: u16) -> String {
        let lnpw = if self.show_linenumber {
            self.line_number_printing_width()
        } else {
            0
        };
        // subtract line number space and extend_mark space from raw_range
        let line_cap_width = self.width_of_text_area();

        // replace tabs with spaces
        let raw_notab = raw.expand_tab(self.tab_width);

        let mut ucdiv = UnicodeStrDivider::new(&raw_notab, self.width_of_text_area());

        let mut s = 0;
        let mut e = line_cap_width;

        let mut wrapped = String::new();
        let fn_lnum_string = |show_linenumber, width, start_pos, line_number| -> String {
            if show_linenumber {
                if start_pos == 0 {
                    Self::gen_line_number_string(width, line_number)
                } else {
                    Self::gen_blank_line_number_string(width)
                }
            } else {
                String::new()
            }
        };

        while let Some(trimed) = ucdiv.next() {
            let uc_range = ucdiv.last_range();

            // highlight line
            let hlline;
            let decorated = if self.show_highlight {
                let hl_ranges = self.hl_match_ranges(&raw_notab);
                hlline = Self::hl_words_for_trimed(&trimed, &uc_range, &hl_ranges);
                &hlline
            } else {
                trimed
            };

            // add line number
            let lnum = fn_lnum_string(self.show_linenumber, lnpw, s, line_number);
            // add wrap marks
            let sol = if s > 0 {
                format!("{}", ExtendMark('+'))
            } else {
                " ".to_owned()
            };

            wrapped.push_str(&format!("{}{}{}\n", lnum, sol, decorated));

            s = e;
            e += line_cap_width;
        }

        if wrapped.is_empty() {
            // add line number
            let lnum = fn_lnum_string(self.show_linenumber, lnpw, s, line_number);
            wrapped.push_str(&format!("{}\n", lnum));
        }

        wrapped
    }

    fn decorate(&self, raw: &str, line_number: u16) -> String {
        if self.wraps_line {
            self.decorate_wrap(raw, line_number)
        } else {
            self.decorate_trim(raw, line_number)
        }
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        // decorate content lines
        let pane_height = self.pane_size()?.1;
        let buf_range = self.range_of_visible_lines()?;
        let mut block = String::new();
        let mut n = 0;

        'outer: for (i, line) in self.linebuf.borrow()[buf_range.start..buf_range.end]
            .iter()
            .enumerate()
        {
            let deco = self.decorate(&line, (buf_range.start + i) as u16);
            let br = BufReader::new(deco.as_bytes());
            self.numof_semantic_flushed_lines = i as u16 + 1;
            for lline in br.lines() {
                block.push_str(&format!("{}\n", lline?));
                n += 1;
                if n >= pane_height {
                    break 'outer;
                }
            }
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
        if self.message.is_empty() && buf_range.start >= self.limit_bottom_y()? as usize {
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

    fn line_number_printing_width(&self) -> usize {
        match self.linebuf.borrow().len() {
            0...99 => 2,
            100...999 => 3,
            1000...9999 => 4,
            _ => 5,
        }
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

    pub fn set_message(&mut self, msg: Option<String>) {
        if let Some(m) = msg {
            self.message = m;
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

    /// Return logical lines (wrapped lines) of specified line number.
    fn count_wrapped_lines(&self, text: &str) -> u16 {
        let pane_width = self.width_of_text_area();
        if pane_width == 0 {
            0
        } else {
            (text.len() / pane_width) as u16 + 1
        }
    }

    /// Return the end of y that is considered buffer lines and window size and wrapped lines.
    fn limit_bottom_y(&self) -> io::Result<u16> {
        let linebuf_height = self.linebuf.borrow().len() as u16;
        let pane_height = self.pane_size()?.1;

        if !self.wraps_line {
            return Ok(if linebuf_height > pane_height {
                linebuf_height - pane_height
            } else {
                0
            });
        }

        // self.wraps_line is enabled
        let mut sum = 0;
        for i in (0..linebuf_height).rev() {
            sum += self.count_wrapped_lines(&self.linebuf.borrow()[i as usize]);
            if sum >= pane_height {
                return Ok(if i == linebuf_height {
                    linebuf_height
                } else {
                    i + 1
                });
            }
        }
        return Ok(0);
    }

    /// Return text area width.
    fn width_of_text_area(&self) -> usize {
        let pane_width = self.pane_size().unwrap().0 as usize;
        let extend_mark_space: usize = if self.wraps_line { 1 } else { 2 };
        let lnpw: usize = if self.show_linenumber {
            self.line_number_printing_width()
        } else {
            0
        };

        if pane_width > lnpw + extend_mark_space {
            pane_width - lnpw - extend_mark_space
        } else {
            0
        }
    }

    /// Return range of visible lines from current line to buffer line end or bottom of pane.
    fn range_of_visible_lines(&self) -> io::Result<ops::Range<usize>> {
        let pane_height = self.pane_size()?.1 as usize;
        let buf_height = self.linebuf.borrow().len();
        let y = self.cur_pos.1 as usize;

        Ok(y..if (buf_height - y) < pane_height {
            buf_height
        } else {
            y + pane_height
        })
    }

    /// Return max width of linebuf range
    fn max_width_of_visible_lines(&self, r: ops::Range<usize>) -> u16 {
        self.linebuf.borrow()[r]
            .iter()
            .map(|s| UnicodeWidthStr::width(s.as_str()))
            .fold(0, cmp::max) as u16
    }

    /// Return the pane printable width
    fn pane_printable_width(&self) -> io::Result<u16> {
        Ok(self.pane_size()?.0 - if self.show_linenumber {
            self.line_number_printing_width() as u16
        } else {
            0
        })
    }

    /// Return the horizontal offset that is considered pane size and string length
    fn limit_right_x(&self, next_x: u16, max_len: u16) -> io::Result<u16> {
        let margined_len = max_len + Self::MARGIN_RIGHT_WIDTH;
        let pane_width = self.pane_printable_width()?;
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
        let step = ss.to_numof_chars(self.numof_semantic_flushed_lines);
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
        let step = ss.to_numof_chars(self.numof_semantic_flushed_lines);
        let end_y = self.limit_bottom_y()?;
        let astep = if end_y > self.cur_pos.1 + step {
            step
        } else if end_y > self.cur_pos.1 {
            end_y - self.cur_pos.1
        } else {
            0
        };
        self.cur_pos.1 += astep;
        Ok(astep)
    }

    // return actual scroll distance
    pub fn scroll_left(&mut self, ss: &ScrollStep) -> io::Result<u16> {
        if self.wraps_line {
            return Ok(0);
        }
        let step = ss.to_numof_chars(self.pane_printable_width()?);
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
        if self.wraps_line {
            return Ok(0);
        }
        let step = ss.to_numof_chars(self.pane_printable_width()?);
        let max_line_width = self.max_width_of_visible_lines(self.range_of_visible_lines()?);
        let x = self.limit_right_x(self.cur_pos.0 + step, max_line_width)?;
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
        if !self.wraps_line {
            self.cur_pos.0 = 0;
        }
        Ok(self.cur_pos)
    }

    /// Go to tail of current line.
    pub fn goto_tail_of_line(&mut self) -> io::Result<(u16, u16)> {
        if !self.wraps_line {
            let max_line_width = self.max_width_of_visible_lines(self.range_of_visible_lines()?);
            self.cur_pos.0 = self.limit_right_x(max_line_width, max_line_width).unwrap();
        }
        Ok(self.cur_pos)
    }

    /// Go to specified absolute line number.
    /// Scroll so that the specified line appears at the top of the pane.
    pub fn goto_absolute_line(&mut self, lineno: u16) -> io::Result<u16> {
        let buf_height = self.linebuf.borrow().len() as u16;
        self.cur_pos.1 = if lineno >= buf_height {
            buf_height - 1
        } else {
            lineno
        };
        Ok(self.cur_pos.1)
    }

    pub fn goto_absolute_horizontal_offset(&mut self, offset: u16) -> io::Result<u16> {
        if !self.wraps_line {
            let max_line_width = self.max_width_of_visible_lines(self.range_of_visible_lines()?);
            self.cur_pos.0 = self.limit_right_x(offset, max_line_width)?;
        }
        Ok(self.cur_pos.0)
    }

    /// Set pane height.
    /// Pane height is limited by the actual terminal height.
    /// Return acutually set pane height.
    pub fn set_height(&mut self, n: u16) -> io::Result<u16> {
        let max = (*self.termsize_getter)()?.1 - Self::MESSAGE_BAR_HEIGHT;
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

    /// Set tab width.
    pub fn set_tab_width(&mut self, w: u16) {
        self.tab_width = w as usize;
    }

    /// Set wrap-line option.
    pub fn set_wrap(&mut self, b: bool) {
        self.wraps_line = b;
        if self.wraps_line {
            self.cur_pos.0 = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::OpenOptions;
    use std::io::BufWriter;

    macro_rules! gen_pane {
        ($w:expr) => {{
            let _w = BufWriter::new($w);
            Pane::new(Box::new(RefCell::new(_w)))
        }};
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
            "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
        ];
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 5);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        let pane_height = 4;
        let _ = pane.set_height(pane_height);
        // to update numof_semantic_flushed_lines
        let _ = pane.refresh();

        let stride_page = pane_height;
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

            assert_eq!(
                pane.scroll_down(&ScrollStep::HalfPage(1)).unwrap(),
                stride_hpage
            );
            assert_eq!(
                pane.scroll_down(&ScrollStep::HalfPage(2)).unwrap(),
                stride_hpage * 2
            );
            assert_eq!(pane.position(), (0, stride_hpage * 3));

            assert_eq!(
                pane.scroll_up(&ScrollStep::HalfPage(1)).unwrap(),
                stride_hpage
            );
            assert_eq!(
                pane.scroll_up(&ScrollStep::HalfPage(2)).unwrap(),
                stride_hpage * 2
            );
            assert_eq!(pane.position(), (0, 0));

            assert_eq!(pane.scroll_down(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(
                pane.scroll_down(&ScrollStep::Page(2)).unwrap(),
                stride_page * 2
            );
            assert_eq!(pane.position(), (0, stride_page * 3));

            assert_eq!(pane.scroll_up(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(
                pane.scroll_up(&ScrollStep::Page(2)).unwrap(),
                stride_page * 2
            );
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
        let t = ["1234567890123456789012345678901234567890"];
        let texts = gen_texts(&t);
        let (size, sizer) = gen_sizer(4, 2);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        let stride_page = size.0;
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

            assert_eq!(
                pane.scroll_right(&ScrollStep::HalfPage(1)).unwrap(),
                stride_hpage
            );
            assert_eq!(
                pane.scroll_right(&ScrollStep::HalfPage(2)).unwrap(),
                stride_hpage * 2
            );
            assert_eq!(pane.position(), (stride_hpage * 3, 0));

            assert_eq!(
                pane.scroll_left(&ScrollStep::HalfPage(1)).unwrap(),
                stride_hpage
            );
            assert_eq!(
                pane.scroll_left(&ScrollStep::HalfPage(2)).unwrap(),
                stride_hpage * 2
            );
            assert_eq!(pane.position(), (0, 0));

            assert_eq!(
                pane.scroll_right(&ScrollStep::Page(1)).unwrap(),
                stride_page
            );
            assert_eq!(
                pane.scroll_right(&ScrollStep::Page(2)).unwrap(),
                stride_page * 2
            );
            assert_eq!(pane.position(), (stride_page * 3, 0));

            assert_eq!(pane.scroll_left(&ScrollStep::Page(1)).unwrap(), stride_page);
            assert_eq!(
                pane.scroll_left(&ScrollStep::Page(2)).unwrap(),
                stride_page * 2
            );
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
                (
                    texts.borrow()[0].len() as u16 - size.0 + Pane::MARGIN_RIGHT_WIDTH,
                    0
                )
            );

            let (w, _) = pane.position();
            assert_eq!(pane.scroll_left(&ScrollStep::Page(10)).unwrap(), w,);
            assert_eq!(pane.position(), (0, 0));
        }

        let t = ["1234567890123456789012345678901234567890", ""];
        let texts = gen_texts(&t);
        pane.load(texts.clone());
        assert_eq!(pane.goto_absolute_line(1).unwrap(), 1);
        // now, draw "" only in terminal.
        assert_eq!(pane.scroll_right(&ScrollStep::Char(10)).unwrap(), 0);
    }

    #[test]
    fn test_goto_vertical_lines() {
        let t = [
            "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
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
        assert_eq!(pane.position(), (0, texts.borrow().len() as u16 - 1));

        // case: buffer height is less than pane height
        let (_, sizer) = gen_sizer(2, 10);
        let t = ["", "", "", ""];
        let texts = gen_texts(&t);
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        let pane_height = 8;
        let _ = pane.set_height(pane_height);
        assert_eq!(pane.goto_bottom_of_lines().unwrap(), (0, 0));
        assert_eq!(pane.position(), (0, 0));
        let t = ["", "", "", "", "", "", "", "", "", ""];
        let texts = gen_texts(&t);
        pane.load(texts.clone());
        assert_eq!(pane.goto_bottom_of_lines().unwrap(), (0, 2));
        assert_eq!(pane.position(), (0, 2));
    }

    #[test]
    fn test_goto_horizontal_line() {
        let t = ["1234567890123456789012345678901234567890"];
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
        assert_eq!(
            pane.set_height(size.1).unwrap(),
            size.1 - Pane::MESSAGE_BAR_HEIGHT
        );
        assert_eq!(
            pane.pane_size().unwrap(),
            (1, size.1 - Pane::MESSAGE_BAR_HEIGHT)
        );
        assert_eq!(
            pane.set_height(size.1 + 1).unwrap(),
            size.1 - Pane::MESSAGE_BAR_HEIGHT
        );
        assert_eq!(
            pane.pane_size().unwrap(),
            (1, size.1 - Pane::MESSAGE_BAR_HEIGHT)
        );

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
        assert_eq!(
            pane.increment_height(100).unwrap(),
            size.1 - Pane::MESSAGE_BAR_HEIGHT
        );
        assert_eq!(
            pane.pane_size().unwrap(),
            (1, size.1 - Pane::MESSAGE_BAR_HEIGHT)
        );
    }

    #[test]
    fn test_set_message() {
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.set_message(None);
        assert!(pane.message.is_empty());
        pane.set_message(Some("ThisIsTest".to_owned()));
        assert_eq!(pane.message, "ThisIsTest");
    }

    #[test]
    fn test_set_highlight_searcher() {}

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
        let a = ["1234567890123456789012345678901234567890"];
        let b = ["ABCD", "EFGH", "IJKL", "MNOP", "QRST", "UVWX", "YZ"];
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
            "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
        ];
        let nbuflines = t.len() as u16;
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 10);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        assert_eq!(pane.set_height(1).unwrap(), 1);
        assert_eq!(pane.limit_bottom_y().unwrap(), nbuflines - 1);
        assert_eq!(pane.set_height(4).unwrap(), 4);
        assert_eq!(pane.limit_bottom_y().unwrap(), nbuflines - 4);
    }

    #[test]
    fn test_range_of_visible_lines() {
        let t = [
            "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
        ];
        let nbuflines = t.len() as u16;
        let texts = gen_texts(&t);
        let (_, sizer) = gen_sizer(2, 10);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);
        assert_eq!(pane.set_height(5).unwrap(), 5);

        assert_eq!(pane.goto_absolute_line(0).unwrap(), 0);
        assert_eq!(pane.range_of_visible_lines().unwrap(), 0..5);
        assert_eq!(pane.goto_absolute_line(10).unwrap(), 10);
        assert_eq!(pane.range_of_visible_lines().unwrap(), 10..15);
        assert_eq!(
            pane.goto_absolute_line(nbuflines - 1).unwrap(),
            nbuflines - 1
        );
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
