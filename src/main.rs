#[macro_use]
extern crate clap;
#[macro_use]
extern crate mcstat;

use std::io::{Cursor, Write};

use mcstat::{AsciiConfig, remove_formatting};
use anyhow::{Context, Result};
use asciify::AsciiBuilder;
use async_minecraft_ping::ConnectionConfig;
use clap::App;
use image::ImageFormat;
use itertools::Itertools;
use termcolor::{Buffer, BufferWriter, ColorChoice, WriteColor};

#[tokio::main]
async fn main() -> Result<()> {
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
                .context("invalid port")?,
        )
        .with_protocol_version(
            matches
                .value_of("protocol-version")
                .unwrap()
                .parse()
                .context("invalid protocol version")?,
        );
    let mut connection = config.connect().await?;
    let response = connection.status().await?;
    //endregion

    //region Image
    let image_size: u32 = matches
        .value_of("size")
        .unwrap()
        .parse()
        .with_context(|| "image size must be number")?;
    let mut image = None;
    if let (Some(favicon), true) = (response.favicon, matches.is_present("image")) {
        //The image parsing and asciifying is done while the table is printing
        image = Some(tokio::spawn(get_image(
            favicon,
            AsciiConfig {
                size: Some(image_size),
                colored: matches.is_present("color"),
                deep: matches.is_present("deep"),
                invert: matches.is_present("invert"),
            },
        )));
    }
    //endregion

    //region printing
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
        handle.write_all(&[b'\n'])?;
        handle.write_all(&img.await??)?;
    }
    //endregion
    Ok(())
}

/// returns the asciifyed image as UTF-8 bytes
async fn get_image(favicon: String, config: AsciiConfig) -> Result<Vec<u8>> {
    let img = image_base64::from_base64(favicon);
    let image =
        image::load(Cursor::new(img), ImageFormat::Png).context("favicon has invalid format")?;

    let builder = config.apply(AsciiBuilder::new_from_image(image));

    let mut buf = if config.colored {
        //this does not write to stdout but just gets the correct color information for stdout
        let mut buf = BufferWriter::stdout(ColorChoice::Always).buffer();
        builder.to_stream_colored(&mut buf);
        buf
    } else {
        let mut buf = Buffer::no_color();
        builder.to_stream(&mut buf);
        buf
    };
    buf.reset()?;
    Ok(buf.as_slice().to_vec())
}
