#[macro_use]
extern crate clap;

use std::error::Error;
use std::io::{Cursor, Write};

use asciify::AsciiBuilder;
use async_minecraft_ping::ConnectionConfig;
use clap::App;
use image::ImageFormat;
use itertools::Itertools;
use termcolor::{Buffer, BufferWriter, ColorChoice, WriteColor};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let yaml = load_yaml!("args.yml");
    let matches = App::from_yaml(yaml).get_matches();

    //region Network
    let config = ConnectionConfig::build(matches.value_of("ip").unwrap().to_owned())
        .with_port(
            matches
                .value_of("port")
                .unwrap()
                .parse()
                .ok()
                .and_then(|p| if p > 0 && p < u16::MAX { Some(p) } else { None })
                .expect("invalid port"),
        )
        .with_protocol_version(
            matches
                .value_of("protocol-version")
                .unwrap()
                .parse()
                .expect("invalid protocol version"),
        );
    let mut connection = config.connect().await?;
    let response = connection.status().await?;
    //endregion

    //region Image
    let image_size: u32 = matches
        .value_of("size")
        .unwrap()
        .parse()
        .expect("image size must be number");
    let mut image = None;
    if let (Some(favicon), true) = (response.favicon, matches.is_present("image")) {
        //The image parsing and asciifying is done while the table is printing
        image = Some(tokio::spawn(get_image(
            favicon,
            image_size,
            matches.is_present("color"),
        )));
    }
    //endregion

    //region printing
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

    let player_sample = response
        .players
        .sample
        .unwrap_or_default()
        .iter()
        .map(|p| p.name.as_str())
        .intersperse("\n")
        .collect::<String>();

    print_table!(
        me "Description" => remove_formatting(&response.description.text),
        me "Player Sample" => remove_formatting(&player_sample),
        se "Server Version" => remove_formatting(&response.version.name),
        s "Online Players" => response.players.online,
        s "Max Players" => response.players.max,
        s "Server Protocol" => response.version.protocol
    );

    if let Some(img) = image {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(&img.await?)?;
    }
    //endregion
    Ok(())
}

fn remove_formatting(s: &str) -> String {
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

/// returns the asciifyed image as UTF-8 bytes
async fn get_image(favicon: String, image_size: u32, colored: bool) -> Vec<u8> {
    let img = image_base64::from_base64(favicon);
    let image =
        image::load(Cursor::new(img), ImageFormat::Png).expect("favicon has invalid format");

    let builder = AsciiBuilder::new_from_image(image).set_resize((image_size * 2, image_size));

    let mut buf = if colored {
        let mut buf = BufferWriter::stdout(ColorChoice::Always).buffer();
        builder.to_stream_colored(&mut buf);
        buf
    } else {
        let mut buf = Buffer::no_color();
        builder.to_stream(&mut buf);
        buf
    };
    buf.reset().unwrap();
    buf.as_slice()
    .to_vec()
}
