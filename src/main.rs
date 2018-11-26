extern crate ape;
#[macro_use]
extern crate clap;

use ape::Config;
use clap::App;
use std::fs::File;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let outfile = File::create(matches.value_of("OUTFILE").unwrap()).unwrap();
    let config = Config::from_path(matches.value_of("CONFIG").unwrap()).unwrap();
    let ape = ape::process(
        config,
        matches.value_of("FIXED").unwrap(),
        matches.value_of("MOVING").unwrap(),
    ).unwrap();
    serde_json::to_writer(outfile, &ape).unwrap();
}
