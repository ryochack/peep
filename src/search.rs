use regex::{self, Regex};
use std::io;

#[derive(Clone, Debug, PartialEq)]
pub struct Match<'t> {
    text: &'t str,
    start: usize,
    end: usize
}

impl<'t> Match<'t> {
    #[inline]
    pub fn start(&self) -> usize {
        self.start
    }
    #[inline]
    pub fn end(&self) -> usize {
        self.end
    }
    #[inline]
    pub fn as_str(&self) -> &'t str {
        &self.text
    }
    fn new(haystack: &'t str, start: usize, end: usize) -> Match<'t> {
        Match {
            text: haystack,
            start: start,
            end: end,
        }
    }
}

#[derive(Debug)]
struct MatchIter<'t> {
    matches: Vec<Match<'t>>,
    index: usize,
}

impl<'t> Iterator for MatchIter<'t> {
    type Item = Match<'t>;

    fn next(&mut self) -> Option<Match<'t>> {
        if self.index >= self.matches.len() {
            return None;
        }
        let m = self.matches[self.index].clone();
        self.index += 1;
        Some(m)
    }
}

trait Search<'t> {
    fn update_pattern(&mut self, &str) -> io::Result<()>;
    fn find(&self, text: &'t str) -> Option<Match<'t>>;
    fn find_iter(&self, &'t str) -> MatchIter<'t>;
}

struct PlaneSearcher {
    pat: String,
}

impl PlaneSearcher {
    fn new() -> Self {
        Self {
            pat: String::new(),
        }
    }
}

impl<'t> Search<'t> for PlaneSearcher {
    fn update_pattern(&mut self, pat: &str) -> io::Result<()> {
        self.pat = pat.to_owned();
        Ok(())
    }

    fn find(&self, text: &'t str) -> Option<Match<'t>> {
        if let Some(start) = text.find(&self.pat) {
            Some(Match {
                text: &text[start..(start+self.pat.len())],
                start: start,
                end: start + self.pat.len(),
            })
        } else {
            None
        }
    }

    fn find_iter(&self, text: &'t str) -> MatchIter<'t> {
        MatchIter {
            matches: text.match_indices(&self.pat)
                .map(|(i,_)| {
                    Match {
                        text: &text[i..(i+self.pat.len())],
                        start: i,
                        end: i + self.pat.len(),
                    }
                }).collect(),
            index: 0,
        }
    }
}

struct RegexSearcher {
    pat: Regex,
}

impl RegexSearcher {
    fn new() -> Self {
        Self {
            pat: Regex::new("").unwrap(),
        }
    }
}

impl<'t> Search<'t> for RegexSearcher {
    fn update_pattern(&mut self, pat: &str) -> io::Result<()> {
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

    fn find(&self, text: &'t str) -> Option<Match<'t>> {
        if let Some(m) = &self.pat.find(text) {
            Some(Match {
                text: m.as_str(),
                start: m.start(),
                end: m.end(),
            })
        } else {
            None
        }
    }

    fn find_iter(&self, text: &'t str) -> MatchIter<'t> {
        MatchIter {
            matches: self.pat.find_iter(text)
                .map(|m| {
                    Match {
                        text: m.as_str(),
                        start: m.start(),
                        end: m.end(),
                    }
                }).collect(),
            index: 0,
        }
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
        assert_eq!(searcher.update_pattern(&pat).unwrap(), ());
        assert_eq!(
            searcher.find(&text).unwrap(),
            Match { text: &pat, start: 1, end: 4 }
        );
        let mut matches = searcher.find_iter(&text);
        assert_eq!(
            matches.next().unwrap(),
            Match { text: &pat, start: 1, end: 4 }
        );
        assert_eq!(
            matches.next().unwrap(),
            Match { text: &pat, start: 4, end: 7 }
        );
        assert_eq!(
            matches.next().unwrap(),
            Match { text: &pat, start: 10, end: 13 }
        );
        assert!(matches.next().is_none());

        let pat = "";
        let text = "xabcabcwowabc";
        assert_eq!(searcher.update_pattern(&pat).unwrap(), ());
        assert_eq!(
            searcher.find(&text).unwrap(),
            Match { text: &pat, start: 0, end: 0 }
        );
        let mut matches = searcher.find_iter(&text);
        for i in 0..text.len() {
            assert_eq!(
                matches.next().unwrap(),
                Match { text: &pat, start: i, end: i }
            );
        }

        let pat = "abc";
        let text = "";
        assert_eq!(searcher.update_pattern(&pat).unwrap(), ());
        assert!(searcher.find(&text).is_none());
        let mut matches = searcher.find_iter(&text);
        assert!(matches.next().is_none());
    }

    #[test]
    fn test_regex() {
        let pat = r"a\wc";
        let text = "xabcabcwowabc";
        let expects = "abc";

        let mut searcher = RegexSearcher::new();
        assert_eq!(searcher.update_pattern(&pat).unwrap(), ());

        assert_eq!(
            searcher.find(&text).unwrap(),
            Match { text: &expects, start: 1, end: 4 }
        );

        let mut matches = searcher.find_iter(&text);
        assert_eq!(
            matches.next().unwrap(),
            Match { text: &expects, start: 1, end: 4 }
        );
        assert_eq!(
            matches.next().unwrap(),
            Match { text: &expects, start: 4, end: 7 }
        );
        assert_eq!(
            matches.next().unwrap(),
            Match { text: &expects, start: 10, end: 13 }
        );

        let pat = "";
        let text = "xabcabcwowabc";
        assert_eq!(searcher.update_pattern(&pat).unwrap(), ());
        assert_eq!(
            searcher.find(&text).unwrap(),
            Match { text: &pat, start: 0, end: 0 }
        );
        let mut matches = searcher.find_iter(&text);
        for i in 0..text.len() {
            assert_eq!(
                matches.next().unwrap(),
                Match { text: &pat, start: i, end: i }
            );
        }

        let pat = r"a\wc";
        let text = "";
        assert_eq!(searcher.update_pattern(&pat).unwrap(), ());
        assert!(searcher.find(&text).is_none());
        let mut matches = searcher.find_iter(&text);
        assert!(matches.next().is_none());
    }
}
