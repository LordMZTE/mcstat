use std::io::Cursor;

use anyhow::{anyhow, Context, Result};
use asciify::AsciiBuilder;
use async_minecraft_ping::{ConnectionConfig, ServerDescription, StatusResponse};
use image::ImageFormat;
use itertools::Itertools;
use termcolor::{Buffer, BufferWriter, ColorChoice, WriteColor};
use time::{Duration, Instant};
use tokio::time;

use mcstat::{get_table, none_if_empty, output::Table, remove_formatting, AsciiConfig};

mod cli;

/// this message is used if getting a value from the arguments fails
const ARGUMENT_FAIL_MESSAGE: &str = "failed to get value from args";

#[tokio::main]
async fn main() -> Result<()> {
    let matches = cli::get_app().get_matches();

    // region Network
    let mut ip = matches
        .value_of("ip")
        .context(ARGUMENT_FAIL_MESSAGE)?
        .splitn(2, ':');

    let config = ConnectionConfig::build(ip.next().context("invalid ip")?.to_owned())
        .with_port(
            ip.next()
                .map_or(Err(()), |p| p.parse::<u16>().map_err(|_| ()))
                .and_then(|p| if p > 0 { Ok(p) } else { Err(()) })
                .unwrap_or(25565),
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

    let (raw_response, ping) = tokio::select! {
        _ = &mut timeout => Err(anyhow!("Connection to server timed out")),
        r = async {
            let start_time = Instant::now();
            let mut con = config.connect().await?;
            // we end the timer here, because at this point, we've sent ONE request to the server,
            // and we don't want to send 2, since then we get double the ping.
            // the connect function may have some processing which may take some time, but it
            // shouldn't make an impact at this code runs at rust speed.
            let end_time = Instant::now();

            let status = con.status_raw().await?;
            Ok((status, end_time - start_time))
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

    if let (Some(favicon), true) = (&response.favicon, matches.is_present("image")) {
        // The image parsing and asciifying is done while the table is printing
        image = Some(tokio::spawn(asciify_base64_image(
            favicon.clone(),
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
    if let (false, Some(_)) = (matches.is_present("mods"), response.forge_mod_info()) {
        println!("This server has mods. To show them use the -m argument\n")
    }

    format_table(
        &response,
        ping.as_millis(),
        matches.is_present("mods"),
        matches.is_present("modversions"),
        matches.is_present("channels"),
    )
    .stdout()?;

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

fn format_table(
    response: &StatusResponse,
    ping: u128,
    mods: bool,
    modversions: bool,
    channels: bool,
) -> Table {
    let player_sample = response
        .players
        .sample
        .as_ref()
        .unwrap_or(&vec![])
        .iter()
        .map(|p| p.name.as_str())
        .intersperse("\n")
        .collect::<String>();

    let mut table = Table::new();

    if let Some((w, _)) = term_size::dimensions() {
        table.max_block_width = w;
    }

    if let Some(s) = none_if_empty!(remove_formatting(&response.description.get_text())) {
        table.big_entry("Description", s);
    }

    if let ServerDescription::Big(big_desc) = &response.description {
        let desc = &big_desc.extra;
        let txt = desc.into_iter().map(|p| p.text.clone()).collect::<String>();
        if let Some(s) = none_if_empty!(txt) {
            table.big_entry("Extra Description", s);
        }
    }

    if let Some(s) = none_if_empty!(remove_formatting(&player_sample)) {
        table.big_entry("Player Sample", s);
    }

    table.blank();

    if let Some(s) = none_if_empty!(remove_formatting(&response.version.name)) {
        table.small_entry("Server Version", s);
    }

    table.small_entry("Online Players", &response.players.online);
    table.small_entry("Max Players", &response.players.max);
    table.small_entry("Ping", ping);
    table.small_entry("Protocol Version", &response.version.protocol);

    table.blank();

    if let (Some(mod_list), true) = (response.forge_mod_info(), mods) {
        let txt = get_table(
            mod_list
                .iter()
                .sorted_by(|a, b| a.modid.cmp(&b.modid))
                .map(|m| (&*m.modid, &*m.version)),
            modversions,
        );

        if let Some(s) = none_if_empty!(txt) {
            table.big_entry("Mods", s);
        }
    }

    if let (true, Some(fd)) = (channels, &response.forge_data) {
        let txt = get_table(
            fd.channels
                .iter()
                .sorted_by(|a, b| a.res.cmp(&b.res))
                .map(|c| (&*c.res, &*c.version)),
            true,
        );

        if let Some(s) = none_if_empty!(txt) {
            table.big_entry("Forge Channels", s);
        }
    }

    table
}
