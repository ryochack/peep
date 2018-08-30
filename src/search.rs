use regex::{self, Regex};
use std::io;

#[derive(Clone, Debug, PartialEq)]
pub struct Match {
    start: usize,
    end: usize,
}

impl Match {
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
    fn new(start: usize, end: usize) -> Match {
        Match { start, end }
    }
}

#[derive(Debug)]
pub struct MatchIter {
    matches: Vec<Match>,
    index: usize,
}

impl Iterator for MatchIter {
    type Item = Match;

    fn next(&mut self) -> Option<Match> {
        if self.index >= self.matches.len() {
            return None;
        }
        let m = self.matches[self.index].clone();
        self.index += 1;
        Some(m)
    }
}

pub trait Search {
    fn as_str(&self) -> &str;
    fn find(&self, text: &str) -> Option<Match>;
    fn find_iter(&self, text: &str) -> MatchIter;
    fn set_pattern(&mut self, pat: &str) -> io::Result<()>;
}

#[derive(Default)]
pub struct NullSearcher;

impl NullSearcher {
    pub fn new() -> Self {
        NullSearcher {}
    }
}

impl Search for NullSearcher {
    fn as_str(&self) -> &str {
        &""
    }

    fn find(&self, _text: &str) -> Option<Match> {
        None
    }

    fn find_iter(&self, _text: &str) -> MatchIter {
        MatchIter {
            matches: Vec::new(),
            index: 0,
        }
    }

    fn set_pattern(&mut self, _pat: &str) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub struct PlaneSearcher {
    pat: String,
}

impl PlaneSearcher {
    pub fn new() -> Self {
        Self { pat: String::new() }
    }
}

impl Search for PlaneSearcher {
    fn as_str(&self) -> &str {
        self.pat.as_str()
    }

    fn find(&self, text: &str) -> Option<Match> {
        if let Some(start) = text.find(&self.pat) {
            Some(Match::new(start, start + self.pat.len()))
        } else {
            None
        }
    }

    fn find_iter(&self, text: &str) -> MatchIter {
        MatchIter {
            matches: text
                .match_indices(&self.pat)
                .map(|(i, _)| Match::new(i, i + self.pat.len()))
                .collect(),
            index: 0,
        }
    }

    fn set_pattern(&mut self, pat: &str) -> io::Result<()> {
        self.pat = pat.to_owned();
        Ok(())
    }
}

#[derive(Clone)]
pub struct RegexSearcher {
    pat: Regex,
}

impl RegexSearcher {
    pub fn new(pat: &str) -> Self {
        Self {
            pat: Regex::new(pat).unwrap(),
        }
    }
}

impl Default for RegexSearcher {
    fn default() -> Self {
        Self::new("")
    }
}

impl Search for RegexSearcher {
    fn as_str(&self) -> &str {
        self.pat.as_str()
    }

    fn find(&self, text: &str) -> Option<Match> {
        if let Some(m) = &self.pat.find(text) {
            Some(Match::new(m.start(), m.end()))
        } else {
            None
        }
    }

    fn find_iter(&self, text: &str) -> MatchIter {
        MatchIter {
            matches: self
                .pat
                .find_iter(text)
                .map(|m| Match::new(m.start(), m.end()))
                .collect(),
            index: 0,
        }
    }

    fn set_pattern(&mut self, pat: &str) -> io::Result<()> {
        let a = Regex::new(pat);
        if a.is_err() {
            // convert error from regex::Error to io::Error
            // TODO: unify pane error list
            return match a.unwrap_err() {
                regex::Error::Syntax(_s) => {
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "Syntax error"))
                }
                regex::Error::CompiledTooBig(_n) => Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Compiled too big",
                )),
                _ => Err(io::Error::new(io::ErrorKind::Other, "Unknown regex error")),
            };
        }
        self.pat = a.unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plane() {
        let pat = "abc";
        let text = "xabcabcwowabc";

        let mut searcher = PlaneSearcher::new();
        assert_eq!(searcher.set_pattern(&pat).unwrap(), ());
        assert_eq!(searcher.find(&text).unwrap(), Match::new(1, 4));
        let mut matches = searcher.find_iter(&text);
        assert_eq!(matches.next().unwrap(), Match::new(1, 4));
        assert_eq!(matches.next().unwrap(), Match::new(4, 7));
        assert_eq!(matches.next().unwrap(), Match::new(10, 13));
        assert!(matches.next().is_none());

        let pat = "";
        let text = "xabcabcwowabc";
        assert_eq!(searcher.set_pattern(&pat).unwrap(), ());
        assert_eq!(searcher.find(&text).unwrap(), Match::new(0, 0));
        let mut matches = searcher.find_iter(&text);
        for i in 0..text.len() {
            assert_eq!(matches.next().unwrap(), Match::new(i, i));
        }

        let pat = "abc";
        let text = "";
        assert_eq!(searcher.set_pattern(&pat).unwrap(), ());
        assert!(searcher.find(&text).is_none());
        let mut matches = searcher.find_iter(&text);
        assert!(matches.next().is_none());
    }

    #[test]
    fn test_regex() {
        let pat = r"a\wc";
        let text = "xabcabcwowabc";

        let mut searcher = RegexSearcher::new();
        assert_eq!(searcher.set_pattern(&pat).unwrap(), ());

        assert_eq!(searcher.find(&text).unwrap(), Match::new(1, 4));

        let mut matches = searcher.find_iter(&text);
        assert_eq!(matches.next().unwrap(), Match::new(1, 4));
        assert_eq!(matches.next().unwrap(), Match::new(4, 7));
        assert_eq!(matches.next().unwrap(), Match::new(10, 13));

        let pat = "";
        let text = "xabcabcwowabc";
        assert_eq!(searcher.set_pattern(&pat).unwrap(), ());
        assert_eq!(searcher.find(&text).unwrap(), Match::new(0, 0));
        let mut matches = searcher.find_iter(&text);
        for i in 0..text.len() {
            assert_eq!(matches.next().unwrap(), Match::new(i, i));
        }

        let pat = r"a\wc";
        let text = "";
        assert_eq!(searcher.set_pattern(&pat).unwrap(), ());
        assert!(searcher.find(&text).is_none());
        let mut matches = searcher.find_iter(&text);
        assert!(matches.next().is_none());
    }
}
