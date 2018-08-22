use std::io::{self, Read, Write};
use keyevt::{KeyEventHandler, KeyOp};
use keybind;
use pane::{self, Pane, ScrollStep};

struct Flags {
    nlines: u32,
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
                show_line_number: false,
            },
        }
    }

    pub fn set_numof_lines(&mut self, nlines: u32) -> &mut Self {
        self.flags.nlines = nlines;
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

    #[allow(unused_must_use)]
    fn handle(&self, keyop: &KeyOp, pane: &mut Pane, buffer: &[String]) {
        match keyop {
            &KeyOp::MoveDown(n) => {
                pane.scroll_down(ScrollStep::Char(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveDown(n));
            }
            &KeyOp::MoveUp(n) => {
                pane.scroll_up(ScrollStep::Char(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveUp(n));
            }
            &KeyOp::MoveLeft(n) => {
                pane.scroll_left(ScrollStep::Char(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveLeft(n));
            }
            &KeyOp::MoveRight(n) => {
                pane.scroll_right(ScrollStep::Char(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveRight(n));
            }
            &KeyOp::MoveDownHalfPages(n) => {
                pane.scroll_down(ScrollStep::Halfpage(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveDownHalfPages(n));
            }
            &KeyOp::MoveUpHalfPages(n) => {
                pane.scroll_up(ScrollStep::Halfpage(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveUpHalfPages(n));
            }
            &KeyOp::MoveDownPages(n) => {
                pane.scroll_down(ScrollStep::Page(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveDownPages(n));
            }
            &KeyOp::MoveUpPages(n) => {
                pane.scroll_up(ScrollStep::Page(n));
                pane.refresh();
                // scr.call(ScreenCall::MoveUpPages(n));
            }
            KeyOp::MoveToHeadOfLine => {
                pane.goto_head_of_line();
                pane.refresh();
                // scr.call(ScreenCall::MoveToHeadOfLine);
            }
            KeyOp::MoveToEndOfLine => {
                pane.goto_tail_of_line();
                pane.refresh();
                // scr.call(ScreenCall::MoveToEndOfLine);
            }
            KeyOp::MoveToTopOfLines => {
                pane.goto_top_of_lines();
                pane.refresh();
                // scr.call(ScreenCall::MoveToTopOfLines);
            }
            KeyOp::MoveToBottomOfLines => {
                pane.goto_bottom_of_lines();
                pane.refresh();
                // scr.call(ScreenCall::MoveToBottomOfLines);
            }
            &KeyOp::MoveToLineNumber(n) => {
                pane.goto_absolute_line(n);
                pane.refresh();
                // scr.call(ScreenCall::MoveToLineNumber(n));
            }
            &KeyOp::ShowLineNumber(b) => {
                pane.show_line_number(b);
                pane.refresh();
                // scr.call(ScreenCall::ShowLineNumber(b));
            }
            &KeyOp::IncrementLines(n) => {
                pane.increment_height(n);
                pane.refresh();
                // scr.call(ScreenCall::IncrementLines(n));
            }
            &KeyOp::DecrementLines(n) => {
                pane.decrement_height(n);
                pane.refresh();
                // scr.call(ScreenCall::DecrementLines(n));
            }
            &KeyOp::SetNumOfLines(n) => {
                pane.set_height(n);
                pane.refresh();
                // scr.call(ScreenCall::SetNumOfLines(n));
            }

            KeyOp::SearchNext => {
                // let cur_pos = scr.position();
                // let next_pos = (cur_pos.0,
                //                 if cur_pos.1 == buffer.len() as u32 - 1 {
                //                     buffer.len() as u32 - 1
                //                 } else {
                //                     cur_pos.1 + 1
                //                 });
                // match self.search(buffer, next_pos, scr.hlword(), false) {
                //     Some(pos) => {
                //         scr.call(ScreenCall::MoveToLineNumber(pos.1));
                //     }
                //     None => {}
                // }
                // scr.call(ScreenCall::Message(None));
                // scr.call(ScreenCall::Refresh);
            }
            KeyOp::SearchPrev => {
                // let cur_pos = scr.position();
                // let next_pos = (cur_pos.0,
                //                 if cur_pos.1 == 0 {
                //                     0
                //                 } else {
                //                     cur_pos.1 - 1
                //                 });
                // match self.search(buffer, next_pos, scr.hlword(), true) {
                //     Some(pos) => {
                //         scr.call(ScreenCall::MoveToLineNumber(pos.1));
                //     }
                //     None => {}
                // }
                // scr.call(ScreenCall::Message(None));
                // scr.call(ScreenCall::Refresh);
            }
            KeyOp::SearchIncremental(s) => {
                pane.set_highlight_word(Some(&s));
                pane.refresh();
                // match self.search(buffer, scr.position(), s.as_str(), false) {
                //     Some(pos) => {
                //         scr.call(ScreenCall::MoveToLineNumber(pos.1));
                //     }
                //     None => {}
                // }
                // scr.call(ScreenCall::Message(Some(&format!("/{}", s))));
                // scr.call(ScreenCall::HighLightWord(Some(&s)));
            }

            KeyOp::Message(s) => {
                // pane.set_message(Some(&s));
                pane.refresh();
                // scr.call(ScreenCall::Message(Some(&s)));
            }
            KeyOp::Cancel => {
                pane.set_message(None);
                pane.set_highlight_word(None);
                pane.refresh();
                // scr.call(ScreenCall::Message(None));
                // scr.call(ScreenCall::HighLightWord(None));
                // scr.call(ScreenCall::Refresh);
            }
            KeyOp::Quit => {
                pane.quit();
                // scr.call(ScreenCall::Quit);
            }
        }
    }

    pub fn run(&mut self, r: &mut Read, w: &mut Write, buffer: &Vec<String>) {
        let mut pane = Pane::new(w);
        pane.load(buffer);
        pane.show_line_number(self.flags.show_line_number);
        pane.refresh();

        let mut kb = keybind::default::KeyBind::new();
        let mut keh = KeyEventHandler::new(r, &mut kb);

        loop {
            match keh.read() {
                Some(keyop) => {
                    self.handle(&keyop, &mut pane, buffer);
                    if keyop == KeyOp::Quit {
                        break;
                    }
                }
                None => {}
            }
        }
    }
}

