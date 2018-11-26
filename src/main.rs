extern crate ape;
#[macro_use]
extern crate clap;

use ape::Config;
use clap::App;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    let _ = ape::process(
        Config::default(),
        matches.value_of("FIXED").unwrap(),
        matches.value_of("MOVING").unwrap(),
    );
}
