/// Key Bind Parser
use event::PeepEvent;

pub trait KeyParser {
    fn parse(&mut self, c: char) -> Option<PeepEvent>;
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
        cmap: HashMap<&'a str, PeepEvent>,
    }

    impl<'a> Default for KeyBind<'a> {
        fn default() -> Self {
            Self::new()
        }
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

        fn default_command_table() -> HashMap<&'a str, PeepEvent> {
            // let mut default: HashMap<&str, PeepEvent> = [
            [
                ("j", PeepEvent::MoveDown(1)),
                ("k", PeepEvent::MoveUp(1)),
                ("h", PeepEvent::MoveLeft(1)),
                ("l", PeepEvent::MoveRight(1)),
                ("d", PeepEvent::MoveDownHalfPages(1)),
                ("u", PeepEvent::MoveUpHalfPages(1)),
                ("H", PeepEvent::MoveLeftHalfPages(1)),
                ("L", PeepEvent::MoveRightHalfPages(1)),
                ("f", PeepEvent::MoveDownPages(1)),
                ("b", PeepEvent::MoveUpPages(1)),
                ("0", PeepEvent::MoveToHeadOfLine),
                ("$", PeepEvent::MoveToEndOfLine),
                ("g", PeepEvent::MoveToTopOfLines),
                ("G", PeepEvent::MoveToBottomOfLines),
                ("#", PeepEvent::ToggleLineNumberPrinting),
                ("-", PeepEvent::DecrementLines(1)),
                ("+", PeepEvent::IncrementLines(1)),
                ("=", PeepEvent::SetNumOfLines(0)),
                ("n", PeepEvent::SearchNext),
                ("N", PeepEvent::SearchPrev),
                ("q", PeepEvent::Quit),
                ("F", PeepEvent::FollowMode),
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

        fn action_ready(&mut self, c: char) -> Option<PeepEvent> {
            match c {
                '/' => {
                    self.trans_to_incsearching();
                    Some(PeepEvent::SearchIncremental("".to_owned()))
                }
                '1'...'9' => {
                    self.trans_to_numbering(c);
                    // Some(PeepEvent::Message(Some(self.number.to_string())))
                    None
                }
                c if !c.is_control() => {
                    self.trans_to_commanding();
                    self.action_commanding(c)
                }
                // ESC
                '\x1b' => Some(PeepEvent::Cancel),
                _ => None,
            }
        }
        fn action_incsearching(&mut self, c: char) -> Option<PeepEvent> {
            match c {
                c if !c.is_control() => {
                    self.wip_keys.push(c);
                    Some(PeepEvent::SearchIncremental(self.wip_keys.to_owned()))
                }
                '\x08' | '\x7f' => {
                    // BackSpace, Delete
                    if self.wip_keys.pop().is_none() {
                        self.trans_to_ready();
                        Some(PeepEvent::Cancel)
                    } else {
                        Some(PeepEvent::SearchIncremental(self.wip_keys.to_owned()))
                    }
                }
                '\n' => {
                    // LF
                    self.trans_to_ready();
                    Some(PeepEvent::SearchTrigger)
                }
                '\x1b' => {
                    // ESC -> Cancel
                    self.trans_to_ready();
                    Some(PeepEvent::Cancel)
                }
                _ => None,
            }
        }
        fn action_numbering(&mut self, c: char) -> Option<PeepEvent> {
            match c {
                '0'...'9' => {
                    self.number = self.number * 10 + c.to_digit(10).unwrap() as u16;
                    // Some(PeepEvent::Message(Some(self.number.to_string())))
                    None
                }
                c if !c.is_control() => {
                    self.trans_to_commanding();
                    self.action_commanding(c)
                }
                '\x1b' | '\n' => {
                    // ESC and LF -> Cancel
                    self.trans_to_ready();
                    None
                    // Some(PeepEvent::Message(None))
                }
                _ => None,
            }
        }
        fn action_commanding(&mut self, c: char) -> Option<PeepEvent> {
            let mut needs_trans = false;
            let op = match c {
                c if !c.is_control() => {
                    self.wip_keys.push(c);
                    match self.cmap.get::<str>(&self.wip_keys) {
                        Some(v) => {
                            needs_trans = true;
                            self.combine_command(v.to_owned())
                        }
                        None => {
                            if self.cmap.keys().any(|&k| k.starts_with(&self.wip_keys)) {
                                // has candidates
                                None
                            } else {
                                // not exist => cancel
                                needs_trans = true;
                                Some(PeepEvent::Message(None))
                            }
                        }
                    }
                }
                _ => {
                    needs_trans = true;
                    Some(PeepEvent::Message(None))
                }
            };
            if needs_trans {
                self.trans_to_ready()
            };
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
                PeepEvent::MoveUpHalfPages(_) => Some(PeepEvent::MoveUpHalfPages(valid_num(self.number))),
                PeepEvent::MoveLeftHalfPages(_) => {
                    Some(PeepEvent::MoveLeftHalfPages(valid_num(self.number)))
                }
                PeepEvent::MoveRightHalfPages(_) => {
                    Some(PeepEvent::MoveRightHalfPages(valid_num(self.number)))
                }
                PeepEvent::MoveDownPages(_) => Some(PeepEvent::MoveDownPages(valid_num(self.number))),
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
                PeepEvent::IncrementLines(_) => Some(PeepEvent::IncrementLines(valid_num(self.number))),
                PeepEvent::DecrementLines(_) => Some(PeepEvent::DecrementLines(valid_num(self.number))),
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

        fn trans(&mut self, c: char) -> Option<PeepEvent> {
            match self.state {
                State::Ready => self.action_ready(c),
                State::IncSearching => self.action_incsearching(c),
                State::Numbering => self.action_numbering(c),
                State::Commanding => self.action_commanding(c),
            }
        }
    }

    impl<'a> KeyParser for KeyBind<'a> {
        fn parse(&mut self, c: char) -> Option<PeepEvent> {
            self.trans(c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use event::PeepEvent;

    #[test]
    fn test_keybind_command() {
        let mut kb = default::KeyBind::new();

        // normal commands
        assert_eq!(kb.parse('j'), Some(PeepEvent::MoveDown(1)));
        assert_eq!(kb.parse('k'), Some(PeepEvent::MoveUp(1)));
        assert_eq!(kb.parse('h'), Some(PeepEvent::MoveLeft(1)));
        assert_eq!(kb.parse('l'), Some(PeepEvent::MoveRight(1)));
        assert_eq!(kb.parse('d'), Some(PeepEvent::MoveDownHalfPages(1)));
        assert_eq!(kb.parse('u'), Some(PeepEvent::MoveUpHalfPages(1)));
        assert_eq!(kb.parse('f'), Some(PeepEvent::MoveDownPages(1)));
        assert_eq!(kb.parse('b'), Some(PeepEvent::MoveUpPages(1)));
        assert_eq!(kb.parse('0'), Some(PeepEvent::MoveToHeadOfLine));
        assert_eq!(kb.parse('$'), Some(PeepEvent::MoveToEndOfLine));
        assert_eq!(kb.parse('g'), Some(PeepEvent::MoveToTopOfLines));
        assert_eq!(kb.parse('G'), Some(PeepEvent::MoveToBottomOfLines));
        assert_eq!(kb.parse('-'), Some(PeepEvent::DecrementLines(1)));
        assert_eq!(kb.parse('+'), Some(PeepEvent::IncrementLines(1)));
        assert_eq!(kb.parse('='), None);
        assert_eq!(kb.parse('n'), Some(PeepEvent::SearchNext));
        assert_eq!(kb.parse('N'), Some(PeepEvent::SearchPrev));
        assert_eq!(kb.parse('q'), Some(PeepEvent::Quit));
        assert_eq!(kb.parse('#'), Some(PeepEvent::ToggleLineNumberPrinting));
        assert_eq!(kb.parse('F'), Some(PeepEvent::FollowMode));
        assert_eq!(kb.parse('\x1b'), Some(PeepEvent::Cancel));
    }

    #[test]
    fn test_keybind_number() {
        let mut kb = default::KeyBind::new();

        // normal commands
        assert_eq!(kb.parse('1'), None);
        assert_eq!(kb.parse('2'), None);
        assert_eq!(kb.parse('\n'), None);

        assert_eq!(kb.parse('1'), None);
        assert_eq!(kb.parse('2'), None);
        assert_eq!(kb.parse('\x1b'), None);

        assert_eq!(kb.parse('2'), None);
        assert_eq!(kb.parse('j'), Some(PeepEvent::MoveDown(2)));

        assert_eq!(kb.parse('1'), None);
        assert_eq!(kb.parse('0'), None);
        assert_eq!(kb.parse('h'), Some(PeepEvent::MoveLeft(10)));

        assert_eq!(kb.parse('1'), None);
        assert_eq!(kb.parse('0'), None);
        assert_eq!(kb.parse('='), Some(PeepEvent::SetNumOfLines(10)));
    }

    #[test]
    fn test_keybind_search() {
        let mut kb = default::KeyBind::new();

        // search commands
        assert_eq!(
            kb.parse('/'),
            Some(PeepEvent::SearchIncremental("".to_owned()))
        );
        assert_eq!(
            kb.parse('w'),
            Some(PeepEvent::SearchIncremental("w".to_owned()))
        );
        assert_eq!(
            kb.parse('o'),
            Some(PeepEvent::SearchIncremental("wo".to_owned()))
        );
        assert_eq!(
            kb.parse('r'),
            Some(PeepEvent::SearchIncremental("wor".to_owned()))
        );
        assert_eq!(
            kb.parse('d'),
            Some(PeepEvent::SearchIncremental("word".to_owned()))
        );
        assert_eq!(kb.parse('\n'), Some(PeepEvent::SearchTrigger));

        assert_eq!(
            kb.parse('/'),
            Some(PeepEvent::SearchIncremental("".to_owned()))
        );
        assert_eq!(
            kb.parse('a'),
            Some(PeepEvent::SearchIncremental("a".to_owned()))
        );
        assert_eq!(
            kb.parse('b'),
            Some(PeepEvent::SearchIncremental("ab".to_owned()))
        );
        assert_eq!(
            kb.parse('\x08'),
            Some(PeepEvent::SearchIncremental("a".to_owned()))
        );
        assert_eq!(
            kb.parse('\x08'),
            Some(PeepEvent::SearchIncremental("".to_owned()))
        );
        assert_eq!(kb.parse('\x08'), Some(PeepEvent::Cancel));

        assert_eq!(
            kb.parse('/'),
            Some(PeepEvent::SearchIncremental("".to_owned()))
        );
        assert_eq!(
            kb.parse('w'),
            Some(PeepEvent::SearchIncremental("w".to_owned()))
        );
        assert_eq!(
            kb.parse('o'),
            Some(PeepEvent::SearchIncremental("wo".to_owned()))
        );
        assert_eq!(kb.parse('\n'), Some(PeepEvent::SearchTrigger));
        assert_eq!(kb.parse('n'), Some(PeepEvent::SearchNext));
        assert_eq!(kb.parse('N'), Some(PeepEvent::SearchPrev));
    }
}
