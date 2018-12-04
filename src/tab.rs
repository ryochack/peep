//! tab module

use unicode_width::UnicodeWidthChar;

pub struct TabExpander {
    tab: String,
}

impl TabExpander {
    fn generate_continuous_spaces(width: u16) -> String {
        let mut spaces = String::new();
        for _ in 0..width {
            spaces.push(' ');
        }
        spaces
    }

    pub fn new(tab_width: u16) -> Self {
        Self {
            tab: Self::generate_continuous_spaces(tab_width),
        }
    }

    pub fn update_width(&mut self, tab_width: u16) {
        self.tab = Self::generate_continuous_spaces(tab_width);
    }

    /// Replace TAB with spaces with considering TAB position
    pub fn expand(&self, from: &str) -> String {
        let mut to = String::new();
        let mut expand_width = 0;

        for c in from.chars() {
            expand_width += if c == '\t' {
                if self.tab.len() > 0 {
                    let frac = self.tab.len() - (expand_width % self.tab.len());
                    to.push_str(&self.tab[0..frac]);
                    frac
                } else {
                    0
                }
            } else {
                c.width_cjk().map_or(0, |w| {
                    to.push(c);
                    w
                })
            }
        }
        to
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab() {
        let tab = TabExpander::new(4);
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
            assert_eq!(tab.expand(d.0), d.1);
        }
    }
}
