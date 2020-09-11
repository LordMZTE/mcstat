#[macro_use]
extern crate clap;

use std::error::Error;
use std::io::Cursor;

use asciify::AsciiBuilder;
use async_minecraft_ping::ConnectionConfig;
use clap::App;
use image::ImageFormat;
use itertools::Itertools;

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
        se "Server Version" => response.version.name,
        s "Online Players" => response.players.online,
        s "Max Players" => response.players.max,
        s "Server Protocol" => response.version.protocol
    );

    //Image
    if let (Some(favicon), true) = (response.favicon, matches.is_present("image")) {
        let img = image_base64::from_base64(favicon);
        let image =
            image::load(Cursor::new(img), ImageFormat::Png).expect("favicon has invalid format");
        let image_size: u32 = matches
            .value_of("size")
            .unwrap()
            .parse()
            .expect("image size must be number");
        AsciiBuilder::new_from_image(image)
            .set_resize((image_size * 2, image_size))
            .to_std_out(matches.is_present("color"));
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
