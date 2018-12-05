//! tab module

use unicode_width::UnicodeWidthChar;

const TAB_SPACE: &str = "                                ";

pub trait TabExpand {
    fn expand_tab(&self, tab_width: usize) -> String;
}

impl TabExpand for str {
    fn expand_tab(&self, tab_width: usize) -> String {
        let tab_width = if tab_width > TAB_SPACE.len() {
            TAB_SPACE.len()
        } else {
            tab_width
        };

        let mut expanded_str = String::new();
        let mut expand_width = 0;

        for c in self.chars() {
            expand_width += if c == '\t' {
                if tab_width > 0 {
                    let frac = tab_width - (expand_width % tab_width);
                    expanded_str.push_str(&TAB_SPACE[0..frac]);
                    frac
                } else {
                    0
                }
            } else {
                c.width_cjk().map_or(0, |w| {
                    expanded_str.push(c);
                    w
                })
            }
        }
        expanded_str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab() {
        let data = [
            ("\t56789", "    56789"),
            ("1\t56789", "1   56789"),
            ("12\t56789", "12  56789"),
            ("123\t56789", "123 56789"),
            ("1234\t9", "1234    9"),
            ("12345\t9", "12345   9"),
            ("123456\t9", "123456  9"),
            ("1234567\t9", "1234567 9"),
            ("12345678\t", "12345678    "),
            ("\t\t", "        "),
            ("1\t\t", "1       "),
            ("12\t\t", "12      "),
            ("123\t\t", "123     "),
            ("123\t\t9", "123     9"),
            ("123\t5\t9", "123 5   9"),
        ];
        for d in data.iter() {
            assert_eq!(d.0.expand_tab(4), d.1);
        }
    }
}
