use crossterm::{
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    ExecutableCommand,
};
use std::{
    cmp::max,
    io::{self, Write},
};
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct Table {
    pub entries: Vec<Box<dyn TableEntry>>,
    pub small_entry_width: usize,
}

impl Table {
    pub fn stdout(&self) -> io::Result<()> {
        self.print(&mut io::stdout())
    }

    pub fn print(&self, out: &mut dyn Write) -> io::Result<()> {
        for e in &self.entries {
            e.print(out, self)?;
        }

        Ok(())
    }

    pub fn new() -> Self {
        Default::default()
    }

    pub fn blank(&mut self) {
        self.entries.push(Box::new(BlankTableEntry));
    }

    pub fn small_entry(&mut self, name: impl ToString, val: impl TableContent + 'static) {
        let name = name.to_string();
        self.set_small_width(name.width());

        self.entries
            .push(Box::new(SmallTableEntry(name, Box::new(val))));
    }

    pub fn big_entry(&mut self, name: impl ToString, val: impl TableContent + 'static) {
        self.entries
            .push(Box::new(BigTableEntry::new(name.to_string(), val)));
    }

    fn set_small_width(&mut self, width: usize) {
        if width > self.small_entry_width {
            self.small_entry_width = width;
        }
    }
}

pub trait TableContent {
    fn width(&self) -> usize;
    fn write_to(&self, out: &mut dyn Write) -> io::Result<()>;
}

impl TableContent for String {
    fn width(&self) -> usize {
        self.lines().map(|s| s.width()).max().unwrap_or_default()
    }

    fn write_to(&self, out: &mut dyn Write) -> io::Result<()> {
        out.write_all(self.as_bytes())
    }
}

/// Table content of a pretty string with minecraft-formatted markup
pub struct McFormatContent(pub String);

impl McFormatContent {
    // compatibility with the `none_if_empty` macro
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl TableContent for McFormatContent {
    fn width(&self) -> usize {
        self.0
            .lines()
            .map(|l| {
                // need to count chars because of § being 2 bytes
                l.chars().count() - l.matches('§').count() * 2
            })
            .max()
            .unwrap_or_default()
    }

    fn write_to(&self, out: &mut dyn Write) -> io::Result<()> {
        macro_rules! exec {
            (fg, $color:ident) => {
                exec!(SetForegroundColor(Color::$color))
            };

            (at, $attr:ident) => {
                exec!(SetAttribute(Attribute::$attr))
            };

            ($action:expr) => {{
                out.execute($action)?;
            }};
        }

        let mut splits = self.0.split('§');
        if let Some(n) = splits.next() {
            exec!(Print(n));
        }

        let mut empty = true;
        for split in splits {
            empty = false;
            if let Some(c) = split.chars().next() {
                match c {
                    // Colors
                    '0' => exec!(fg, Black),
                    '1' => exec!(fg, DarkBlue),
                    '2' => exec!(fg, DarkGreen),
                    '3' => exec!(fg, DarkCyan),
                    '4' => exec!(fg, DarkRed),
                    '5' => exec!(fg, DarkMagenta),
                    '6' => exec!(fg, DarkYellow),
                    '7' => exec!(fg, Grey),
                    '8' => exec!(fg, DarkGrey),
                    '9' => exec!(fg, Blue),
                    'a' => exec!(fg, Green),
                    'b' => exec!(fg, Cyan),
                    'c' => exec!(fg, Red),
                    'd' => exec!(fg, Magenta),
                    'e' => exec!(fg, Yellow),
                    'f' => exec!(fg, White),

                    // Formatting
                    // Obfuscated. This is the closest thing, althogh not many terminals support it.
                    'k' => exec!(at, RapidBlink),
                    'l' => exec!(at, Bold),
                    'm' => exec!(at, CrossedOut),
                    'n' => exec!(at, Underlined),
                    'o' => exec!(at, Italic),
                    'r' => exec!(ResetColor),
                    _ => {},
                }
                exec!(Print(&split[1..]));
            }
        }

        // no need to reset color if there were no escape codes.
        if !empty {
            exec!(ResetColor);
        }

        Ok(())
    }
}

pub trait TableEntry {
    fn print(&self, out: &mut dyn Write, table: &Table) -> io::Result<()>;
}

pub struct SmallTableEntry(String, Box<dyn TableContent>);

impl TableEntry for SmallTableEntry {
    fn print(&self, out: &mut dyn Write, table: &Table) -> io::Result<()> {
        write!(
            out,
            "{: <width$} | ",
            self.0,
            width = table.small_entry_width
        )?;
        self.1.write_to(out)?;
        out.write_all(b"\n")?;

        Ok(())
    }
}

pub struct BigTableEntry {
    name: String,
    val: Box<dyn TableContent>,
}

impl TableEntry for BigTableEntry {
    fn print(&self, out: &mut dyn Write, _table: &Table) -> io::Result<()> {
        let width = max(self.val.width(), self.name.width() + 4);

        writeln!(out, "{:=^width$}", self.name)?;
        self.val.write_to(out)?;
        writeln!(out, "\n{:=<width$}", "")?;

        Ok(())
    }
}

impl BigTableEntry {
    pub fn new(name: String, val: impl TableContent + 'static) -> Self {
        Self {
            name,
            val: Box::new(val),
        }
    }
}

pub struct BlankTableEntry;

impl TableEntry for BlankTableEntry {
    fn print(&self, out: &mut dyn Write, _: &Table) -> io::Result<()> {
        out.write(b"\n").map(|_| ())
    }
}
