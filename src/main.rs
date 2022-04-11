use async_minecraft_ping::{ConnectionConfig, ServerDescription, StatusResponse};

use clap::Parser;
use itertools::Itertools;
use miette::{IntoDiagnostic, WrapErr};
use time::{Duration, Instant};
use tokio::time;

use mcstat::{
    get_table,
    none_if_empty,
    output::{McFormatContent, Table},
    parse_base64_image,
    resolve_address,
    EitherStatusResponse,
};
use tracing::{info, Level};

/// Queries information about a minecraft server
#[derive(Debug, Parser)]
#[clap(name = "mcstat")]
struct Opt {
    /// The Address to ping. By default, a SRV lookup will be made to resolve
    /// this, unless the port is specified
    ip: String,

    /// the protocol version to use
    #[clap(long = "protocol", default_value = "751")]
    protocol_version: usize,

    /// the time before the server ping times out in milliseconds
    #[clap(long, short, default_value = "5000")]
    timeout: u64,

    /// print raw json response
    #[clap(long, short)]
    raw: bool,

    /// print mod list
    #[clap(long, short)]
    mods: bool,

    /// print mod versions
    #[clap(long, short = 'V', requires = "mods")]
    modversions: bool,

    /// displays forge mod channels if the server sends them
    #[clap(long)]
    channels: bool,

    /// print the server's favicon to stdout
    #[clap(long, short)]
    image: bool,

    /// size of the favicon ascii art
    #[clap(short, requires = "image")]
    size: Option<u32>,

    /// use verbose logging
    #[clap(long, short, parse(from_occurrences))]
    verbose: u32,
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
async fn main() -> miette::Result<()> {
    let opt = Opt::parse();

    let log_level = match opt.verbose {
        0 => Level::ERROR,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .compact()
        .with_max_level(log_level)
        .init();

    let (addr, port) = resolve_address(&opt.ip)
        .await
        .wrap_err("Error resolving address")?;

    info!("Using address '{}:{}'", &addr, &port);

    let config = ConnectionConfig::build(addr)
        .with_port(port)
        .with_protocol_version(opt.protocol_version);

    // create timeout for server connection
    let (raw_response, ping) = time::timeout(Duration::from_millis(opt.timeout), async {
        info!("Connecting to server");
        let start_time = Instant::now();
        let mut con = config.connect().await.into_diagnostic()?;
        // we end the timer here, because at this point, we've sent ONE request to the
        // server, and we don't want to send 2, since then we get double the
        // ping. the connect function may have some processing which may take
        // some time, but it shouldn't make an impact since this code runs at rust
        // speed.
        let end_time = Instant::now();

        info!("Requesting status");
        let status = con.status_raw().await.into_diagnostic()?;
        Result::<_, miette::Error>::Ok((status, end_time - start_time))
    })
    .await
    .into_diagnostic()
    .context("Connection to server timed out.")??;

    if opt.raw {
        println!("{}", raw_response);
        return Ok(());
    }

    info!("Parsing status");
    let response = serde_json::from_str::<EitherStatusResponse>(&raw_response).into_diagnostic()?;

    let response = match response {
        EitherStatusResponse::Text { text } => {
            println!("The server says:\n{}", text);

            return Ok(());
        },
        EitherStatusResponse::Normal(r) => r,
    };

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
    .stdout()
    .into_diagnostic()?;

    if let (Some(img), true) = (response.favicon, opt.image) {
        let decoded = parse_base64_image(img)?;
        viuer::print(&decoded, &opt.get_viuer_conf()).into_diagnostic()?;
    }
    Ok(())
}

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

    if let Some(s) = none_if_empty!(McFormatContent(response.description.get_text().clone())) {
        table.big_entry("Description", s);
    }

    if let ServerDescription::Big(big_desc) = &response.description {
        let desc = &big_desc.extra;
        let txt = desc.iter().map(|p| p.text.clone()).collect::<String>();
        if let Some(s) = none_if_empty!(txt) {
            table.big_entry("Extra Description", McFormatContent(s));
        }
    }

    if let Some(s) = none_if_empty!(McFormatContent(player_sample)) {
        table.big_entry("Player Sample", s);
    }

    table.blank();

    if let Some(s) = none_if_empty!(response.version.name.clone()) {
        table.small_entry("Server Version", s);
    }

    table.small_entry("Online Players", response.players.online.to_string());
    table.small_entry("Max Players", response.players.max.to_string());
    table.small_entry("Ping", ping.to_string());
    table.small_entry("Protocol Version", response.version.protocol.to_string());

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
