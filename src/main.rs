extern crate bincode;
#[macro_use]
extern crate clap;
extern crate scanlib;
#[macro_use]
extern crate serde_derive;

use clap::ArgMatches;
use std::path::Path;

fn main() {
    use clap::App;

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("incl") {
        incl(matches);
    }
}

fn incl(matches: &ArgMatches) {
    let infile = matches.value_of("INFILE").unwrap();
    let outfile = matches.value_of("OUTFILE").unwrap();

    let inclinations = read_inclinations(infile, matches.is_present("sync-to-pps"));
    write_inclinations(inclinations, outfile);
}

fn read_inclinations<P: AsRef<Path>>(path: P, sync_to_pps: bool) -> Vec<Inclination> {
    scanlib::inclinations_from_path(path, sync_to_pps)
        .unwrap()
        .into_iter()
        .map(|i| i.into())
        .collect()
}

fn write_inclinations<P: AsRef<Path>>(inclinations: Vec<Inclination>, path: P) {
    use std::fs::File;
    use bincode::Infinite;

    let mut write = File::create(path).unwrap();
    bincode::serialize_into(&mut write, &inclinations, Infinite).unwrap();
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Inclination {
    time: f64,
    roll: f32,
    pitch: f32,
}

impl From<scanlib::Inclination> for Inclination {
    fn from(i: scanlib::Inclination) -> Inclination {
        Inclination {
            time: i.time,
            roll: i.roll as f32,
            pitch: i.pitch as f32,
        }
    }
}
