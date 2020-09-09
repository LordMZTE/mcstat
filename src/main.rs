use std::error::Error;
use std::io::Cursor;

use asciify::AsciiBuilder;
use async_minecraft_ping::ConnectionConfig;
use clap::{App, Arg};
use image::ImageFormat;
use itertools::Itertools;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let matches = App::new("mcstat")
        .about("queries information about a minecraft server")
        .arg(
            Arg::with_name("ip")
                .value_name("IP_ADDRESS")
                .help("the ip of the server to ping")
                .takes_value(true)
                .index(1)
                .required(true)
        )
        .arg(
            Arg::with_name("port")
                .value_name("PORT")
                .help("the port of the server")
                .long("port")
                .short("p")
                .default_value("25565")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("protocol-version")
                .long("protocol")
                .value_name("PROTOCOL_VERSION")
                .help("the protocol version to use")
                .default_value("751")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("image")
                .short("i")
                .help("if an the server\'s favicon should be printed as ASCII art")
        )
        .arg(
            Arg::with_name("color")
                .short("c")
                .help("if the favicon image should be printed with ANSII color formatting or monochrome")
        )
        .get_matches();

    let mut config = ConnectionConfig::build(matches.value_of("ip").unwrap().to_owned());
    config = config.with_port(matches.value_of("port").unwrap().parse().ok().and_then(|p| if p > 0 && p < u16::MAX { Some(p) } else { None }).expect("invalid port"));
    config = config.with_protocol_version(matches.value_of("protocol-version").unwrap().parse().expect("invalid protocol version"));
    let mut connection = config.connect().await?;
    let response = connection.status().await?;


    macro_rules! print_table {
        ($($l:expr => $k:expr),+) => {
            $(println!("{: <15} | {}", $l, $k);)*
        };
    }

    //region printing
    print_table!(
        "Online Players" => response.players.online,
        "Max Players" => response.players.max,
        "Server Version" => response.version.name,
        "Server Protocol" => response.version.protocol,
        "Player Sample" => response.players.sample.unwrap_or_default().iter().map(|p| p.name.as_str()).intersperse(", ").collect::<String>(),
        "Description" => response.description.text
    );

    //Image
    if let (Some(favicon), true) = (response.favicon, matches.is_present("image")) {
        let img = image_base64::from_base64(favicon);
        let image = image::load(Cursor::new(img), ImageFormat::Png).expect("favicon has invalid format");
        AsciiBuilder::new_from_image(image)
            .set_resize((32, 16))
            .to_std_out(matches.is_present("color"));
    }
    //endregion
    Ok(())
}
