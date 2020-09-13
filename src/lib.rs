
use asciify::AsciiBuilder;

#[macro_export]
macro_rules! print_table {
    (s $l:expr => $k:expr) => {
        println!("{: <20} | {}", $l, $k);
    };

    (m $l:expr => $k:expr) => {
        println!("====={:=<20}\n{}", $l, $k);
    };

    (se $l:expr => $k:expr) => {
        if !&$k.is_empty() {
            println!("{: <20} | {}", $l, $k);
        }
    };

    (me $l:expr => $k:expr) => {
        if !&$k.is_empty() {
            println!("====={:=<20}\n{}\n=========================\n", $l, $k);
        }
    };

    ($($t:tt $l:expr => $k:expr),+) => {
        $(print_table!($t $l => $k);)*
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
