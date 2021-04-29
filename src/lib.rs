#[macro_use]
extern crate smart_default;

use crate::output::Table;
use anyhow::{anyhow, bail, Context};
use image::{DynamicImage, ImageFormat};
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

/// parses a base64 formatted image
pub fn parse_base64_image(data: String) -> anyhow::Result<DynamicImage> {
    let (header, data) = data
        .split_once(",")
        .context("Couldn't parse base64 image due to missing format header.")?;
    let (data_type, image_format) = header
        .split_once("/")
        .context("Failed to parse base64 image, header has invalid format.")?;
    let image_format = image_format
        .split(";")
        .next()
        .context("Failed to parse base64 image, header has invalid format.")?;

    if data_type != "data:image" {
        bail!("base64 image is not an image! Has type {}", data_type);
    }

    let format = ImageFormat::from_extension(image_format).context(format!(
        "Failed to parse base64 image due to unknown image type: {}",
        image_format
    ))?;
    let data =
        base64::decode(data).map_err(|e| anyhow!("Failed to decode base64 image data: {}", e))?;
    image::load(Cursor::new(data), format)
        .map_err(|e| anyhow!("Failed to load base64 image: {}", e))
}
