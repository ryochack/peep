extern crate ctrlc;
extern crate termion;

use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::sync::mpsc;
use std::thread::spawn;

use keybind;
use keyevt::{KeyEventHandler, KeyOp};
use pane::{Pane, ScrollStep};
use tty;

#[derive(Debug)]
pub struct App {
    pub show_linenumber: bool,
    pub nlines: u16,
    file_path: String,
}

impl App {
    pub fn new() -> Self {
        App {
            show_linenumber: false,
            nlines: 5,
            file_path: String::new(),
        }
    }

    fn read_buffer(&mut self) -> io::Result<(Vec<String>)> {
        let mut linebuf: Vec<String> = Vec::new();
        if self.file_path == "-" {
            // read from stdin if pipe
            let inp = io::stdin();
            if termion::is_tty(&inp) {
                // stdin is tty. not pipe.
                return Err(io::Error::new(io::ErrorKind::NotFound, "no input"));
            }
            let inp = inp.lock();
            for v in inp.lines().map(|v| v.unwrap()) {
                linebuf.push(v);
            }
        } else {
            // read from file
            if let Ok(file) = File::open(&self.file_path) {
                let mut bufreader = BufReader::new(file);
                for v in bufreader.lines().map(|v| v.unwrap()) {
                    linebuf.push(v);
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "not found"));
            }
        }
        Ok(linebuf)
    }

    pub fn run(&mut self, path: &str) -> io::Result<()> {
        self.file_path = path.to_owned();
        let linebuf = self.read_buffer()?;

        // to input key from stdin when pipe is enable.
        tty::switch_stdin_to_tty();

        let writer = io::stdout();
        let mut writer = writer.lock();

        let (sender, reciever) = mpsc::channel();
        let sig_sender = sender.clone();

        // Ctrl-C handler
        ctrlc::set_handler(move || {
            // receive SIGINT
            sig_sender.send(KeyOp::SigInt).unwrap();
        }).expect("Error setting ctrl-c handler");

        let mut pane = Pane::new(&mut writer);
        pane.load(&linebuf);
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
                        if keyop == KeyOp::Quit {
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
                if keyop == KeyOp::SigInt {
                    // receive SIGINT
                    // ring a bel
                    pane.set_message(Some("\x07"));
                    pane.refresh()?;
                    pane.set_message(None);
                } else {
                    self.handle(&keyop, &mut pane, &linebuf)?;
                    if keyop == KeyOp::Quit {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    fn handle(&mut self, keyop: &KeyOp, pane: &mut Pane, linebuf: &[String]) -> io::Result<()> {
        match keyop {
            &KeyOp::MoveDown(n) => {
                pane.scroll_down(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveUp(n) => {
                pane.scroll_up(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveLeft(n) => {
                pane.scroll_left(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveRight(n) => {
                pane.scroll_right(ScrollStep::Char(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveDownHalfPages(n) => {
                pane.scroll_down(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveUpHalfPages(n) => {
                pane.scroll_up(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveLeftHalfPages(n) => {
                pane.scroll_left(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveRightHalfPages(n) => {
                pane.scroll_right(ScrollStep::Halfpage(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveDownPages(n) => {
                pane.scroll_down(ScrollStep::Page(n))?;
                pane.refresh()?;
            }
            &KeyOp::MoveUpPages(n) => {
                pane.scroll_up(ScrollStep::Page(n))?;
                pane.refresh()?;
            }
            KeyOp::MoveToHeadOfLine => {
                pane.goto_head_of_line()?;
                pane.refresh()?;
            }
            KeyOp::MoveToEndOfLine => {
                pane.goto_tail_of_line()?;
                pane.refresh()?;
            }
            KeyOp::MoveToTopOfLines => {
                pane.goto_top_of_lines()?;
                pane.refresh()?;
            }
            KeyOp::MoveToBottomOfLines => {
                pane.goto_bottom_of_lines()?;
                pane.refresh()?;
            }
            &KeyOp::MoveToLineNumber(n) => {
                pane.goto_absolute_line(n)?;
                pane.refresh()?;
            }
            &KeyOp::ToggleLineNumberPrinting => {
                self.show_linenumber = !self.show_linenumber;
                pane.show_line_number(self.show_linenumber);
                pane.refresh()?;
            }
            &KeyOp::IncrementLines(n) => {
                pane.increment_height(n)?;
                pane.refresh()?;
            }
            &KeyOp::DecrementLines(n) => {
                pane.decrement_height(n)?;
                pane.refresh()?;
            }
            &KeyOp::SetNumOfLines(n) => {
                pane.set_height(n)?;
                pane.refresh()?;
            }
            KeyOp::SearchIncremental(s) => {
                pane.set_message(Some(&format!("/{}", s)));
                // pane.set_highlight_word(Some(&s));
                let _ = pane.set_highlight_regex(Some(&s));

                // if let Some(pos) = self.search_by_str(linebuf, pane.position(), &s, false) {
                //     pane.goto_absolute_line(pos.1)?;
                // }

                let hlpat = pane.ref_highlight_regex().to_owned();
                if let Some(pos) = self.search_by_regex(linebuf, pane.position(), &hlpat, false) {
                    pane.goto_absolute_line(pos.1)?;
                }

                pane.refresh()?;
            }
            KeyOp::SearchTrigger => {
                pane.set_message(None);
                pane.refresh()?;
            }
            KeyOp::SearchNext => {
                let cur_pos = pane.position();
                let next_pos = (
                    cur_pos.0,
                    if cur_pos.1 == linebuf.len() as u16 - 1 {
                        linebuf.len() as u16 - 1
                    } else {
                        cur_pos.1 + 1
                    },
                );

                let hlpat = pane.ref_highlight_regex().to_owned();
                if let Some(pos) = self.search_by_regex(linebuf, next_pos, &hlpat, false) {
                    pane.goto_absolute_line(pos.1)?;
                }

                // let hlpat = pane.ref_highlight_word().unwrap_or("").to_owned();
                // if let Some(pos) = self.search_by_str(linebuf, next_pos, &hlpat, false) {
                //     pane.goto_absolute_line(pos.1)?;
                // }

                pane.set_message(None);
                pane.refresh()?;
            }
            KeyOp::SearchPrev => {
                let cur_pos = pane.position();
                let next_pos = (cur_pos.0, if cur_pos.1 == 0 { 0 } else { cur_pos.1 - 1 });

                let hlpat = pane.ref_highlight_regex().to_owned();
                if let Some(pos) = self.search_by_regex(linebuf, next_pos, &hlpat, true) {
                    pane.goto_absolute_line(pos.1)?;
                }

                // let hlpat = pane.ref_highlight_word().unwrap_or("").to_owned();
                // if let Some(pos) = self.search_by_str(linebuf, next_pos, &hlpat, true) {
                //     pane.goto_absolute_line(pos.1)?;
                // }

                pane.set_message(None);
                pane.refresh()?;
            }
            KeyOp::Message(s) => {
                pane.set_message(s.as_ref().map(|x| &**x));
                pane.refresh()?;
            }
            KeyOp::Cancel => {
                pane.set_message(None);
                pane.set_highlight_word(None);
                pane.refresh()?;
            }
            KeyOp::Quit => {
                pane.quit();
            }
            _ => {}
        }
        Ok(())
    }

    #[allow(dead_code)]
    /// return (x, y)
    fn search_by_str(
        &self,
        buffer: &[String],
        pos: (u16, u16),
        pat: &str,
        reverse: bool,
    ) -> Option<(u16, u16)> {
        if pat.is_empty() {
            return None;
        }
        if !reverse {
            for (i, line) in buffer[(pos.1 as usize)..].iter().enumerate() {
                if let Some(c) = line.find(pat) {
                    return Some((c as u16, pos.1 + i as u16));
                }
            }
        } else {
            for (i, line) in buffer[0..(pos.1 as usize) + 1].iter().rev().enumerate() {
                if let Some(c) = line.find(pat) {
                    // hit!
                    return Some((c as u16, pos.1 - i as u16));
                }
            }
        }
        None
    }

    /// return (x, y)
    fn search_by_regex(
        &self,
        buffer: &[String],
        pos: (u16, u16),
        re: &Regex,
        reverse: bool,
    ) -> Option<(u16, u16)> {
        if !reverse {
            for (i, line) in buffer[(pos.1 as usize)..].iter().enumerate() {
                if let Some(m) = re.find(&line) {
                    return Some((m.start() as u16, pos.1 + i as u16));
                }
            }
        } else {
            for (i, line) in buffer[0..(pos.1 as usize) + 1].iter().rev().enumerate() {
                if let Some(m) = re.find(&line) {
                    return Some((m.start() as u16, pos.1 - i as u16));
                }
            }
        }
        None
    }
}
