use std::io::{Read, BufRead, Write};
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

    fn handle(&self, keyop: &KeyOp, scr: &mut Screen) {
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
                // TODO: implementation
            }
            KeyOp::SearchPrev => {
                // TODO: implementation
            }
            KeyOp::SearchIncremental(s) => {
                // TODO: implementation
                scr.call(ScreenCall::Message(Some(&s)));
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

    pub fn run(&mut self, keystream: &mut Read, bufinstream: &mut BufRead, outstream: &mut Write) {
        // read buffer from buffer-stream
        let mut buffer: Vec<String> = vec![];
        for v in bufinstream.lines().map(|v| v.unwrap()) {
            buffer.push(v);
        }
        let mut scr = Screen::new(&buffer, outstream, self.flags.nlines);
        scr.call(ScreenCall::ShowLineNumber(self.flags.show_line_number));
        scr.call(ScreenCall::ShowNonPrinting(self.flags.show_nonprinting));
        scr.call(ScreenCall::Refresh);

        let mut kb = keybind::default::KeyBind::new();
        let mut keh = KeyEventHandler::new(keystream, &mut kb);

        loop {
            match keh.read() {
                Some(keyop) => {
                    self.handle(&keyop, &mut scr);
                    if keyop == KeyOp::Quit {
                        break;
                    }
                }
                None => {}
            }
        }
    }
}

