//! unicode_divide module

use std::io;
use std::io::{Seek, SeekFrom};
use unicode_width::UnicodeWidthChar;

/// Unicode String Divider
pub struct UnicodeStrDivider<'a> {
    inner: &'a str, // raw str
    prev_pos: usize,
    pos: usize,   // array index
    width: usize, // display width at once
}

impl<'a> UnicodeStrDivider<'a> {
    pub fn new(line: &'a str, width: usize) -> Self {
        Self {
            inner: line,
            prev_pos: 0,
            pos: 0,
            width,
        }
    }

    pub fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    pub fn last_range(&self) -> (usize, usize) {
        (self.prev_pos, self.pos)
    }
}

/// Count unicode chars and Return range-end index of unicode_str
///
/// unicode_str is source str, and visual_width is required width of unicode string.
fn unicode_index_of_width(unicode_str: &str, visual_width: usize) -> Option<usize> {
    let mut interrupted = false;
    if let Some(end) = unicode_str
        .char_indices()
        .map(|(i, c)| (i, c.width_cjk()))
        .scan(0, |sum, (i, w)| {
            if interrupted {
                None
            } else {
                *sum += w.unwrap_or(0);
                if *sum > visual_width {
                    interrupted = true;
                }
                Some(i)
            }
        }).last()
    {
        let end = if interrupted {
            end
        } else {
            // reach the end of text
            unicode_str.len()
        };
        Some(end)
    } else {
        None
    }
}

impl<'a> Iterator for UnicodeStrDivider<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.inner.len() {
            None
        } else if let Some(end) = unicode_index_of_width(&self.inner[self.pos..], self.width) {
            let start = self.pos;
            let end = start + end;
            self.prev_pos = self.pos;
            self.pos = end;
            Some(&self.inner[start..end])
        } else {
            None
        }
    }
}

impl<'a> Seek for UnicodeStrDivider<'a> {
    fn seek(&mut self, sf: SeekFrom) -> io::Result<u64> {
        match sf {
            SeekFrom::Start(n) => Ok(if let Some(dist) =
                unicode_index_of_width(self.inner, n as usize)
            {
                self.pos = dist;
                self.prev_pos = self.pos;
                dist as u64
            } else {
                0u64
            }),
            SeekFrom::End(_) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "SeekFrom::End is not supported",
            )),
            SeekFrom::Current(n) => Ok(if let Some(dist) =
                unicode_index_of_width(&self.inner[self.pos..], n as usize)
            {
                self.pos += dist;
                self.prev_pos = self.pos;
                dist as u64
            } else {
                0u64
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iterator() {
        let ascii_sentence = "1234567890";
        let unicode_sentence = "あいうえお";

        let mut ucdiv = UnicodeStrDivider::new(&ascii_sentence, 2);
        assert_eq!(ucdiv.next().unwrap(), "12");
        assert_eq!(ucdiv.next().unwrap(), "34");
        assert_eq!(ucdiv.next().unwrap(), "56");
        assert_eq!(ucdiv.next().unwrap(), "78");
        assert_eq!(ucdiv.next().unwrap(), "90");
        assert_eq!(ucdiv.next(), None);

        let mut ucdiv = UnicodeStrDivider::new(&unicode_sentence, 2);
        assert_eq!(ucdiv.next().unwrap(), "あ");
        assert_eq!(ucdiv.next().unwrap(), "い");
        assert_eq!(ucdiv.next().unwrap(), "う");
        assert_eq!(ucdiv.next().unwrap(), "え");
        assert_eq!(ucdiv.next().unwrap(), "お");
        assert_eq!(ucdiv.next(), None);

        let mut ucdiv = UnicodeStrDivider::new(&unicode_sentence, 4);
        assert_eq!(ucdiv.next().unwrap(), "あい");
        assert_eq!(ucdiv.next().unwrap(), "うえ");
        assert_eq!(ucdiv.next().unwrap(), "お");
        assert_eq!(ucdiv.next(), None);
    }

    #[test]
    fn test_seek() {
        let ascii_sentence = "1234567890";
        let unicode_sentence = "あいうえお";

        let mut ucdiv = UnicodeStrDivider::new(&ascii_sentence, 2);
        assert_eq!(ucdiv.next().unwrap(), "12");
        assert!(ucdiv.seek(SeekFrom::Start(0)).is_ok());
        assert_eq!(ucdiv.next().unwrap(), "12");
        assert!(ucdiv.seek(SeekFrom::Current(5)).is_ok());
        assert_eq!(ucdiv.next().unwrap(), "89");
        assert_eq!(ucdiv.next().unwrap(), "0");
        assert_eq!(ucdiv.next(), None);
        assert!(ucdiv.seek(SeekFrom::Start(1)).is_ok());
        assert_eq!(ucdiv.next().unwrap(), "23");

        let mut ucdiv = UnicodeStrDivider::new(&unicode_sentence, 2);
        assert_eq!(ucdiv.next().unwrap(), "あ");
        assert!(ucdiv.seek(SeekFrom::Start(1)).is_ok());
        assert_eq!(ucdiv.next().unwrap(), "あ");
        assert!(ucdiv.seek(SeekFrom::Current(2)).is_ok());
        assert_eq!(ucdiv.next().unwrap(), "う");
        assert!(ucdiv.seek(SeekFrom::Current(10)).is_ok());
        assert_eq!(ucdiv.next(), None);
        assert!(ucdiv.seek(SeekFrom::Start(3)).is_ok());
        assert_eq!(ucdiv.next().unwrap(), "い");

        assert!(ucdiv.seek(SeekFrom::End(0)).is_err());
    }
}
