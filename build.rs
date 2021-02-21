use clap::Shell;
use std::{env, str::FromStr};

include!("src/cli.rs");

fn main() {
    let outdir = match env::var_os("OUT_DIR") {
        None => return,
        Some(d) => d,
    };

    let mut app = get_app();
    for s in Shell::variants()
        .iter()
        .map(|v| Shell::from_str(v).unwrap())
    {
        app.gen_completions("mcstat", s, &outdir);
    }
}
