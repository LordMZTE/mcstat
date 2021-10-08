#[macro_use]
extern crate smart_default;

use crate::output::Table;
use crossterm::{
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    ExecutableCommand,
};
use image::{DynamicImage, ImageFormat};
use itertools::Itertools;
use miette::{bail, miette, IntoDiagnostic, WrapErr};
use std::io::{self, Cursor, Write};

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

/// Print mincraft-formatted text to `out` using crossterm
pub fn print_mc_formatted(s: &str, mut out: impl Write) -> io::Result<()> {
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

    let mut splits = s.split('§');
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
                _ => {}
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

pub fn mc_formatted_to_ansi(s: &str) -> io::Result<String> {
    let mut bytes = Vec::new();
    let mut c = Cursor::new(&mut bytes);
    print_mc_formatted(s, &mut c)?;

    // this shouldn't be able to fail, as we started of with a valid utf8 string.
    #[cfg(debug_assertions)]
    let out = String::from_utf8(bytes).unwrap();

    #[cfg(not(debug_assertions))]
    let out = unsafe { String::from_utf8_unchecked(bytes) };

    Ok(out)
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

/// parses a base64 formatted image
pub fn parse_base64_image(data: String) -> miette::Result<DynamicImage> {
    let (header, data) = data
        .split_once(',')
        .ok_or_else(|| miette!("Couldn't parse base64 image due to missing format header."))?;
    let (data_type, image_format) = header
        .split_once('/')
        .ok_or_else(|| miette!("Failed to parse base64 image, header has invalid format."))?;
    let image_format = image_format
        .split(';')
        .next()
        .ok_or_else(|| miette!("Failed to parse base64 image, header has invalid format."))?;

    if data_type != "data:image" {
        bail!("base64 image is not an image! Has type {}", data_type);
    }

    let format = ImageFormat::from_extension(image_format).ok_or_else(|| {
        miette!(
            "Failed to parse base64 image due to unknown image type: {}",
            image_format
        )
    })?;
    let data = base64::decode(data)
        .into_diagnostic()
        .wrap_err("Failed to decode base64 image data")?;
    image::load(Cursor::new(data), format)
        .into_diagnostic()
        .wrap_err("Failed to load base64 image")
}
