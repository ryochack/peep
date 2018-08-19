use std::io::{Read, Write};
use screen::{Screen, ScreenCall};
use keyevt::{KeyEventHandler, KeyOp};
use keybind;

struct Flags {
    nlines: u32,
    show_nonprinting: bool,
    show_line_number: bool,
}

pub struct App {
    flags: Flags,
}

impl App {
    pub fn new() -> Self {
        App {
            flags: Flags {
                nlines: 5,
                show_nonprinting: false,
                show_line_number: false,
            },
        }
    }

    pub fn set_numof_lines(&mut self, nlines: u32) -> &mut Self {
        self.flags.nlines = nlines;
        self
    }

    pub fn set_show_nonprinting(&mut self, show_nonprinting: bool) -> &mut Self {
        self.flags.show_nonprinting = show_nonprinting;
        self
    }

    pub fn set_show_line_number(&mut self, show_line_number: bool) -> &mut Self {
        self.flags.show_line_number = show_line_number;
        self
    }

    fn search(&self, buffer: &[String], pos: (u32, u32), pat: &str, reverse: bool) -> Option<(u32, u32)> {
        if pat.is_empty() {
            return None;
        }
        // TODO: BAD...
        if !reverse {
            for (i, line) in buffer[(pos.1 as usize)..].iter().enumerate() {
                match line.find(pat) {
                    Some(c) => {
                        // hit!
                        return Some((c as u32, pos.1 + i as u32))
                    }
                    None => {}
                }
            }
        } else {
            for (i, line) in buffer[0..(pos.1 as usize) + 1].iter().rev().enumerate() {
                match line.find(pat) {
                    Some(c) => {
                        // hit!
                        return Some((c as u32, pos.1 - i as u32))
                    }
                    None => {}
                }
            }
        }
        None
    }

    fn handle(&self, keyop: &KeyOp, scr: &mut Screen, buffer: &[String]) {
        match keyop {
            &KeyOp::MoveDown(n) => {
                scr.call(ScreenCall::MoveDown(n));
            }
            &KeyOp::MoveUp(n) => {
                scr.call(ScreenCall::MoveUp(n));
            }
            &KeyOp::MoveLeft(n) => {
                scr.call(ScreenCall::MoveLeft(n));
            }
            &KeyOp::MoveRight(n) => {
                scr.call(ScreenCall::MoveRight(n));
            }
            &KeyOp::MoveDownHalfPages(n) => {
                scr.call(ScreenCall::MoveDownHalfPages(n));
            }
            &KeyOp::MoveUpHalfPages(n) => {
                scr.call(ScreenCall::MoveUpHalfPages(n));
            }
            &KeyOp::MoveDownPages(n) => {
                scr.call(ScreenCall::MoveDownPages(n));
            }
            &KeyOp::MoveUpPages(n) => {
                scr.call(ScreenCall::MoveUpPages(n));
            }
            KeyOp::MoveToHeadOfLine => {
                scr.call(ScreenCall::MoveToHeadOfLine);
            }
            KeyOp::MoveToEndOfLine => {
                scr.call(ScreenCall::MoveToEndOfLine);
            }
            KeyOp::MoveToTopOfLines => {
                scr.call(ScreenCall::MoveToTopOfLines);
            }
            KeyOp::MoveToBottomOfLines => {
                scr.call(ScreenCall::MoveToBottomOfLines);
            }
            &KeyOp::MoveToLineNumber(n) => {
                scr.call(ScreenCall::MoveToLineNumber(n));
            }
            &KeyOp::ShowLineNumber(b) => {
                scr.call(ScreenCall::ShowLineNumber(b));
            }
            &KeyOp::ShowNonPrinting(b) => {
                scr.call(ScreenCall::ShowNonPrinting(b));
            }
            &KeyOp::IncrementLines(n) => {
                scr.call(ScreenCall::IncrementLines(n));
            }
            &KeyOp::DecrementLines(n) => {
                scr.call(ScreenCall::DecrementLines(n));
            }
            &KeyOp::SetNumOfLines(n) => {
                scr.call(ScreenCall::SetNumOfLines(n));
            }
            KeyOp::SearchNext => {
                let cur_pos = scr.position();
                let next_pos = (cur_pos.0,
                                if cur_pos.1 == buffer.len() as u32 - 1 {
                                    buffer.len() as u32 - 1
                                } else {
                                    cur_pos.1 + 1
                                });
                match self.search(buffer, next_pos, scr.hlword(), false) {
                    Some(pos) => {
                        scr.call(ScreenCall::MoveToLineNumber(pos.1));
                    }
                    None => {}
                }
                scr.call(ScreenCall::Message(None));
                scr.call(ScreenCall::Refresh);
            }
            KeyOp::SearchPrev => {
                let cur_pos = scr.position();
                let next_pos = (cur_pos.0,
                                if cur_pos.1 == 0 {
                                    0
                                } else {
                                    cur_pos.1 - 1
                                });
                match self.search(buffer, next_pos, scr.hlword(), true) {
                    Some(pos) => {
                        scr.call(ScreenCall::MoveToLineNumber(pos.1));
                    }
                    None => {}
                }
                scr.call(ScreenCall::Message(None));
                scr.call(ScreenCall::Refresh);
            }
            KeyOp::SearchIncremental(s) => {
                match self.search(buffer, scr.position(), s.as_str(), false) {
                    Some(pos) => {
                        scr.call(ScreenCall::MoveToLineNumber(pos.1));
                    }
                    None => {}
                }
                scr.call(ScreenCall::Message(Some(&format!("/{}", s))));
                scr.call(ScreenCall::HighLightWord(Some(&s)));
            }
            KeyOp::Message(s) => {
                scr.call(ScreenCall::Message(Some(&s)));
            }
            KeyOp::Cancel => {
                scr.call(ScreenCall::Message(None));
                scr.call(ScreenCall::HighLightWord(None));
                scr.call(ScreenCall::Refresh);
            }
            KeyOp::Quit => {
                scr.call(ScreenCall::Quit);
            }
        }
    }

    pub fn run(&mut self, instream: &mut Read, outstream: &mut Write, buffer: &Vec<String>) {
        let mut scr = Screen::new(buffer, outstream, self.flags.nlines);
        scr.call(ScreenCall::ShowLineNumber(self.flags.show_line_number));
        scr.call(ScreenCall::ShowNonPrinting(self.flags.show_nonprinting));
        scr.call(ScreenCall::Refresh);

        let mut kb = keybind::default::KeyBind::new();
        let mut keh = KeyEventHandler::new(instream, &mut kb);

        loop {
            match keh.read() {
                Some(keyop) => {
                    self.handle(&keyop, &mut scr, buffer);
                    if keyop == KeyOp::Quit {
                        break;
                    }
                }
                None => {}
            }
        }
    }
}

