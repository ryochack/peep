extern crate ctrlc;
extern crate termion;
extern crate termios;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};
use std::sync::mpsc;
use std::rc::Rc;
use std::cell::RefCell;
use std::thread::spawn;
use std::os::unix::io::AsRawFd;

use keybind;
use event::PeepEvent;
use filewatch;
use pane::{Pane, ScrollStep};
use search;
use term::{self, Block};

static FOLLOWING_MESSAGE: &'static str = "\x1b[7mWaiting for data... (interrupt to abort)\x1b[0m";

pub struct KeyEventHandler<'a> {
    istream: &'a mut Read,
    parser: &'a mut keybind::KeyParser,
}

impl<'a> KeyEventHandler<'a> {
    pub fn new(istream: &'a mut Read, parser: &'a mut keybind::KeyParser) -> Self {
        KeyEventHandler {
            istream: istream,
            parser: parser,
        }
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

pub struct App {
    pub show_linenumber: bool,
    pub nlines: u16,
    pub follow_mode: bool,
    file_path: String,
    seek_pos: u64,
    searcher: Rc<RefCell<search::Search>>,
    linebuf: Rc<RefCell<Vec<String>>>,
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

impl App {
    pub fn new() -> Self {
        // Prepare key input setting
        let ftty = File::open("/dev/tty").unwrap();
        let term_restorer = term::TermAttrSetter::new(ftty.as_raw_fd())
            .lflag(0, term::ICANON | term::ECHO)
            .set();

        App {
            show_linenumber: false,
            nlines: 5,
            follow_mode: false,
            file_path: String::new(),
            seek_pos: 0,
            searcher: Rc::new(RefCell::new(search::PlaneSearcher::new())),
            linebuf: Rc::new(RefCell::new(Vec::new())),
            term_restorer: Some(term_restorer),
        }
    }

    fn read_buffer(&mut self) -> io::Result<()> {
        if self.file_path == "-" {
            // read from stdin if pipe
            let stdin = io::stdin();
            if termion::is_tty(&stdin) {
                // stdin is tty. not pipe.
                return Err(io::Error::new(io::ErrorKind::NotFound, "no input"));
            }

            stdin.nonblocking();

            let stdinlock = stdin.lock();
            let mut lines_iter = stdinlock.lines();
            while let Some(Ok(v)) = lines_iter.next() {
                self.linebuf.borrow_mut().push(v);
            }

            stdin.blocking();
        } else {
            // read from file
            if let Ok(mut file) = File::open(&self.file_path) {
                self.seek_pos = file.seek(SeekFrom::Start(self.seek_pos))?;
                let mut bufreader = BufReader::new(file);
                for v in bufreader.lines().map(|v| v.unwrap()) {
                    // +1 is LR length
                    self.seek_pos += v.as_bytes().len() as u64 + 1;
                    self.linebuf.borrow_mut().push(v);
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
            }
        }
        Ok(())
    }

    pub fn run(&mut self, path: &str) -> io::Result<()> {
        self.file_path = path.to_owned();
        self.read_buffer()?;

        let writer = io::stdout();
        let writer = writer.lock();

        let (event_sender, event_receiver) = mpsc::channel();

        let sig_sender = event_sender.clone();
        // Ctrl-C handler
        ctrlc::set_handler(move || {
            // receive SIGINT
            sig_sender.send(PeepEvent::SigInt).unwrap();
        }).expect("Error setting ctrl-c handler");

        self.searcher = Rc::new(RefCell::new(search::RegexSearcher::new()));

        let mut pane = Pane::new(Box::new(RefCell::new(writer)));
        pane.load(self.linebuf.clone());
        pane.set_highlight_searcher(self.searcher.clone());
        pane.show_line_number(self.show_linenumber);
        pane.set_height(self.nlines)?;
        if self.follow_mode {
            pane.goto_bottom_of_lines()?;
            pane.set_message(Some(FOLLOWING_MESSAGE));
        }
        pane.refresh()?;

        let key_sender = event_sender.clone();
        // Key reading thread
        let _keythread = spawn(move || {
            let mut keyin = File::open("/dev/tty").unwrap();
            // let reader = io::stdin();
            // let mut reader = reader.lock();
            let mut kb = keybind::default::KeyBind::new();
            let mut keh = KeyEventHandler::new(&mut keyin, &mut kb);

            loop {
                match keh.read() {
                    Some(event) => {
                        key_sender.send(event.clone()).unwrap();
                    }
                    None => {}
                }
            }
        });

        // spawn inotifier thread for following mode
        filewatch::inotifier(&self.file_path, event_sender);

        // app loop
        loop {
            if let Ok(event) = event_receiver.recv() {
                if !self.follow_mode {
                    self.handle_normal(&event, &mut pane)?;
                    match event {
                        PeepEvent::Quit => {
                            break;
                        }
                        _ => {}
                    }
                } else {
                    self.handle_follow(&event, &mut pane)?;
                }
            }
        }
        Ok(())
    }

    fn handle_normal(&mut self, event: &PeepEvent, pane: &mut Pane) -> io::Result<()> {
        match event {
            &PeepEvent::MoveDown(n) => {
                pane.scroll_down(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveUp(n) => {
                pane.scroll_up(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveLeft(n) => {
                pane.scroll_left(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveRight(n) => {
                pane.scroll_right(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveDownHalfPages(n) => {
                pane.scroll_down(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveUpHalfPages(n) => {
                pane.scroll_up(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveLeftHalfPages(n) => {
                pane.scroll_left(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveRightHalfPages(n) => {
                pane.scroll_right(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveDownPages(n) => {
                pane.scroll_down(ScrollStep::Page(n))?;
                pane.refresh()?;
            }
            &PeepEvent::MoveUpPages(n) => {
                pane.scroll_up(ScrollStep::Page(n))?;
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
                pane.set_message(Some(&format!("/{}", s)));
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
                pane.set_message(None);
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
                pane.set_message(None);
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
                pane.set_message(None);
                pane.refresh()?;
            }
            PeepEvent::Message(s) => {
                pane.set_message(s.as_ref().map(|x| &**x));
                pane.refresh()?;
            }
            PeepEvent::Cancel => {
                pane.set_message(None);
                pane.show_highlight(false);
                pane.refresh()?;
            }
            PeepEvent::FollowMode => {
                // Enter follow mode
                self.follow_mode = true;
                // Relaod file
                self.read_buffer()?;
                pane.goto_bottom_of_lines()?;
                pane.set_message(Some(FOLLOWING_MESSAGE));
                pane.refresh()?;
            }
            PeepEvent::Quit => {
                pane.quit();
            }
            PeepEvent::SigInt => {
                // receive SIGINT
                // ring a bel
                pane.set_message(Some("\x07"));
                pane.refresh()?;
                pane.set_message(None);
            }
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
            PeepEvent::FileUpdated => {
                self.read_buffer()?;
                pane.goto_bottom_of_lines()?;
                pane.set_message(Some(FOLLOWING_MESSAGE));
                pane.refresh()?;
            }
            PeepEvent::SigInt => {
                // Leave follow mode
                self.follow_mode = false;
                pane.set_message(None);
                pane.refresh()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn search(
        &self,
        pos: (u16, u16),
    ) -> Option<(u16, u16)> {
        let searcher = self.searcher.borrow();
        let ref_linebuf = self.linebuf.borrow();
        for (i, line) in ref_linebuf[(pos.1 as usize)..].iter().enumerate() {
            if let Some(m) = searcher.find(line) {
                return Some((m.start() as u16, pos.1 + i as u16));
            }
        }
        None
    }

    fn search_rev(
        &self,
        pos: (u16, u16),
    ) -> Option<(u16, u16)> {
        let searcher = self.searcher.borrow();
        let ref_linebuf = self.linebuf.borrow();
        for (i, line) in ref_linebuf[0..(pos.1 as usize) + 1].iter().rev().enumerate() {
            if let Some(m) = searcher.find(line) {
                return Some((m.start() as u16, pos.1 - i as u16));
            }
        }
        None
    }
}

