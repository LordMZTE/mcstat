use anyhow::{anyhow, Context, Result};
use async_minecraft_ping::{ConnectionConfig, ServerDescription, StatusResponse};

use itertools::Itertools;
use structopt::StructOpt;
use time::{Duration, Instant};
use tokio::time;

use mcstat::{get_table, none_if_empty, output::Table, parse_base64_image, remove_formatting};

#[derive(Debug, StructOpt)]
#[structopt(
    name = "mcstat",
    about = "queries information about a minecraft server"
)]
struct Opt {
    #[structopt(
        index = 1,
        help = "the ip of the server to ping. you may also specify the port, if it is not \
                specified or invalid it will default to 25565"
    )]
    ip: String,

    #[structopt(
        long = "protocol",
        help = "the protocol version to use",
        default_value = "751"
    )]
    protocol_version: usize,

    #[structopt(
        long,
        short,
        help = "the time before the server ping times out in milliseconds",
        default_value = "5000"
    )]
    timeout: u64,

    #[structopt(long, short, help = "print raw json response")]
    raw: bool,

    #[structopt(long, short, help = "print mod list")]
    mods: bool,

    #[structopt(
        long,
        short = "v",
        requires = "mods",
        help = "also prints mod versions"
    )]
    modversions: bool,

    #[structopt(long, help = "displays forge mod channels if the server sends them")]
    channels: bool,

    #[structopt(long, short, help = "print the server's favicon to stdout")]
    image: bool,

    #[structopt(short, requires = "image", help = "size of the favicon ascii art")]
    size: Option<u32>,
}

impl Opt {
    fn get_viuer_conf(&self) -> viuer::Config {
        let size = self.size.unwrap_or(16);
        viuer::Config {
            transparent: true,
            absolute_offset: false,
            width: Some(size * 2),
            height: Some(size),
            ..viuer::Config::default()
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args_safe()?;

    let mut ip = opt.ip.splitn(2, ':');
    let config = ConnectionConfig::build(ip.next().context("invalid ip")?.to_owned())
        .with_port(
            ip.next()
                .map_or(Err(()), |p| p.parse::<u16>().map_err(|_| ()))
                .and_then(|p| if p > 0 { Ok(p) } else { Err(()) })
                .unwrap_or(25565),
        )
        .with_protocol_version(opt.protocol_version);

    // create timeout for server connection
    let (raw_response, ping) = time::timeout(Duration::from_millis(opt.timeout), async {
        let start_time = Instant::now();
        let mut con = config.connect().await?;
        // we end the timer here, because at this point, we've sent ONE request to the
        // server, and we don't want to send 2, since then we get double the
        // ping. the connect function may have some processing which may take
        // some time, but it shouldn't make an impact at this code runs at rust
        // speed.
        let end_time = Instant::now();

        let status = con.status_raw().await?;
        Result::<_, anyhow::Error>::Ok((status, end_time - start_time))
    })
    .await
    .context("Connection to server timed out.")??;

    if opt.raw {
        println!("{}", raw_response);
        return Ok(());
    }

    let response = serde_json::from_str::<StatusResponse>(&raw_response)?;
    // endregion

    // region printing
    // if the server has mods, and the user hasn't used the -m argument, notify
    // that.
    if let (false, Some(_)) = (opt.mods, response.forge_mod_info()) {
        println!("This server has mods. To show them use the -m argument\n")
    }

    format_table(
        &response,
        ping.as_millis(),
        opt.mods,
        opt.modversions,
        opt.channels,
    )
    .stdout()?;

    if let (Some(img), true) = (response.favicon, opt.image) {
        let decoded = parse_base64_image(img)?;
        viuer::print(&decoded, &opt.get_viuer_conf())
            .map_err(|e| anyhow!("Failed to print favicon: {}", e))?;
    }
    // endregion
    Ok(())
}

// returns the asciifyed image from base64
// returns Err if the base64 image is invalid
// async fn asciify_base64_image(favicon: String, config: AsciiConfig) ->
// Result<String> { let image = parse_base64_image(favicon)?;
//
// let builder = config.apply(AsciiBuilder::new_from_image(image));
//
// let mut buf = if config.colored {
// this does not write to stdout but just gets the correct color
// information for stdout
// let mut buf = BufferWriter::stdout(ColorChoice::Always).buffer();
// builder.to_stream_colored(&mut buf);
// buf
// } else {
// let mut buf = Buffer::no_color();
// builder.to_stream(&mut buf);
// buf
// };
// reset color
// buf.reset()?;
//
// let bytes = buf.as_slice().to_vec();
//
// only check utf8 format in debug mode
// #[cfg(debug_assertions)]
// let out = String::from_utf8(bytes).expect("asciifyed image is invalid utf8");
// bytes should always be valid utf8
// #[cfg(not(debug_assertions))]
// let out = unsafe { String::from_utf8_unchecked(bytes) };
//
// Ok(out)
// }

fn format_table(
    response: &StatusResponse,
    ping: u128,
    mods: bool,
    modversions: bool,
    channels: bool,
) -> Table {
    // this syntax is used due to a nightly function which will be added to rust
    // also called intersperse
    let player_sample = Itertools::intersperse(
        response
            .players
            .sample
            .as_ref()
            .unwrap_or(&vec![])
            .iter()
            .map(|p| p.name.as_str()),
        "\n",
    )
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
