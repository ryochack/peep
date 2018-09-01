//! Pane module

use csi::cursor_ext;
use search::{NullSearcher, Search};
use std::cell::RefCell;
use std::cmp;
use std::io::{self, Write};
use std::ops;
use std::rc::Rc;
use termion;

const DEFAULT_PANE_HEIGHT: u16 = 5;

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

    pub fn new<W: 'a + Write>(w: Box<RefCell<W>>) -> Self {
        let pane = Pane {
            linebuf: Rc::new(RefCell::new(Vec::new())),
            writer: w,
            height: DEFAULT_PANE_HEIGHT,
            numof_flushed_lines: DEFAULT_PANE_HEIGHT,
            cur_pos: (0, 0),
            show_linenumber: false,
            show_highlight: false,
            hlsearcher: Rc::new(RefCell::new(NullSearcher::new())),
            // _highlight_word: "".to_owned(),
            // highlight_re: Regex::new("").unwrap(),
            message: "".to_owned(),
            termsize_getter: Box::new(termion::terminal_size),
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

    /// Load text buffer and reset position.
    /// After this function is called, current pane postion is set to (0, 0).
    pub fn load(&mut self, buf: Rc<RefCell<Vec<String>>>) {
        self.linebuf = buf;
        self.cur_pos = (0, 0);
    }

    fn flush(&self) {
        self.writer.borrow_mut().flush().unwrap();
    }

    fn sweep(&self) {
        let mut s = String::new();
        s.push_str(&format!("{}", cursor_ext::HorizontalAbsolute(1)));
        for _ in 0..self.numof_flushed_lines {
            s.push_str(&format!("{}", termion::clear::CurrentLine));
            s.push_str("\n");
        }
        s.push_str(&format!("{}", termion::clear::CurrentLine));
        if self.numof_flushed_lines > 0 {
            s.push_str(&format!(
                "{}",
                cursor_ext::PreviousLine(self.numof_flushed_lines as u16)
            ));
        }
        self.writer.borrow_mut().write_all(s.as_bytes()).unwrap();
    }

    /// Highlight line with the highlight word
    fn highlight_words(&self, raw: &str) -> String {
        let mut line = String::new();
        let mut i = 0;
        for m in self.hlsearcher.borrow().find_iter(raw) {
            let hl = (m.start(), m.end());
            line.push_str(raw.get(i..hl.0).unwrap_or("#"));
            line.push_str(&format!("{}", termion::style::Invert));
            line.push_str(raw.get(hl.0..hl.1).unwrap_or("#"));
            line.push_str(&format!("{}", termion::style::Reset));
            i = hl.1;
        }
        if i < raw.len() {
            line.push_str(raw.get(i..).unwrap_or("#"));
        }
        line
    }

    //    ....[[[....]]]....
    // [:  1   2  3   4
    //  1: ^
    //  2:   ^
    //  3:        ^           (highlighting)
    //  4:              ^
    //
    //    ....[[[....]]]....
    // ]:     1  2   3   4
    //  1:   ^
    //  2:       ^            (highlighting)
    //  3:              ^
    // # ORIGINAL
    // "impl<'a>"
    //   ~~       :Hilight
    //
    // # HIGHLIGHT STR
    // [0:'i'][1:'\u{1b}'][2:'['][3:'7'][4:'m'][5:'m'][6:'p'][7:'\u{1b}'][8:'['][9:'m'][10:'l'][11:'<'][12:'\''][13:'a'][14:'>']
    // # LOGIC INDEX
    // [0:'i'][5:('m',HL)][6:('p',HL)][10:'l'][11:'<'][12:'\''][13:'a'][14:'>']
    //
    /// Generate indices that ignore CSI sequence
    /// This function return tuple (index, is_highlighting)
    fn gen_logic_indices(raw: &str) -> Vec<(usize, bool)> {
        let mut nongraphic = String::new();
        let mut pat = format!("{}", termion::style::Invert);
        let mut highlighting = false;

        raw.char_indices()
            .filter_map(move |(i, c)| {
                if !nongraphic.is_empty() {
                    // CSI sequence mode
                    nongraphic.push(c);
                    if nongraphic == pat {
                        // Match CSI sequence and leave CSI sequence mode
                        nongraphic.clear();
                        highlighting = !highlighting;
                        pat = if highlighting {
                            format!("{}", termion::style::Reset)
                        } else {
                            format!("{}", termion::style::Invert)
                        };
                    }
                    None
                } else if c == '\x1B' {
                    // Enter CSI sequence mode
                    nongraphic.push(c);
                    None
                } else {
                    Some((i, highlighting))
                }
            }).collect::<Vec<(usize, bool)>>()
    }

    /// Trim with logical length to fit pane width.
    /// This function consider CSI sequence.
    fn trim(raw: &str, range: ops::Range<usize>) -> String {
        if raw.is_empty() || raw.len() < range.start {
            return raw[0..0].to_owned();
        }

        let logic_indices = Pane::gen_logic_indices(raw);

        if logic_indices.is_empty() {
            return raw.to_owned();
        }
        if logic_indices.len() <= range.start {
            return raw[0..0].to_owned();
        }

        let s = logic_indices[range.start];
        let mut e = *logic_indices
            .get(range.end - 1)
            .unwrap_or_else(|| logic_indices.last().unwrap());
        let mut trimed = String::new();
        if s.1 {
            // if start with highlight, push CSI invert to head
            trimed.push_str(&format!("{}", termion::style::Invert));
        }

        // If end index is not UTF-8 code char boundary, search next char to find boundary.
        e.0 += 1;
        while !raw.is_char_boundary(e.0) { e.0 += 1; }

        trimed.push_str(raw.get(s.0..e.0).unwrap());
        if e.1 {
            // if end with highlight, push CSI Reset to end
            trimed.push_str(&format!("{}", termion::style::Reset));
        }
        trimed
    }

    // Decorate line
    fn decorate(&self, raw: &str, line_number: u16) -> String {
        let hlline = if self.show_highlight {
            self.highlight_words(raw)
        } else {
            raw.to_owned()
        };

        // right margin is for extend marks
        let right_blank_margin: usize = 2;
        let mut range = (
            self.cur_pos.0 as usize,
            (self.cur_pos.0 + self.size().unwrap().0) as usize - right_blank_margin,
        );

        // add line number
        let ln = if self.show_linenumber {
            let linenumber_space = 4;
            range.1 -= linenumber_space;
            format!("{:>4}", line_number + 1)
        } else {
            String::new()
        };

        // add extend marks
        let sol = if range.0 > 0 {
            format!("{}", ExtendMark('+'))
        } else {
            " ".to_owned()
        };

        // trimed line
        let trimed = Pane::trim(&hlline, range.0..cmp::min(raw.len(), range.1)).to_owned();

        // add extend marks
        let eol = if raw.len() > range.1 {
            format!("{}", ExtendMark('+'))
        } else {
            format!("{}", termion::style::Reset)
        };

        format!("{}{}{}{}", ln, sol, trimed, eol)
    }

    pub fn refresh(&mut self) -> io::Result<()> {
        // content lines
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
        self.sweep();
        self.writer
            .borrow_mut()
            .write_all(block.as_bytes())
            .unwrap();
        self.flush();
        self.numof_flushed_lines = (buf_range.end - buf_range.start) as u16;
        Ok(())
    }

    pub fn quit(&self) {
        write!(self.writer.borrow_mut(), "{}", termion::clear::CurrentLine).unwrap();
        writeln!(self.writer.borrow_mut()).unwrap();
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
        let ph = self.size().unwrap_or((1, 1)).1;
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

    /// Return (width, height)
    pub fn size(&self) -> io::Result<(u16, u16)> {
        (*self.termsize_getter)().map(|(tw, th)| (tw, cmp::min(th, self.height)))
    }

    /// Return (x, y)
    pub fn position(&self) -> (u16, u16) {
        self.cur_pos
    }

    /// Return the end of y that is considered buffer lines and window size
    fn limit_bottom_y(&self) -> io::Result<u16> {
        let linebuf_height = self.linebuf.borrow().len() as u16;
        let pane_height = self.size()?.1;

        Ok(if linebuf_height > pane_height {
            linebuf_height - pane_height
        } else {
            linebuf_height
        })
    }

    /// Return range of visible lines from current line to buffer line end or bottom of pane.
    fn range_of_visible_lines(&self) -> io::Result<ops::Range<usize>> {
        let pane_height = self.size()?.1 as usize;
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
        let pane_width = self.size()?.0;

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
    pub fn scroll_down(&mut self, ss: &ScrollStep) -> io::Result<u16> {
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
    pub fn scroll_left(&mut self, ss: &ScrollStep) -> io::Result<u16> {
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
    pub fn scroll_right(&mut self, ss: &ScrollStep) -> io::Result<u16> {
        let step = ss.to_numof_chars(self.size()?.0);
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
        let max = (*self.termsize_getter)()?.1;
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
        let (size, sizer) = gen_sizer(2, 4);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        let stride_page  = size.1;
        let stride_hpage = size.1 / 2;

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
                texts.borrow().len() as u16 - size.1
            );
            assert_eq!(
                pane.position(),
                (0, texts.borrow().len() as u16 - size.1)
            );

            assert_eq!(
                pane.scroll_up(&ScrollStep::Page(10)).unwrap(),
                texts.borrow().len() as u16 - size.1
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
        let (size, sizer) = gen_sizer(2, 4);
        let mut pane = gen_pane!(OpenOptions::new().write(true).open("/dev/null").unwrap());
        pane.load(texts.clone());
        pane.replace_termsize_getter(sizer);

        pane.scroll_right(&ScrollStep::Char(1)).unwrap();
        assert_eq!(
            pane.goto_bottom_of_lines().unwrap(),
            (0, texts.borrow().len() as u16 - size.1)
        );
        assert_eq!(
            pane.position(),
            (0, texts.borrow().len() as u16 - size.1)
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
        assert_eq!(pane.size().unwrap(), (1, 5));
        assert_eq!(pane.set_height(0).unwrap(), 1);
        assert_eq!(pane.size().unwrap(), (1, 1));
        assert_eq!(pane.set_height(size.1).unwrap(), size.1);
        assert_eq!(pane.size().unwrap(), (1, size.1));
        assert_eq!(pane.set_height(size.1 + 1).unwrap(), size.1);
        assert_eq!(pane.size().unwrap(), (1, size.1));

        assert_eq!(pane.set_height(5).unwrap(), 5);
        assert_eq!(pane.size().unwrap(), (1, 5));
        assert_eq!(pane.decrement_height(1).unwrap(), 4);
        assert_eq!(pane.size().unwrap(), (1, 4));
        assert_eq!(pane.decrement_height(3).unwrap(), 1);
        assert_eq!(pane.size().unwrap(), (1, 1));
        assert_eq!(pane.decrement_height(100).unwrap(), 1);
        assert_eq!(pane.size().unwrap(), (1, 1));

        assert_eq!(pane.set_height(5).unwrap(), 5);
        assert_eq!(pane.size().unwrap(), (1, 5));
        assert_eq!(pane.increment_height(1).unwrap(), 6);
        assert_eq!(pane.size().unwrap(), (1, 6));
        assert_eq!(pane.increment_height(3).unwrap(), 9);
        assert_eq!(pane.size().unwrap(), (1, 9));
        assert_eq!(pane.increment_height(100).unwrap(), size.1);
        assert_eq!(pane.size().unwrap(), (1, size.1));
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
    fn test_gen_logic_indices() {
        let csi_contained = "this is \x1B[7mHIGHLIGHT\x1B[m word";
        let expects: Vec<(usize, bool)> = vec![
            (0, false), // 't'
            (1, false), // 'h'
            (2, false), // 'i'
            (3, false), // 's'
            (4, false), // ' '
            (5, false), // 'i'
            (6, false), // 's'
            (7, false), // ' '
            // skip "\x1B[7m"
            (12, true), // 'H'
            (13, true), // 'I'
            (14, true), // 'G'
            (15, true), // 'H'
            (16, true), // 'L'
            (17, true), // 'I'
            (18, true), // 'G'
            (19, true), // 'H'
            (20, true), // 'T'
            // skip "\x1B[m"
            (24, false), // ' '
            (25, false), // 'w'
            (26, false), // 'o'
            (27, false), // 'r'
            (28, false), // 'd'
        ];
        assert_eq!(
            Pane::gen_logic_indices(&csi_contained),
            expects
        );

        let csi_not_contained = "this is normal word";
        let expects: Vec<(usize, bool)> = vec![
            (0, false), // 't'
            (1, false), // 'h'
            (2, false), // 'i'
            (3, false), // 's'
            (4, false), // ' '
            (5, false), // 'i'
            (6, false), // 's'
            (7, false), // ' '
            (8, false), // 'n'
            (9, false), // 'o'
            (10, false), // 'r'
            (11, false), // 'm'
            (12, false), // 'a'
            (13, false), // 'l'
            (14, false), // ' '
            (15, false), // 'w'
            (16, false), // 'o'
            (17, false), // 'r'
            (18, false), // 'd'
        ];
        assert_eq!(
            Pane::gen_logic_indices(&csi_not_contained),
            expects
        );
    }

    #[test]
    fn test_trim() {
        let raw = "0123456789ABCEDF";
        assert_eq!(Pane::trim(&raw, 0..10), "0123456789");
        assert_eq!(Pane::trim(&raw, 4..12), "456789AB");
        assert_eq!(Pane::trim(&raw, 0..raw.len()), raw);
        assert_eq!(Pane::trim(&raw, 0..raw.len() + 10), raw);
        assert_eq!(Pane::trim(&raw, raw.len() + 1..raw.len() + 10), "");

        let empty = "";
        assert_eq!(Pane::trim(&empty, 0..10), "");
        assert_eq!(Pane::trim(&empty, 4..12), "");

        let csi_contained = "0123\x1B[7m4567\x1B[m89";
        assert_eq!(Pane::trim(&csi_contained, 0..10), "0123\x1B[7m4567\x1B[m89");
        assert_eq!(Pane::trim(&csi_contained, 0..4), "0123");
        assert_eq!(Pane::trim(&csi_contained, 0..5), "0123\x1B[7m4\x1B[m");
        assert_eq!(Pane::trim(&csi_contained, 4..8), "\x1B[7m4567\x1B[m");
        assert_eq!(Pane::trim(&csi_contained, 3..9), "3\x1B[7m4567\x1B[m8");
        assert_eq!(Pane::trim(&csi_contained, 7..8), "\x1B[7m7\x1B[m");
        assert_eq!(Pane::trim(&csi_contained, 7..9), "\x1B[7m7\x1B[m8");
        assert_eq!(Pane::trim(&csi_contained, 8..10), "89");

        let multibyte_str = "it becomes the new default for subse‐";
        println!("length = {}", multibyte_str.len());
        assert_eq!(
            Pane::trim(&multibyte_str, 0..multibyte_str.len()),
            "it becomes the new default for subse‐"
        );
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
