use std::{
    cmp::{max, min},
    io::{self, Write},
};

#[derive(SmartDefault)]
pub struct Table {
    pub entries: Vec<Box<dyn TableEntry>>,
    pub small_entry_width: usize,
    #[default(usize::MAX)]
    pub max_block_width: usize,
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

    pub fn small_entry(&mut self, name: impl ToString, val: impl ToString) {
        let name = name.to_string();
        self.set_small_width(name.len());

        self.entries
            .push(Box::new(SmallTableEntry(name, val.to_string())));
    }

    pub fn big_entry(&mut self, name: impl ToString, val: impl ToString) {
        self.entries.push(Box::new(BigTableEntry::new(
            name.to_string(),
            val.to_string(),
            self.max_block_width,
        )));
    }

    pub fn opt_small_entry(&mut self, name: impl ToString, val: Option<impl ToString>) {
        let name = name.to_string();
        self.set_small_width(name.len());

        self.entries.push(Box::new(OptSmallTableEntry(
            val.map(|t| SmallTableEntry(name, t.to_string())),
        )));
    }

    pub fn opt_big_entry(&mut self, name: impl ToString, val: Option<impl ToString>) {
        self.entries.push(Box::new(OptBigTableEntry(val.map(|t| {
            BigTableEntry::new(name.to_string(), t.to_string(), self.max_block_width)
        }))));
    }

    fn set_small_width(&mut self, width: usize) {
        if width > self.small_entry_width {
            self.small_entry_width = width;
        }
    }
}

pub trait TableEntry {
    fn print(&self, out: &mut dyn Write, table: &Table) -> io::Result<()>;
}

pub struct SmallTableEntry(String, String);

impl TableEntry for SmallTableEntry {
    fn print(&self, out: &mut dyn Write, table: &Table) -> io::Result<()> {
        writeln!(
            out,
            "{: <width$} | {}",
            self.0,
            self.1,
            width = table.small_entry_width
        )
    }
}

pub struct BigTableEntry {
    name: String,
    val: String,
    width: usize,
}

impl TableEntry for BigTableEntry {
    fn print(&self, out: &mut dyn Write, _table: &Table) -> io::Result<()> {
        writeln!(
            out,
            "{:=^width$}\n{}\n{:=<width$}",
            self.name,
            self.val,
            "",
            width = self.width,
        )
    }
}

impl BigTableEntry {
    pub fn new(name: String, val: String, maxwidth: usize) -> Self {
        let val_width = min(
            max(
                val.lines().map(|s| s.len() + 4).max().unwrap_or_default(),
                name.len() + 4,
            ),
            maxwidth,
        );

        Self {
            width: max(name.len(), val_width),
            name,
            val,
        }
    }
}

pub struct OptSmallTableEntry(Option<SmallTableEntry>);

impl TableEntry for OptSmallTableEntry {
    fn print(&self, out: &mut dyn Write, table: &Table) -> io::Result<()> {
        if let Some(entry) = &self.0 {
            entry.print(out, table)
        } else {
            Ok(())
        }
    }
}

pub struct OptBigTableEntry(Option<BigTableEntry>);

impl TableEntry for OptBigTableEntry {
    fn print(&self, out: &mut dyn Write, table: &Table) -> io::Result<()> {
        if let Some(entry) = &self.0 {
            entry.print(out, table)
        } else {
            Ok(())
        }
    }
}

pub struct BlankTableEntry;

impl TableEntry for BlankTableEntry {
    fn print(&self, out: &mut dyn Write, _: &Table) -> io::Result<()> {
        out.write(b"\n").map(|_| ())
    }
}
