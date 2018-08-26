extern crate ctrlc;
extern crate termion;
extern crate termios;

use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::sync::mpsc;
use std::rc::Rc;
use std::cell::RefCell;
use std::thread::spawn;

use keybind;
use event::PeepEvent;
use pane::{Pane, ScrollStep};
use search;
use tty;


pub struct KeyEventHandler<'a> {
    istream: &'a mut Read,
    parser: &'a mut keybind::KeyParser,
    oldstat: Box<termios::Termios>,
}

impl<'a> Drop for KeyEventHandler<'a> {
    fn drop(&mut self) {
        tty::echo_on(&*self.oldstat);
    }
}

impl<'a> KeyEventHandler<'a> {
    pub fn new(istream: &'a mut Read, parser: &'a mut keybind::KeyParser) -> Self {
        KeyEventHandler {
            istream: istream,
            parser: parser,
            oldstat: Box::new(tty::echo_off()),
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
    file_path: String,
    searcher: Rc<RefCell<search::Search>>,
    linebuf: Rc<RefCell<Vec<String>>>,
}

impl App {
    pub fn new() -> Self {
        App {
            show_linenumber: false,
            nlines: 5,
            file_path: String::new(),
            searcher: Rc::new(RefCell::new(search::PlaneSearcher::new())),
            linebuf: Rc::new(RefCell::new(Vec::new())),
        }
    }

    fn read_buffer(&mut self) -> io::Result<()> {
        // let mut linebuf: Vec<String> = Vec::new();
        if self.file_path == "-" {
            // read from stdin if pipe
            let inp = io::stdin();
            if termion::is_tty(&inp) {
                // stdin is tty. not pipe.
                return Err(io::Error::new(io::ErrorKind::NotFound, "no input"));
            }
            let inp = inp.lock();
            for v in inp.lines().map(|v| v.unwrap()) {
                self.linebuf.borrow_mut().push(v);
            }
        } else {
            // read from file
            if let Ok(file) = File::open(&self.file_path) {
                let mut bufreader = BufReader::new(file);
                for v in bufreader.lines().map(|v| v.unwrap()) {
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

        // to input key from stdin when pipe is enable.
        tty::switch_stdin_to_tty();

        let writer = io::stdout();
        let writer = writer.lock();

        let (sender, reciever) = mpsc::channel();
        let sig_sender = sender.clone();

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
        pane.refresh()?;

        // Key reading thread
        let _keythread = spawn(move || {
            let reader = io::stdin();
            let mut reader = reader.lock();
            let mut kb = keybind::default::KeyBind::new();
            let mut keh = KeyEventHandler::new(&mut reader, &mut kb);

            loop {
                match keh.read() {
                    Some(keyop) => {
                        sender.send(keyop.clone()).unwrap();
                        if keyop == PeepEvent::Quit {
                            break;
                        }
                    }
                    None => {}
                }
            }
        });

        // app loop
        loop {
            if let Ok(keyop) = reciever.recv() {
                if keyop == PeepEvent::SigInt {
                    // receive SIGINT
                    // ring a bel
                    pane.set_message(Some("\x07"));
                    pane.refresh()?;
                    pane.set_message(None);
                } else {
                    self.handle(&keyop, &mut pane)?;
                    if keyop == PeepEvent::Quit {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn handle(&mut self, keyop: &PeepEvent, pane: &mut Pane) -> io::Result<()> {
        match keyop {
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
            PeepEvent::Quit => {
                pane.quit();
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
