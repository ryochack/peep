extern crate termios;

/// Key Event Handler
use keybind;
use ttyecho;
use std::io::Read;

#[derive(Clone, Debug, PartialEq)]
pub enum KeyOp {
    MoveDown(u32),
    MoveUp(u32),
    MoveLeft(u32),
    MoveRight(u32),
    MoveDownHalfPages(u32),
    MoveUpHalfPages(u32),
    MoveDownPages(u32),
    MoveUpPages(u32),
    MoveToHeadOfLine,
    MoveToEndOfLine,
    MoveToTopOfLines,
    MoveToBottomOfLines,
    MoveToLineNumber(u32),

    ShowLineNumber(bool),
    ShowNonPrinting(bool),
    IncrementLines(u32),
    DecrementLines(u32),
    SetNumOfLines(u32),

    SearchNext,
    SearchPrev,
    SearchIncremental(String),

    Message(String),

    Cancel,
    Quit,
}

pub struct KeyEventHandler<'a> {
    istream: &'a mut Read,
    parser: &'a mut keybind::KeyParser,
    oldstat: Box<termios::Termios>,
}

impl<'a> Drop for KeyEventHandler<'a> {
    fn drop(&mut self) {
        ttyecho::on(&*self.oldstat);
    }
}

impl<'a> KeyEventHandler<'a> {
    pub fn new(istream: &'a mut Read, parser: &'a mut keybind::KeyParser) -> Self {
        KeyEventHandler {
            istream: istream,
            parser: parser,
            oldstat: Box::new(ttyecho::off()),
        }
    }

    pub fn read(&mut self) -> Option<KeyOp> {
        for b in self.istream.bytes().filter_map(|v| v.ok()) {
            let v = self.parser.parse(b as char);
            if v.is_some() {
                return v;
            }
        }
        None
    }
}

