use crate::output::Table;
use async_minecraft_ping::StatusResponse;
use image::{DynamicImage, ImageFormat};
use itertools::Itertools;
use miette::{bail, miette, IntoDiagnostic, WrapErr};
use serde::Deserialize;
use std::{io::Cursor, net::IpAddr};
use tracing::info;
use trust_dns_resolver::TokioAsyncResolver;

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

#[derive(Deserialize)]
#[serde(untagged)]
pub enum EitherStatusResponse {
    Text { text: String },
    Normal(StatusResponse),
}

pub async fn resolve_address(addr_and_port: &str) -> miette::Result<(String, u16)> {
    info!("Resolving address");
    let addr;
    let port;
    if let Some((addr_, port_)) = addr_and_port.split_once(':') {
        info!("Address has explicit port");
        addr = addr_;
        port = Some(
            port_
                .parse()
                .into_diagnostic()
                .wrap_err("User provided port is invalid")?,
        );
    } else {
        info!("Address has no explicit port");
        addr = addr_and_port;
        port = None;
    }

    if let Some(port) = port {
        Ok((addr.to_string(), port))
    } else if addr.parse::<IpAddr>().is_ok() {
        info!("Got IP address without explicit port, assuming 25565");
        // if we only have an IP and no port, there is no domain to lookup so we can
        // only default to port 25565.
        Ok((addr.to_string(), 25565))
    } else {
        info!("Sending SRV request");
        let dns = TokioAsyncResolver::tokio_from_system_conf()
            .into_diagnostic()
            .wrap_err("Failed to create DNS resolver")?;

        let lookup = dns.srv_lookup(format!("_minecraft._tcp.{}.", addr)).await;

        if let Ok(lookup) = lookup {
            info!("Found SRV record");
            let srv = lookup
                .iter()
                .next()
                .ok_or_else(|| miette!("No SRV record found"))?;

            let addr = srv.target().to_string();
            let addr = addr.trim_end_matches('.');

            let port = srv.port();

            Ok((addr.to_string(), port))
        } else {
            info!("No SRV record found. Defaulting to 25565");
            // if there is no SRV record, we have to default to port 25565
            Ok((addr.to_string(), 25565))
        }
    }
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
            table.small_entry(entry.0, entry.1.to_string());
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
pub fn parse_base64_image(data: String) -> miette::Result<DynamicImage> {
    info!("Parsing base64 image");
    let (header, data) = data
        .split_once(',')
        .ok_or_else(|| miette!("Couldn't parse base64 image due to missing format header."))?;
    let (data_type, image_format) = header
        .split_once('/')
        .ok_or_else(|| miette!("Failed to parse base64 image, header has invalid format."))?;
    let image_format = image_format
        .split(';')
        .next()
        .ok_or_else(|| miette!("Failed to parse base64 image, header has invalid format."))?;

    if data_type != "data:image" {
        bail!("base64 image is not an image! Has type {}", data_type);
    }

    let format = ImageFormat::from_extension(image_format).ok_or_else(|| {
        miette!(
            "Failed to parse base64 image due to unknown image type: {}",
            image_format
        )
    })?;
    let data = base64::decode(data)
        .into_diagnostic()
        .wrap_err("Failed to decode base64 image data")?;
    image::load(Cursor::new(data), format)
        .into_diagnostic()
        .wrap_err("Failed to load base64 image")
}
