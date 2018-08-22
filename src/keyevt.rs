extern crate termios;

/// Key Event Handler
use keybind;
use tty;
use std::io::Read;

#[derive(Clone, Debug, PartialEq)]
pub enum KeyOp {
    MoveDown(u16),
    MoveUp(u16),
    MoveLeft(u16),
    MoveRight(u16),
    MoveDownHalfPages(u16),
    MoveUpHalfPages(u16),
    MoveLeftHalfPages(u16),
    MoveRightHalfPages(u16),
    MoveDownPages(u16),
    MoveUpPages(u16),
    MoveToHeadOfLine,
    MoveToEndOfLine,
    MoveToTopOfLines,
    MoveToBottomOfLines,
    MoveToLineNumber(u16),

    ToggleLineNumberPrinting,
    IncrementLines(u16),
    DecrementLines(u16),
    SetNumOfLines(u16),

    SearchIncremental(String),
    SearchTrigger,
    SearchNext,
    SearchPrev,

    Message(Option<String>),

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

