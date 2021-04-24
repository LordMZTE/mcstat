#[macro_use]
extern crate smart_default;

use crate::output::Table;
use asciify::AsciiBuilder;
use itertools::Itertools;
use std::io::Cursor;

pub mod output;

/// returns an `Option` of the expression passed in
/// `None` if the `is_empty` on the expression returns true, `Some(x)` otherwise
/// this is a macro and not a function because `is_empty` is not defined in any
/// trait
#[macro_export]
macro_rules! none_if_empty {
    ($x:expr) => {{
        // let binding to avoid copying
        let x = $x;
        if x.is_empty() {
            None
        } else {
            Some(x)
        }
    }};
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
    if second_column {
        let mut table = Table::new();
        for entry in entries {
            table.small_entry(entry.0, entry.1);
        }
        let mut cursor = Cursor::new(Vec::<u8>::new());
        table.print(&mut cursor).unwrap();
        String::from_utf8(cursor.into_inner()).unwrap()
    } else {
        // this syntax is used due to a nightly function which will be added to rust
        // also called intersperse
        Itertools::intersperse(entries.map(|x| x.0), "\n").collect()
    }
}
