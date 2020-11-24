#[macro_use]
extern crate smart_default;

use asciify::AsciiBuilder;
use itertools::Itertools;

pub mod output;

/// returns an `Option` of the expression passed in
/// `None` if the `is_empty` on the expression returns true, `Some(x)` otherwise
/// this is a macro and not a function because `is_empty` is not defined in any
/// trait
#[macro_export]
macro_rules! none_if_empty {
    ($x:expr) => {
        if $x.is_empty() {
            None
        } else {
            Some($x)
        }
    };
}

pub struct AsciiConfig {
    pub size: Option<u32>,
    pub colored: bool,
    pub deep: bool,
    pub invert: bool,
}

impl AsciiConfig {
    pub fn apply(&self, mut builder: AsciiBuilder) -> AsciiBuilder {
        if let Some(n) = self.size {
            builder = builder.set_resize((n * 2, n))
        }
        builder.set_deep(self.deep).set_invert(self.invert)
    }
}

pub fn remove_formatting(s: &str) -> String {
    let chars = s.char_indices().rev();
    let mut buf = s.to_owned();
    for c in chars {
        if c.1 == '§' {
            buf.remove(c.0);
            if c.0 < buf.len() {
                buf.remove(c.0);
            }
        }
    }
    buf
}

/// formats a iterator to a readable list
///
/// if `second_column`, the right strings will also be displayed
pub fn get_table<'a>(
    entries: impl Iterator<Item = (&'a str, &'a str)> + Clone,
    second_column: bool,
) -> String {
    // the width at which | characters should be placed this is the length of the
    // longest entry
    let max_width = if second_column {
        entries.clone().map(|m| m.0.len()).max().unwrap_or_default()
    } else {
        // this will not be used in case second_column is off so we just use 0
        0
    };

    entries
        .map(|m| {
            if second_column {
                format!("{: <width$} | {}", m.0, m.1, width = max_width)
            } else {
                m.0.to_owned()
            }
        })
        .intersperse("\n".to_owned())
        .collect()
}
