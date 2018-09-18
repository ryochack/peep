extern crate ctrlc;
extern crate termion;

use std::cell::RefCell;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Cursor, Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::sync::mpsc;
use std::thread::spawn;

use event::PeepEvent;
use filewatch::{self, FileWatch};
use keybind;
use pane::{Pane, ScrollStep};
use search;
use term::{self, Block};

const DEFAULT_PANE_HEIGHT: u16 = 10;
const DEFAULT_TAB_WIDTH: u16 = 4;

const FOLLOWING_MESSAGE: &str = "\x1b[7mwaiting for data... (press 'F' to abort)\x1b[0m";
const FOLLOWING_HL_MESSAGE: &str = "\x1b[7mwaiting for data... \x1b[0m:";
const DEFAULT_POLL_TIMEOUT_MS: u64 = 200;

pub struct KeyEventHandler<'a> {
    istream: &'a mut Read,
    parser: &'a mut keybind::KeyParser,
}

impl<'a> KeyEventHandler<'a> {
    pub fn new(istream: &'a mut Read, parser: &'a mut keybind::KeyParser) -> Self {
        KeyEventHandler { istream, parser }
    }

    pub fn read(&mut self) -> Option<PeepEvent> {
        for b in self.istream.bytes().filter_map(|v| v.ok()) {
            let v = self.parser.parse(b as char);
            if v.is_some() {
                return v;
            }
        }
        None
    }
}

struct PipeReader {
    end_with_crlf: bool,
}

impl Default for PipeReader {
    fn default() -> Self {
        Self::new()
    }
}

impl PipeReader {
    pub fn new() -> Self {
        Self {
            end_with_crlf: true,
        }
    }

    /// chomp end of CRFL. Return whethre it was chomped or not.
    pub fn chomp(s: &mut String) -> bool {
        if s.ends_with('\n') {
            s.pop();
            if s.ends_with('\r') {
                s.pop();
            }
            true
        } else {
            false
        }
    }

    /// Read from pipe input.
    fn read(&mut self, linebuf: &mut Vec<String>, timeout_ms: u64) -> io::Result<()> {
        use std::os::unix::io::AsRawFd;
        use std::time::Duration;

        const INBUF_SIZE: usize = 8192;

        let mut tmo = timeout_ms;
        let stdin = io::stdin();

        let mut stdinwatcher = filewatch::StdinWatcher::new(stdin.as_raw_fd())?;
        let mut buf = [0u8; INBUF_SIZE];

        stdin.nonblocking();
        let mut stdinlock = stdin.lock();

        loop {
            let ready = stdinwatcher.watch(Some(Duration::from_millis(tmo)))?;
            if ready.is_none() {
                // time out
                break;
            }
            tmo = DEFAULT_POLL_TIMEOUT_MS;
            while let Ok(cap) = stdinlock.read(&mut buf) {
                if cap == 0 {
                    break;
                }

                let mut cursor = Cursor::new(&buf[..cap]);
                loop {
                    let mut line = String::new();
                    if let Ok(n) = cursor.read_line(&mut line) {
                        if n == 0 {
                            break;
                        }
                        let is_chmoped = PipeReader::chomp(&mut line);

                        if !self.end_with_crlf && linebuf.last_mut().is_some() {
                            linebuf.last_mut().unwrap().push_str(&line);
                        } else {
                            linebuf.push(line);
                        }
                        self.end_with_crlf = is_chmoped;
                    } else {
                        break;
                    }
                }
            }
            if ready.unwrap() {
                // is_hup()?
                break;
            }
        }

        stdin.blocking();
        Ok(())
    }
}

pub struct App {
    pub show_linenumber: bool,
    pub nlines: u16,
    pub follow_mode: bool,
    pub tab_width: u16,
    pub wraps_line: bool,
    typing_word: Option<String>,
    file_path: String,
    seek_pos: u64,
    searcher: Rc<RefCell<search::Search>>,
    linebuf: Rc<RefCell<Vec<String>>>,
    pipereader: PipeReader,
    // termios parameter moved from KeyEventHandler to App to detect Drop App.
    term_restorer: Option<term::TermAttrRestorer>,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(ref tr) = self.term_restorer {
            // Prepare key input setting
            let ftty = File::open("/dev/tty").unwrap();
            tr.restore(ftty.as_raw_fd());
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        // Prepare key input setting
        let ftty = File::open("/dev/tty").unwrap();
        let term_restorer = term::TermAttrSetter::new(ftty.as_raw_fd())
            .lflag(0, term::ICANON | term::ECHO)
            .set();

        App {
            show_linenumber: false,
            nlines: DEFAULT_PANE_HEIGHT,
            follow_mode: false,
            tab_width: DEFAULT_TAB_WIDTH,
            wraps_line: false,
            typing_word: None,
            file_path: String::new(),
            seek_pos: 0,
            searcher: Rc::new(RefCell::new(search::PlaneSearcher::new())),
            linebuf: Rc::new(RefCell::new(Vec::new())),
            pipereader: Default::default(),
            term_restorer: Some(term_restorer),
        }
    }

    fn read_buffer(&mut self, tmo_ms: u64) -> io::Result<()> {
        if self.file_path == "-" {
            // read from stdin if pipe
            if termion::is_tty(&io::stdin()) {
                // stdin is tty. not pipe.
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "Error. No input from stdin",
                ));
            }
            self.pipereader
                .read(&mut self.linebuf.borrow_mut(), tmo_ms)?;
        } else if let Ok(mut file) = File::open(&self.file_path) {
            // read from file
            self.seek_pos = file.seek(SeekFrom::Start(self.seek_pos))?;
            let mut bufreader = BufReader::new(file);
            for v in bufreader.lines().map(|v| v.unwrap()) {
                // +1 is LR length
                self.seek_pos += v.as_bytes().len() as u64 + 1;
                self.linebuf.borrow_mut().push(v);
            }
        } else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Error. {} is not found", self.file_path),
            ));
        }
        Ok(())
    }

    pub fn run(&mut self, path: &str) -> io::Result<()> {
        self.file_path = path.to_owned();
        self.read_buffer(1000)?;

        let writer = io::stdout();
        let writer = writer.lock();

        let (event_sender, event_receiver) = mpsc::channel();

        let sig_sender = event_sender.clone();
        // Ctrl-C handler
        ctrlc::set_handler(move || {
            // receive SIGINT
            sig_sender.send(PeepEvent::Quit).unwrap();
        }).expect("Error setting ctrl-c handler");

        self.searcher = Rc::new(RefCell::new(search::RegexSearcher::new("")));

        let mut pane = Pane::new(Box::new(RefCell::new(writer)));
        pane.load(self.linebuf.clone());
        pane.set_highlight_searcher(self.searcher.clone());
        pane.show_line_number(self.show_linenumber);
        pane.set_tab_width(self.tab_width);
        pane.set_wrap(self.wraps_line);
        pane.set_height(self.nlines)?;
        if self.follow_mode {
            pane.goto_bottom_of_lines()?;
        }
        pane.set_message(self.mode_default_message());
        pane.refresh()?;

        let key_sender = event_sender.clone();
        // Key reading thread
        let _keythread = spawn(move || {
            let mut keyin = File::open("/dev/tty").unwrap();
            let mut kb = keybind::default::KeyBind::new();
            let mut keh = KeyEventHandler::new(&mut keyin, &mut kb);

            loop {
                if let Some(event) = keh.read() {
                    key_sender.send(event.clone()).unwrap();
                }
            }
        });

        // spawn inotifier thread for following mode
        let file_path_to_watch = self.file_path.clone();
        let _fwthread = spawn(move || filewatch::file_watcher(&file_path_to_watch, &event_sender));

        // app loop
        loop {
            if let Ok(event) = event_receiver.recv() {
                if !self.follow_mode {
                    self.handle_normal(&event, &mut pane)?;
                } else {
                    self.handle_follow(&event, &mut pane)?;
                }

                if event == PeepEvent::Quit {
                    break;
                }
            }
        }
        Ok(())
    }

    fn mode_default_message(&self) -> Option<String> {
        if !self.follow_mode {
            // normal mode
            None
        } else if let Some(ref tw) = self.typing_word {
            // follow mode + highlighting
            Some(format!("{}/{}", FOLLOWING_HL_MESSAGE, tw))
        } else {
            // follow mode
            Some(FOLLOWING_MESSAGE.to_owned())
        }
    }

    fn handle_normal(&mut self, event: &PeepEvent, pane: &mut Pane) -> io::Result<()> {
        match event {
            &PeepEvent::MoveDown(n) => {
                pane.scroll_down(&ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveUp(n) => {
                pane.scroll_up(&ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveLeft(n) => {
                pane.scroll_left(&ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveRight(n) => {
                pane.scroll_right(&ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveDownHalfPages(n) => {
                pane.scroll_down(&ScrollStep::HalfPage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveUpHalfPages(n) => {
                pane.scroll_up(&ScrollStep::HalfPage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveLeftHalfPages(n) => {
                pane.scroll_left(&ScrollStep::HalfPage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveRightHalfPages(n) => {
                pane.scroll_right(&ScrollStep::HalfPage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveDownPages(n) => {
                pane.scroll_down(&ScrollStep::Page(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveUpPages(n) => {
                pane.scroll_up(&ScrollStep::Page(n))?;
                pane.refresh()?;
            }
            PeepEvent::MoveToHeadOfLine => {
                pane.goto_head_of_line()?;
                pane.refresh()?;
            }
            PeepEvent::MoveToEndOfLine => {
                pane.goto_tail_of_line()?;
                pane.refresh()?;
            }
            PeepEvent::MoveToTopOfLines => {
                pane.goto_top_of_lines()?;
                pane.refresh()?;
            }
            PeepEvent::MoveToBottomOfLines => {
                pane.goto_bottom_of_lines()?;
                pane.refresh()?;
            }
            &PeepEvent::MoveToLineNumber(n) => {
                pane.goto_absolute_line(n)?;
                pane.refresh()?;
            }
            PeepEvent::ToggleLineNumberPrinting => {
                self.show_linenumber = !self.show_linenumber;
                pane.show_line_number(self.show_linenumber);
                pane.refresh()?;
            }
            PeepEvent::ToggleLineWraps => {
                self.wraps_line = !self.wraps_line;
                pane.set_wrap(self.wraps_line);
                pane.refresh()?;
            }
            &PeepEvent::IncrementLines(n) => {
                pane.increment_height(n)?;
                pane.refresh()?;
            }
            &PeepEvent::DecrementLines(n) => {
                pane.decrement_height(n)?;
                pane.refresh()?;
            }
            &PeepEvent::SetNumOfLines(n) => {
                pane.set_height(n)?;
                pane.refresh()?;
            }
            PeepEvent::SearchIncremental(s) => {
                self.typing_word = Some(s.to_owned());
                pane.set_message(Some(format!("/{}", s)));
                if s.is_empty() {
                    let _ = self.searcher.borrow_mut().set_pattern(&s);
                    pane.show_highlight(false);
                } else {
                    let _ = self.searcher.borrow_mut().set_pattern(&s);
                    if let Some(pos) = self.search(pane.position()) {
                        pane.goto_absolute_line(pos.1)?;
                    }
                    pane.show_highlight(true);
                }
                pane.refresh()?;
            }
            PeepEvent::SearchTrigger => {
                self.typing_word = None;
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::SearchNext => {
                let cur_pos = pane.position();
                let next_pos = (
                    cur_pos.0,
                    if cur_pos.1 == self.linebuf.borrow().len() as u16 - 1 {
                        self.linebuf.borrow().len() as u16 - 1
                    } else {
                        cur_pos.1 + 1
                    },
                );

                if self.searcher.borrow().as_str().is_empty() {
                    pane.show_highlight(false);
                } else {
                    if let Some(pos) = self.search(next_pos) {
                        pane.goto_absolute_line(pos.1)?;
                    }
                    pane.show_highlight(true);
                }
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::SearchPrev => {
                let cur_pos = pane.position();
                let next_pos = (cur_pos.0, if cur_pos.1 == 0 { 0 } else { cur_pos.1 - 1 });

                if self.searcher.borrow().as_str().is_empty() {
                    pane.show_highlight(false);
                } else {
                    if let Some(pos) = self.search_rev(next_pos) {
                        pane.goto_absolute_line(pos.1)?;
                    }
                    pane.show_highlight(true);
                }
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::Message(s) => {
                pane.set_message(s.to_owned());
                pane.refresh()?;
            }
            PeepEvent::Cancel => {
                self.typing_word = None;
                pane.set_message(self.mode_default_message());
                pane.show_highlight(false);
                pane.refresh()?;
            }
            PeepEvent::FollowMode => {
                // Enter follow mode
                self.follow_mode = true;
                // Reload file
                self.read_buffer(DEFAULT_POLL_TIMEOUT_MS)?;
                pane.goto_bottom_of_lines()?;
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::Quit => {
                pane.quit();
            }
            PeepEvent::SigInt => {}
            _ => {}
        }
        Ok(())
    }

    fn handle_follow(&mut self, event: &PeepEvent, pane: &mut Pane) -> io::Result<()> {
        match event {
            &PeepEvent::ToggleLineNumberPrinting => {
                self.show_linenumber = !self.show_linenumber;
                pane.show_line_number(self.show_linenumber);
                pane.refresh()?;
            }
            &PeepEvent::IncrementLines(n) => {
                pane.increment_height(n)?;
                pane.refresh()?;
            }
            &PeepEvent::DecrementLines(n) => {
                pane.decrement_height(n)?;
                pane.refresh()?;
            }
            &PeepEvent::SetNumOfLines(n) => {
                pane.set_height(n)?;
                pane.refresh()?;
            }
            PeepEvent::SearchIncremental(s) => {
                self.typing_word = Some(s.to_owned());
                pane.set_message(Some(format!("{}/{}", FOLLOWING_HL_MESSAGE, s)));
                if s.is_empty() {
                    let _ = self.searcher.borrow_mut().set_pattern(&s);
                    pane.show_highlight(false);
                } else {
                    let _ = self.searcher.borrow_mut().set_pattern(&s);
                    pane.show_highlight(true);
                }
                pane.refresh()?;
            }
            PeepEvent::SearchTrigger => {
                self.typing_word = None;
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::Cancel => {
                self.typing_word = None;
                pane.set_message(self.mode_default_message());
                pane.show_highlight(false);
                pane.refresh()?;
            }
            PeepEvent::FileUpdated => {
                self.read_buffer(DEFAULT_POLL_TIMEOUT_MS)?;
                pane.goto_bottom_of_lines()?;
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::FollowMode => {
                // Leave follow mode
                self.follow_mode = false;
                pane.set_message(self.mode_default_message());
                pane.refresh()?;
            }
            PeepEvent::Quit => {
                pane.quit();
            }
            PeepEvent::SigInt => {}
            _ => {}
        }
        Ok(())
    }

    fn search(&self, pos: (u16, u16)) -> Option<(u16, u16)> {
        let searcher = self.searcher.borrow();
        let ref_linebuf = self.linebuf.borrow();
        for (i, line) in ref_linebuf[(pos.1 as usize)..].iter().enumerate() {
            if let Some(m) = searcher.find(line) {
                return Some((m.start() as u16, pos.1 + i as u16));
            }
        }
        None
    }

    fn search_rev(&self, pos: (u16, u16)) -> Option<(u16, u16)> {
        let searcher = self.searcher.borrow();
        let ref_linebuf = self.linebuf.borrow();
        for (i, line) in ref_linebuf[0..=(pos.1 as usize)].iter().rev().enumerate() {
            if let Some(m) = searcher.find(line) {
                return Some((m.start() as u16, pos.1 - i as u16));
            }
        }
        None
    }
}
