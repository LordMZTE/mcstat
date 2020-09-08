use clap::{App, Arg};
use image::ImageFormat;
use asciify::AsciiBuilder;
use std::io::Cursor;

fn main() {
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

    let response = mcio::ping(
        matches.value_of("ip").unwrap(),
        matches.value_of("port").unwrap().parse().ok().and_then(|p| if p > 0 && p < u16::MAX {Some(p)} else {None}).expect("invalid port"),
        matches.value_of("protocol-version").unwrap().parse().expect("invalid protocol version"),
    ).expect("invalid response from server");


    //region printing
    macro_rules! print_table {
        ($($l:expr => $k:expr),+) => {
            $(println!("{: <15} | {}", $l, $k);)*
        };
    }

    print_table!(
        "Online Players" => response.players.online,
        "Max Players" => response.players.max,
        "Server Version" => response.version.name,
        "Server Protocol" => response.version.protocol
    );

    //Image
    if matches.is_present("image") {
        let img = image_base64::from_base64(response.favicon);
        let image = image::load(Cursor::new(img), ImageFormat::Png).expect("favicon has invalid format");
        AsciiBuilder::new_from_image(image)
            .set_resize((32, 16))
            .to_std_out(matches.is_present("color"));
    }
    //endregion
}
