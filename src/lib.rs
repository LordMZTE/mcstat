use asciify::AsciiBuilder;

/// prints a table with the entries supplied
/// the identifier at the start of each entry sets the type
/// 
/// l = list entry
/// b = block
/// lo = list entry option
/// bo = block option
/// 
/// options are checked if they are `Some` and won't be printed if they aren't
#[macro_export]
macro_rules! print_table {
    //list entry
    (l $l:expr => $k:expr) => {
        println!("{: <20} | {}", $l, $k);
    };

    //block
    (b $l:expr => $k:expr) => {
        println!("{:=^25}\n{}\n=========================", $l, $k);
    };

    //list entry option
    (lo $l:expr => $k:expr) => {
        if let Some(txt) = $k {
            println!("{: <20} | {}", $l, txt);
        }
    };

    //block option
    (bo $l:expr => $k:expr) => {
        if let Some(txt) = $k {
            println!("{:=^25}\n{}\n=========================", $l, txt);
        }
    };

    ($($t:tt $l:expr => $k:expr),+ $(,)?) => {
        $(print_table!($t $l => $k);)*
    };
}

/// returns an `Option` of the expression passed in
/// `None` if the `is_empty` on the expression returns true, `Some(x)` otherwise
/// this is a macro and not a function because `is_empty` is not defined in any trait
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
