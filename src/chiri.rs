//! CLI chiri tool

use die::{Die, die};

use std::env;

/// CLI entrypoint
fn main() {
    let brief: String = format!("Usage: {} [<OPTIONS>]", env!("CARGO_PKG_NAME"));

    let mut opts: getopts::Options = getopts::Options::new();
    opts.optflag("c", "clean", "remove artifacts");
    opts.optflag("d", "debug", "enable additional logging");
    opts.optflag("h", "help", "show usage menu");
    opts.optflag("v", "version", "show version banner");

    let usage: String = opts.usage(&brief);
    let arguments: Vec<String> = env::args().collect();
    let optmatches: getopts::Matches = opts.parse(&arguments[1..]).die(&usage);

    if !optmatches.free.is_empty() {
        die!(usage);
    }

    if optmatches.opt_present("h") {
        die!(0; usage);
    }

    if optmatches.opt_present("v") {
        die!(0; format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
    }

    let mut c = match chiri::Chiri::load(chiri::CONFIGURATION_FILENAME) {
        Err(e) => die!(1; format!("error: {e}")),
        Ok(e) => e,
    };

    if optmatches.opt_present("d") {
        c.debug = Some(true);
    }

    if let Some(true) = c.debug {
        eprintln!("debug: configuration: {:?}", c);
    }

    if optmatches.opt_present("c") {
        if let Err(e) = c.clean() {
            die!(1; format!("error: {e}"));
        } else {
            die!(0);
        }
    }

    if let Err(e) = c.bloom() {
        die!(1; format!("error: {e}"));
    }
}
