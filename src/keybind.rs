/// Key Bind Parser
use crate::event::PeepEvent;
use termion::event::Key;

pub trait KeyParser {
    fn parse(&mut self, k: termion::event::Key) -> Option<PeepEvent>;
}

/// Default key map
pub mod default {
    use super::*;
    use std::collections::HashMap;

    // Key input state transition
    //
    // | State        | '/'          | 0-9       | Any Commands | Enter | Esc   | (Complete Command) |
    // | ------------ | ------------ | --------- | ------------ | ----- | ----- | ------------------ |
    // | Ready        | IncSearching | Numbering | Commanding   | -     | -     | -                  |
    // | IncSearching | -            | -         | -            | Ready | Ready | -                  |
    // | Numbering    | -            | -         | Commanding   | Ready | Ready | -                  |
    // | Commanding   | -            | -         | -            | -     | -     | Ready              |
    enum State {
        Ready,
        IncSearching,
        Numbering,
        Commanding,
    }

    pub struct KeyBind {
        state: State,
        number: u16,
        searching_keys: String,
        cmap: HashMap<Key, PeepEvent>,
    }

    impl Default for KeyBind {
        fn default() -> Self {
            Self::new()
        }
    }

    impl KeyBind {
        pub fn new() -> Self {
            let mut kb = KeyBind {
                state: State::Ready,
                number: 0,
                searching_keys: String::with_capacity(64),
                cmap: HashMap::new(),
            };
            kb.cmap = KeyBind::default_command_table();
            kb
        }

        fn default_command_table() -> HashMap<Key, PeepEvent> {
            [
                (Key::Char('j'), PeepEvent::MoveDown(1)),
                (Key::Ctrl('j'), PeepEvent::MoveDown(1)),
                (Key::Ctrl('n'), PeepEvent::MoveDown(1)),
                (Key::Char('k'), PeepEvent::MoveUp(1)),
                (Key::Ctrl('k'), PeepEvent::MoveUp(1)),
                (Key::Ctrl('p'), PeepEvent::MoveUp(1)),
                (Key::Char('h'), PeepEvent::MoveLeft(1)),
                (Key::Char('l'), PeepEvent::MoveRight(1)),
                (Key::Char('d'), PeepEvent::MoveDownHalfPages(1)),
                (Key::Ctrl('d'), PeepEvent::MoveDownHalfPages(1)),
                (Key::Char('u'), PeepEvent::MoveUpHalfPages(1)),
                (Key::Ctrl('u'), PeepEvent::MoveUpHalfPages(1)),
                (Key::Char('H'), PeepEvent::MoveLeftHalfPages(1)),
                (Key::Char('L'), PeepEvent::MoveRightHalfPages(1)),
                (Key::Char('f'), PeepEvent::MoveDownPages(1)),
                (Key::Ctrl('f'), PeepEvent::MoveDownPages(1)),
                (Key::Char(' '), PeepEvent::MoveDownPages(1)),
                (Key::Char('b'), PeepEvent::MoveUpPages(1)),
                (Key::Ctrl('b'), PeepEvent::MoveUpPages(1)),
                (Key::Char('0'), PeepEvent::MoveToHeadOfLine),
                (Key::Ctrl('a'), PeepEvent::MoveToHeadOfLine),
                (Key::Char('$'), PeepEvent::MoveToEndOfLine),
                (Key::Ctrl('e'), PeepEvent::MoveToEndOfLine),
                (Key::Char('g'), PeepEvent::MoveToTopOfLines),
                (Key::Char('G'), PeepEvent::MoveToBottomOfLines),
                (Key::Char('#'), PeepEvent::ToggleLineNumberPrinting),
                (Key::Char('!'), PeepEvent::ToggleLineWraps),
                (Key::Char('-'), PeepEvent::DecrementLines(1)),
                (Key::Char('+'), PeepEvent::IncrementLines(1)),
                (Key::Char('='), PeepEvent::SetNumOfLines(0)),
                (Key::Char('n'), PeepEvent::SearchNext),
                (Key::Char('N'), PeepEvent::SearchPrev),
                (Key::Char('q'), PeepEvent::Quit),
                (Key::Char('Q'), PeepEvent::QuitWithClear),
                (Key::Char('F'), PeepEvent::FollowMode),
            ]
            .iter()
            .cloned()
            .collect()
        }

        fn trans_to_ready(&mut self) {
            self.state = State::Ready;
            self.number = 0;
        }
        fn trans_to_incsearching(&mut self) {
            self.state = State::IncSearching;
            self.number = 0;
        }
        fn trans_to_numbering(&mut self, c: char) {
            self.state = State::Numbering;
            self.number = c.to_digit(10).unwrap() as u16;
            self.searching_keys.push(c);
        }
        fn trans_to_commanding(&mut self) {
            self.state = State::Commanding;
        }

        fn action_ready(&mut self, k: Key) -> Option<PeepEvent> {
            match k {
                Key::Char('/') => {
                    self.trans_to_incsearching();
                    Some(PeepEvent::SearchIncremental("".to_owned()))
                }
                Key::Char(c @ '1'..='9') => {
                    self.trans_to_numbering(c);
                    // Some(PeepEvent::Message(Some(self.number.to_string())))
                    None
                }
                Key::Char(_) | Key::Ctrl(_) => {
                    self.trans_to_commanding();
                    self.action_commanding(k)
                }
                Key::Esc => Some(PeepEvent::Cancel),
                _ => None,
            }
        }

        fn action_incsearching(&mut self, k: Key) -> Option<PeepEvent> {
            match k {
                Key::Char('\n') => {
                    self.searching_keys.clear();
                    self.trans_to_ready();
                    Some(PeepEvent::SearchTrigger)
                }
                Key::Char(c) => {
                    self.searching_keys.push(c);
                    Some(PeepEvent::SearchIncremental(self.searching_keys.to_owned()))
                }
                Key::Backspace | Key::Delete => {
                    if self.searching_keys.pop().is_none() {
                        self.searching_keys.clear();
                        self.trans_to_ready();
                        Some(PeepEvent::Cancel)
                    } else {
                        Some(PeepEvent::SearchIncremental(self.searching_keys.to_owned()))
                    }
                }
                Key::Esc => {
                    // ESC -> Cancel
                    self.searching_keys.clear();
                    self.trans_to_ready();
                    Some(PeepEvent::Cancel)
                }
                _ => None,
            }
        }

        fn action_numbering(&mut self, k: Key) -> Option<PeepEvent> {
            match k {
                Key::Char(c @ '0'..='9') => {
                    self.number = self.number * 10 + c.to_digit(10).unwrap() as u16;
                    // Some(PeepEvent::Message(Some(self.number.to_string())))
                    None
                }
                Key::Esc | Key::Char('\n') => {
                    // ESC and LF -> Cancel
                    self.trans_to_ready();
                    None
                    // Some(PeepEvent::Message(None))
                }
                Key::Char(_) | Key::Ctrl(_) => {
                    self.trans_to_commanding();
                    self.action_commanding(k)
                }
                _ => None,
            }
        }

        fn action_commanding(&mut self, k: Key) -> Option<PeepEvent> {
            let op = match k {
                Key::Char(_) | Key::Ctrl(_) => {
                    match self.cmap.get::<Key>(&k) {
                        Some(v) => {
                            // hit command
                            self.combine_command(v.to_owned())
                        }
                        None => {
                            // not exist => cancel
                            Some(PeepEvent::Message(None))
                        }
                    }
                }
                _ => Some(PeepEvent::Message(None)),
            };
            self.trans_to_ready();
            op
        }

        fn combine_command(&self, op: PeepEvent) -> Option<PeepEvent> {
            let valid_num = |n| if n == 0 { 1 } else { n };
            match op {
                PeepEvent::MoveDown(_) => Some(PeepEvent::MoveDown(valid_num(self.number))),
                PeepEvent::MoveUp(_) => Some(PeepEvent::MoveUp(valid_num(self.number))),
                PeepEvent::MoveLeft(_) => Some(PeepEvent::MoveLeft(valid_num(self.number))),
                PeepEvent::MoveRight(_) => Some(PeepEvent::MoveRight(valid_num(self.number))),
                PeepEvent::MoveDownHalfPages(_) => {
                    Some(PeepEvent::MoveDownHalfPages(valid_num(self.number)))
                }
                PeepEvent::MoveUpHalfPages(_) => {
                    Some(PeepEvent::MoveUpHalfPages(valid_num(self.number)))
                }
                PeepEvent::MoveLeftHalfPages(_) => {
                    Some(PeepEvent::MoveLeftHalfPages(valid_num(self.number)))
                }
                PeepEvent::MoveRightHalfPages(_) => {
                    Some(PeepEvent::MoveRightHalfPages(valid_num(self.number)))
                }
                PeepEvent::MoveDownPages(_) => {
                    Some(PeepEvent::MoveDownPages(valid_num(self.number)))
                }
                PeepEvent::MoveUpPages(_) => Some(PeepEvent::MoveUpPages(valid_num(self.number))),
                PeepEvent::MoveToTopOfLines | PeepEvent::MoveToBottomOfLines => {
                    if self.number == 0 {
                        Some(op)
                    } else {
                        Some(PeepEvent::MoveToLineNumber(self.number - 1))
                    }
                }
                PeepEvent::MoveToLineNumber(_) => {
                    Some(PeepEvent::MoveToLineNumber(valid_num(self.number)))
                }
                PeepEvent::IncrementLines(_) => {
                    Some(PeepEvent::IncrementLines(valid_num(self.number)))
                }
                PeepEvent::DecrementLines(_) => {
                    Some(PeepEvent::DecrementLines(valid_num(self.number)))
                }
                PeepEvent::SetNumOfLines(_) => {
                    if self.number == 0 {
                        None
                    } else {
                        Some(PeepEvent::SetNumOfLines(self.number))
                    }
                }
                _ => Some(op),
            }
        }

        fn trans(&mut self, k: Key) -> Option<PeepEvent> {
            match self.state {
                State::Ready => self.action_ready(k),
                State::IncSearching => self.action_incsearching(k),
                State::Numbering => self.action_numbering(k),
                State::Commanding => self.action_commanding(k),
            }
        }
    }

    impl KeyParser for KeyBind {
        fn parse(&mut self, k: Key) -> Option<PeepEvent> {
            self.trans(k)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::PeepEvent;

    #[rustfmt::skip]
    #[test]
    fn test_keybind_command() {
        let mut kb = default::KeyBind::new();

        // normal commands
        assert_eq!(kb.parse(Key::Char('j')), Some(PeepEvent::MoveDown(1)));
        assert_eq!(kb.parse(Key::Ctrl('j')), Some(PeepEvent::MoveDown(1)));
        assert_eq!(kb.parse(Key::Ctrl('n')), Some(PeepEvent::MoveDown(1)));
        assert_eq!(kb.parse(Key::Char('k')), Some(PeepEvent::MoveUp(1)));
        assert_eq!(kb.parse(Key::Ctrl('k')), Some(PeepEvent::MoveUp(1)));
        assert_eq!(kb.parse(Key::Ctrl('p')), Some(PeepEvent::MoveUp(1)));
        assert_eq!(kb.parse(Key::Char('h')), Some(PeepEvent::MoveLeft(1)));
        assert_eq!(kb.parse(Key::Char('l')), Some(PeepEvent::MoveRight(1)));
        assert_eq!(kb.parse(Key::Char('d')), Some(PeepEvent::MoveDownHalfPages(1)));
        assert_eq!(kb.parse(Key::Ctrl('d')), Some(PeepEvent::MoveDownHalfPages(1)));
        assert_eq!(kb.parse(Key::Char('u')), Some(PeepEvent::MoveUpHalfPages(1)));
        assert_eq!(kb.parse(Key::Ctrl('u')), Some(PeepEvent::MoveUpHalfPages(1)));
        assert_eq!(kb.parse(Key::Char('f')), Some(PeepEvent::MoveDownPages(1)));
        assert_eq!(kb.parse(Key::Ctrl('f')), Some(PeepEvent::MoveDownPages(1)));
        assert_eq!(kb.parse(Key::Char(' ')), Some(PeepEvent::MoveDownPages(1)));
        assert_eq!(kb.parse(Key::Char('b')), Some(PeepEvent::MoveUpPages(1)));
        assert_eq!(kb.parse(Key::Ctrl('b')), Some(PeepEvent::MoveUpPages(1)));
        assert_eq!(kb.parse(Key::Char('0')), Some(PeepEvent::MoveToHeadOfLine));
        assert_eq!(kb.parse(Key::Ctrl('a')), Some(PeepEvent::MoveToHeadOfLine));
        assert_eq!(kb.parse(Key::Char('$')), Some(PeepEvent::MoveToEndOfLine));
        assert_eq!(kb.parse(Key::Ctrl('e')), Some(PeepEvent::MoveToEndOfLine));
        assert_eq!(kb.parse(Key::Char('g')), Some(PeepEvent::MoveToTopOfLines));
        assert_eq!(kb.parse(Key::Char('G')), Some(PeepEvent::MoveToBottomOfLines));
        assert_eq!(kb.parse(Key::Char('-')), Some(PeepEvent::DecrementLines(1)));
        assert_eq!(kb.parse(Key::Char('+')), Some(PeepEvent::IncrementLines(1)));
        assert_eq!(kb.parse(Key::Char('=')), None);
        assert_eq!(kb.parse(Key::Char('n')), Some(PeepEvent::SearchNext));
        assert_eq!(kb.parse(Key::Char('N')), Some(PeepEvent::SearchPrev));
        assert_eq!(kb.parse(Key::Char('q')), Some(PeepEvent::Quit));
        assert_eq!(kb.parse(Key::Char('Q')), Some(PeepEvent::QuitWithClear));
        assert_eq!(kb.parse(Key::Char('#')), Some(PeepEvent::ToggleLineNumberPrinting));
        assert_eq!(kb.parse(Key::Char('!')), Some(PeepEvent::ToggleLineWraps));
        assert_eq!(kb.parse(Key::Char('F')), Some(PeepEvent::FollowMode));
        assert_eq!(kb.parse(Key::Esc), Some(PeepEvent::Cancel));
    }

    #[rustfmt::skip]
    #[test]
    fn test_keybind_number() {
        let mut kb = default::KeyBind::new();

        // normal commands
        assert_eq!(kb.parse(Key::Char('1')), None);
        assert_eq!(kb.parse(Key::Char('2')), None);
        assert_eq!(kb.parse(Key::Char('\n')), None);

        assert_eq!(kb.parse(Key::Char('1')), None);
        assert_eq!(kb.parse(Key::Char('2')), None);
        assert_eq!(kb.parse(Key::Esc), None);

        assert_eq!(kb.parse(Key::Char('2')), None);
        assert_eq!(kb.parse(Key::Char('j')), Some(PeepEvent::MoveDown(2)));

        assert_eq!(kb.parse(Key::Char('1')), None);
        assert_eq!(kb.parse(Key::Char('0')), None);
        assert_eq!(kb.parse(Key::Char('h')), Some(PeepEvent::MoveLeft(10)));

        assert_eq!(kb.parse(Key::Char('1')), None);
        assert_eq!(kb.parse(Key::Char('0')), None);
        assert_eq!(kb.parse(Key::Char('=')), Some(PeepEvent::SetNumOfLines(10)));
    }

    #[rustfmt::skip]
    #[test]
    fn test_keybind_search() {
        let mut kb = default::KeyBind::new();

        // search commands
        assert_eq!(kb.parse(Key::Char('/')), Some(PeepEvent::SearchIncremental("".to_owned())));
        assert_eq!(kb.parse(Key::Char('w')), Some(PeepEvent::SearchIncremental("w".to_owned())));
        assert_eq!(kb.parse(Key::Char('o')), Some(PeepEvent::SearchIncremental("wo".to_owned())));
        assert_eq!(kb.parse(Key::Char('r')), Some(PeepEvent::SearchIncremental("wor".to_owned())));
        assert_eq!(kb.parse(Key::Char('d')), Some(PeepEvent::SearchIncremental("word".to_owned())));
        assert_eq!(kb.parse(Key::Char('\n')), Some(PeepEvent::SearchTrigger));

        assert_eq!(kb.parse(Key::Char('/')), Some(PeepEvent::SearchIncremental("".to_owned())));
        assert_eq!(kb.parse(Key::Char('a')), Some(PeepEvent::SearchIncremental("a".to_owned())));
        assert_eq!(kb.parse(Key::Char('b')), Some(PeepEvent::SearchIncremental("ab".to_owned())));
        assert_eq!(kb.parse(Key::Backspace), Some(PeepEvent::SearchIncremental("a".to_owned())));
        assert_eq!(kb.parse(Key::Backspace), Some(PeepEvent::SearchIncremental("".to_owned())));
        assert_eq!(kb.parse(Key::Backspace), Some(PeepEvent::Cancel));

        assert_eq!(kb.parse(Key::Char('/')), Some(PeepEvent::SearchIncremental("".to_owned())));
        assert_eq!(kb.parse(Key::Char('w')), Some(PeepEvent::SearchIncremental("w".to_owned())));
        assert_eq!(kb.parse(Key::Char('o')), Some(PeepEvent::SearchIncremental("wo".to_owned())));
        assert_eq!(kb.parse(Key::Char('\n')), Some(PeepEvent::SearchTrigger));
        assert_eq!(kb.parse(Key::Char('n')), Some(PeepEvent::SearchNext));
        assert_eq!(kb.parse(Key::Char('N')), Some(PeepEvent::SearchPrev));
    }
}
