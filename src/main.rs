#[macro_use]
extern crate clap;
#[macro_use]
extern crate mcstat;
#[macro_use]
extern crate anyhow;

use std::io::Cursor;
use time::Duration;
use tokio::time;

use anyhow::{Context, Result};
use asciify::AsciiBuilder;
use async_minecraft_ping::{ConnectionConfig, ModInfo, ServerDescription, StatusResponse};
use clap::App;
use image::ImageFormat;
use itertools::Itertools;
use mcstat::{remove_formatting, AsciiConfig};
use termcolor::{Buffer, BufferWriter, ColorChoice, WriteColor};

/// this message is used if getting a value from the arguments fails
const ARGUMENT_FAIL_MESSAGE: &str = "failed to get value from args";
#[tokio::main]
async fn main() -> Result<()> {
    let yaml = load_yaml!("args.yml");
    let matches = App::from_yaml(yaml).get_matches();

    // region Network
    let config = ConnectionConfig::build(
        matches
            .value_of("ip")
            .context(ARGUMENT_FAIL_MESSAGE)?
            .to_owned(),
    )
    .with_port(
        matches
            .value_of("port")
            .context(ARGUMENT_FAIL_MESSAGE)?
            .parse()
            .context("invalid port")
            .and_then(|p| {
                // the port must be above 0
                if p > 0 {
                    Ok(p)
                } else {
                    // this error will be overriden anyways
                    Err(anyhow!(""))
                }
            })
            .context("invalid port")?,
    )
    .with_protocol_version(
        matches
            .value_of("protocol-version")
            .context(ARGUMENT_FAIL_MESSAGE)?
            .parse()
            .context("invalid protocol version")?,
    );

    // create timeout for server connection
    let mut timeout = time::delay_for(Duration::from_millis(
        matches
            .value_of("timeout")
            .context(ARGUMENT_FAIL_MESSAGE)?
            .parse()
            .context("timeout is invalid value")?,
    ));

    let raw_response = tokio::select! {
        _ = &mut timeout => Err(anyhow!("Connection to server timed out")),
        r = async {
            let mut con = config.connect().await?;
            con.status_raw().await
        } => r,
    }?;

    if matches.is_present("raw") {
        println!("{}", raw_response);
        return Ok(());
    }

    let response = serde_json::from_str::<StatusResponse>(&raw_response)?;
    // endregion

    // region Image
    let image_size: u32 = matches
        .value_of("size")
        .context("failed to get value from args")?
        .parse()
        .context("image size must be number")?;
    let mut image = None;

    if let (Some(favicon), true) = (response.favicon, matches.is_present("image")) {
        // The image parsing and asciifying is done while the table is printing
        image = Some(tokio::spawn(asciify_base64_image(
            favicon,
            AsciiConfig {
                size: Some(image_size),
                colored: matches.is_present("color"),
                deep: matches.is_present("deep"),
                invert: matches.is_present("invert"),
            },
        )));
    }
    // endregion

    // region printing
    // if the server has mods, and the user hasn't used the -m argument, notify
    // that.
    if let (false, Some(_)) = (matches.is_present("mods"), &response.modinfo) {
        println!("This server has mods. To show them use the -m argument\n")
    }

    let player_sample = response
        .players
        .sample
        .unwrap_or_default()
        .iter()
        .map(|p| p.name.as_str())
        .intersperse("\n")
        .collect::<String>();

    print_table! {
        40;
        bo "Description" => none_if_empty!(remove_formatting(&response.description.get_text())),
        bo "Extra Description" => {
            if let ServerDescription::Big(big_desc) = response.description {
                let desc = big_desc.extra;
                if desc.is_empty() {
                    None
                } else {
                    Some(desc.into_iter().map(|p| p.text).collect::<String>())
               }
            } else {
                None
            }
        },
        bo "Player Sample" => none_if_empty!(remove_formatting(&player_sample)),
        lo "Server Version" => none_if_empty!(remove_formatting(&response.version.name)),
        l "Online Players" => response.players.online,
        l "Max Players" => response.players.max,
        bo "Mods" => if let (Some(mods), true) = (response.modinfo, matches.is_present("mods")) {
                Some(get_modlist(mods, matches.is_present("modversions")))
            } else {
                None
            },
        l "Server Protocol" => response.version.protocol,
    };

    if let Some(img) = image {
        println!("\n{}", img.await??);
    }
    // endregion
    Ok(())
}

/// returns the asciifyed image from base64
/// returns Err if the base64 image is invalid
async fn asciify_base64_image(favicon: String, config: AsciiConfig) -> Result<String> {
    let img = image_base64::from_base64(favicon);
    // TODO for some reason, image_base64 returns the format as string (and using
    // regex!) which is useless and also inefficient, so Png is temporarily
    // hardcoded. we should probably stop using this library
    let image =
        image::load(Cursor::new(img), ImageFormat::Png).context("image has invalid format")?;

    let builder = config.apply(AsciiBuilder::new_from_image(image));

    let mut buf = if config.colored {
        // this does not write to stdout but just gets the correct color
        // information for stdout
        let mut buf = BufferWriter::stdout(ColorChoice::Always).buffer();
        builder.to_stream_colored(&mut buf);
        buf
    } else {
        let mut buf = Buffer::no_color();
        builder.to_stream(&mut buf);
        buf
    };
    // reset color
    buf.reset()?;

    let bytes = buf.as_slice().to_vec();

    // only check utf8 format in debug mode
    #[cfg(debug_assertions)]
    let out = String::from_utf8(bytes).expect("asciifyed image is invalid utf8");
    // bytes should always be valid utf8
    #[cfg(not(debug_assertions))]
    let out = unsafe { String::from_utf8_unchecked(bytes) };

    Ok(out)
}

/// formats a ModInfo to a readable list of mods
///
/// if `version_info`, the version of the mods will also be displayed
fn get_modlist(list: ModInfo, version_info: bool) -> String {
    let infos = match list {
        ModInfo::Forge { mod_list: l } => l,
    };

    // the width at which | characters should be placed this is the length of the
    // longest modid
    let max_width = if version_info {
        infos
            .iter()
            .map(|m| m.modid.len())
            .max()
            .unwrap_or_default()
    } else {
        // this will not be used in case version_info is off so we just use 0
        0
    };

    infos
        // we use into_iter instead of iter because a String cannot be collected from &String
        // and since we don't need infos again
        .into_iter()
        .map(|m| {
            if version_info {
                format!("{: <width$} | {}", m.modid, m.version, width = max_width)
            } else {
                m.modid
            }
        })
        .intersperse("\n".to_owned())
        .collect()
}
