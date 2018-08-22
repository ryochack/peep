/// Key Bind Parser
use keyevt::KeyOp;

pub trait KeyParser {
    fn parse(&mut self, c: char) -> Option<KeyOp>;
}

pub mod default {
    use super::*;
    use std::collections::HashMap;

    // Ready -> IncSearching
    // IncSearching -> Ready
    //
    // Ready -> Numbering
    // Numbering -> Ready : cancel
    // Numbering -> Commanding
    //
    // Ready -> Commanding
    // Commanding -> Ready
    enum State {
        Ready,
        IncSearching,
        Numbering,
        Commanding,
    }

    pub struct KeyBind<'a> {
        state: State,
        number: u16,
        wip_keys: String,
        cmap: HashMap<&'a str, KeyOp>,
    }

    impl<'a> KeyBind<'a> {
        pub fn new() -> Self {
            let mut kb = KeyBind {
                state: State::Ready,
                number: 0,
                wip_keys: String::with_capacity(64),
                cmap: HashMap::new(),
            };
            kb.cmap = KeyBind::default_command_table();
            kb
        }

        fn default_command_table() -> HashMap<&'a str, KeyOp> {
            // let mut default: HashMap<&str, KeyOp> = [
            [
                ("j", KeyOp::MoveDown(1)),
                ("k", KeyOp::MoveUp(1)),
                ("h", KeyOp::MoveLeft(1)),
                ("l", KeyOp::MoveRight(1)),
                ("d", KeyOp::MoveDownHalfPages(1)),
                ("u", KeyOp::MoveUpHalfPages(1)),
                ("f", KeyOp::MoveDownPages(1)),
                ("b", KeyOp::MoveUpPages(1)),
                ("0", KeyOp::MoveToHeadOfLine),
                ("$", KeyOp::MoveToEndOfLine),
                ("gg", KeyOp::MoveToTopOfLines),
                ("G", KeyOp::MoveToBottomOfLines),
                ("#", KeyOp::ToggleLineNumberPrinting),
                ("-", KeyOp::DecrementLines(1)),
                ("+", KeyOp::IncrementLines(1)),
                ("n", KeyOp::SearchNext),
                ("N", KeyOp::SearchPrev),
                ("q", KeyOp::Quit),
            ]
                .iter()
                .cloned()
                .collect()
        }

        fn trans_to_ready(&mut self) {
            self.state = State::Ready;
            self.number = 0;
            self.wip_keys.clear();
        }
        fn trans_to_incsearching(&mut self) {
            self.state = State::IncSearching;
            self.number = 0;
            self.wip_keys.clear();
        }
        fn trans_to_numbering(&mut self, c: char) {
            self.state = State::Numbering;
            self.number = c.to_digit(10).unwrap() as u16;
            self.wip_keys.push(c);
        }
        fn trans_to_commanding(&mut self) {
            self.state = State::Commanding;
            self.wip_keys.clear();
        }

        fn action_ready(&mut self, c: char) -> Option<KeyOp> {
            match c {
                '/' => {
                    self.trans_to_incsearching();
                    Some(KeyOp::SearchIncremental(format!("{}", self.wip_keys)))
                }
                '1'...'9' => {
                    self.trans_to_numbering(c);
                    // Some(KeyOp::Message(Some(self.number.to_string())))
                    None
                }
                c if !c.is_control() => {
                    self.trans_to_commanding();
                    self.action_commanding(c)
                }
                // ESC
                '\x1b' => Some(KeyOp::Cancel),
                _ => None,
            }
        }
        fn action_incsearching(&mut self, c: char) -> Option<KeyOp> {
            match c {
                c if !c.is_control() => {
                    self.wip_keys.push(c);
                    Some(KeyOp::SearchIncremental(format!("{}", self.wip_keys)))
                }
                '\x08' | '\x7f' => {
                    // BackSpace, Delete
                    if self.wip_keys.pop().is_none() {
                        self.trans_to_ready();
                        Some(KeyOp::Cancel)
                    } else {
                        Some(KeyOp::SearchIncremental(format!("{}", self.wip_keys)))
                    }
                }
                '\n' => {
                    // LF
                    self.trans_to_ready();
                    Some(KeyOp::SearchTrigger)
                }
                '\x1b' => {
                    // ESC -> Cancel
                    self.trans_to_ready();
                    Some(KeyOp::Cancel)
                }
                _ => None,
            }
        }
        fn action_numbering(&mut self, c: char) -> Option<KeyOp> {
            match c {
                '0'...'9' => {
                    self.number = self.number * 10 + c.to_digit(10).unwrap() as u16;
                    // Some(KeyOp::Message(Some(self.number.to_string())))
                    None
                }
                c if !c.is_control() => {
                    self.trans_to_commanding();
                    self.action_commanding(c)
                }
                '\x1b' | '\n' => {
                    // ESC and LF -> Cancel
                    self.trans_to_ready();
                    Some(KeyOp::Message(None))
                }
                _ => None,
            }
        }
        fn action_commanding(&mut self, c: char) -> Option<KeyOp> {
            let mut needs_trans = false;
            let op = match c {
                c if !c.is_control() => {
                    self.wip_keys.push(c);
                    match self.cmap.get::<str>(&self.wip_keys) {
                        Some(v) => {
                            needs_trans = true;
                            Some(self.combine_command(v.to_owned()))
                        }
                        None => {
                            if self.cmap.keys().any(|&k| k.starts_with(&self.wip_keys)) {
                                // has candidates
                                None
                            } else {
                                // not exist => cancel
                                needs_trans = true;
                                Some(KeyOp::Message(None))
                            }
                        }
                    }
                }
                _ => {
                    needs_trans = true;
                    Some(KeyOp::Message(None))
                }
            };
            if needs_trans {
                self.trans_to_ready()
            };
            op
        }

        fn combine_command(&self, op: KeyOp) -> KeyOp {
            let valid_num = |n| if n == 0 { 1 } else { n };
            match op {
                KeyOp::MoveDown(_) => KeyOp::MoveDown(valid_num(self.number)),
                KeyOp::MoveUp(_) => KeyOp::MoveUp(valid_num(self.number)),
                KeyOp::MoveLeft(_) => KeyOp::MoveLeft(valid_num(self.number)),
                KeyOp::MoveRight(_) => KeyOp::MoveRight(valid_num(self.number)),
                KeyOp::MoveDownHalfPages(_) => KeyOp::MoveDownHalfPages(valid_num(self.number)),
                KeyOp::MoveUpHalfPages(_) => KeyOp::MoveUpHalfPages(valid_num(self.number)),
                KeyOp::MoveDownPages(_) => KeyOp::MoveDownPages(valid_num(self.number)),
                KeyOp::MoveUpPages(_) => KeyOp::MoveUpPages(valid_num(self.number)),
                KeyOp::MoveToTopOfLines | KeyOp::MoveToBottomOfLines => {
                    if self.number == 0 {
                        op
                    } else {
                        KeyOp::MoveToLineNumber(self.number - 1)
                    }
                },
                KeyOp::MoveToLineNumber(_) => KeyOp::MoveToLineNumber(valid_num(self.number)),
                KeyOp::IncrementLines(_) => KeyOp::IncrementLines(valid_num(self.number)),
                KeyOp::DecrementLines(_) => KeyOp::DecrementLines(valid_num(self.number)),
                KeyOp::SetNumOfLines(_) => KeyOp::SetNumOfLines(valid_num(self.number)),
                _ => op,
            }
        }

        fn trans(&mut self, c: char) -> Option<KeyOp> {
            match self.state {
                State::Ready => self.action_ready(c),
                State::IncSearching => self.action_incsearching(c),
                State::Numbering => self.action_numbering(c),
                State::Commanding => self.action_commanding(c),
            }
        }
    }

    impl<'a> KeyParser for KeyBind<'a> {
        fn parse(&mut self, c: char) -> Option<KeyOp> {
            self.trans(c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use keyevt::KeyOp;

    #[test]
    fn test_keybind_command() {
        let mut kb = default::KeyBind::new();

        // normal commands
        assert_eq!(kb.parse('j'), Some(KeyOp::MoveDown(1)));
        assert_eq!(kb.parse('k'), Some(KeyOp::MoveUp(1)));
        assert_eq!(kb.parse('h'), Some(KeyOp::MoveLeft(1)));
        assert_eq!(kb.parse('l'), Some(KeyOp::MoveRight(1)));
        assert_eq!(kb.parse('d'), Some(KeyOp::MoveDownHalfPages(1)));
        assert_eq!(kb.parse('u'), Some(KeyOp::MoveUpHalfPages(1)));
        assert_eq!(kb.parse('f'), Some(KeyOp::MoveDownPages(1)));
        assert_eq!(kb.parse('b'), Some(KeyOp::MoveUpPages(1)));
        assert_eq!(kb.parse('0'), Some(KeyOp::MoveToHeadOfLine));
        assert_eq!(kb.parse('$'), Some(KeyOp::MoveToEndOfLine));

        assert_eq!(kb.parse('g'), None);
        assert_eq!(kb.parse('g'), Some(KeyOp::MoveToTopOfLines));

        assert_eq!(kb.parse('G'), Some(KeyOp::MoveToBottomOfLines));
        assert_eq!(kb.parse('-'), Some(KeyOp::DecrementLines(1)));
        assert_eq!(kb.parse('+'), Some(KeyOp::IncrementLines(1)));
        assert_eq!(kb.parse('n'), Some(KeyOp::SearchNext));
        assert_eq!(kb.parse('N'), Some(KeyOp::SearchPrev));
        assert_eq!(kb.parse('q'), Some(KeyOp::Quit));
        assert_eq!(kb.parse('#'), Some(KeyOp::ToggleLineNumberPrinting));

        assert_eq!(kb.parse('g'), Some(KeyOp::Message(Some("g".to_owned()))));
        assert_eq!(kb.parse('x'), Some(KeyOp::Message(None)));

        assert_eq!(kb.parse('\x1b'), Some(KeyOp::Cancel));
    }

    #[test]
    fn test_keybind_number() {
        let mut kb = default::KeyBind::new();

        // normal commands
        assert_eq!(kb.parse('1'), Some(KeyOp::Message(Some("1".to_owned()))));
        assert_eq!(kb.parse('2'), Some(KeyOp::Message(Some("12".to_owned()))));
        assert_eq!(kb.parse('\n'), Some(KeyOp::Message(None)));

        assert_eq!(kb.parse('1'), Some(KeyOp::Message(Some("1".to_owned()))));
        assert_eq!(kb.parse('2'), Some(KeyOp::Message(Some("12".to_owned()))));
        assert_eq!(kb.parse('\x1b'), Some(KeyOp::Message(None)));

        assert_eq!(kb.parse('2'), Some(KeyOp::Message(Some("2".to_owned()))));
        assert_eq!(kb.parse('j'), Some(KeyOp::MoveDown(2)));

        assert_eq!(kb.parse('1'), Some(KeyOp::Message(Some("1".to_owned()))));
        assert_eq!(kb.parse('0'), Some(KeyOp::Message(Some("10".to_owned()))));
        assert_eq!(kb.parse('h'), Some(KeyOp::MoveLeft(10)));
    }

    #[test]
    fn test_keybind_search() {
        let mut kb = default::KeyBind::new();

        // search commands
        assert_eq!(kb.parse('/'), Some(KeyOp::Message(Some("/".to_owned()))));
        assert_eq!(
            kb.parse('w'),
            Some(KeyOp::SearchIncremental("w".to_owned()))
        );
        assert_eq!(
            kb.parse('o'),
            Some(KeyOp::SearchIncremental("wo".to_owned()))
        );
        assert_eq!(
            kb.parse('r'),
            Some(KeyOp::SearchIncremental("wor".to_owned()))
        );
        assert_eq!(
            kb.parse('d'),
            Some(KeyOp::SearchIncremental("word".to_owned()))
        );
        assert_eq!(kb.parse('\n'), Some(KeyOp::SearchTrigger));

        assert_eq!(kb.parse('/'), Some(KeyOp::Message(Some("/".to_owned()))));
        assert_eq!(
            kb.parse('a'),
            Some(KeyOp::SearchIncremental("a".to_owned()))
        );
        assert_eq!(
            kb.parse('b'),
            Some(KeyOp::SearchIncremental("ab".to_owned()))
        );
        assert_eq!(
            kb.parse('\x08'),
            Some(KeyOp::SearchIncremental("a".to_owned()))
        );
        assert_eq!(
            kb.parse('\x08'),
            Some(KeyOp::SearchIncremental("".to_owned()))
        );
        assert_eq!(
            kb.parse('\x08'),
            Some(KeyOp::SearchIncremental("".to_owned()))
        );
        assert_eq!(
            kb.parse('w'),
            Some(KeyOp::SearchIncremental("w".to_owned()))
        );
        assert_eq!(
            kb.parse('o'),
            Some(KeyOp::SearchIncremental("wo".to_owned()))
        );
        assert_eq!(kb.parse('\n'), Some(KeyOp::SearchTrigger));
        assert_eq!(kb.parse('n'), Some(KeyOp::SearchNext));
        assert_eq!(kb.parse('N'), Some(KeyOp::SearchPrev));
    }
}
