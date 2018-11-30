extern crate ape;
extern crate csv;
#[macro_use]
extern crate clap;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use ape::{Ape, Config, Sample};
use clap::App;
use csv::Writer;
use std::fs::File;

fn main() {
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();
    if let Some(matches) = matches.subcommand_matches("sample") {
        let outfile = File::create(matches.value_of("OUTFILE").unwrap()).unwrap();
        let config = Config::from_path(matches.value_of("CONFIG").unwrap()).unwrap();
        let ape = ape::process(
            config,
            matches.value_of("FIXED").unwrap(),
            matches.value_of("MOVING").unwrap(),
        ).unwrap();
        serde_json::to_writer(outfile, &ape).unwrap();
    } else if let Some(matches) = matches.subcommand_matches("to-csv") {
        let infile = File::open(matches.value_of("INFILE").unwrap()).unwrap();
        let ape: Ape = serde_json::from_reader(infile).unwrap();
        let mut writer = Writer::from_path(matches.value_of("OUTFILE").unwrap()).unwrap();
        for sample in ape.samples {
            writer.serialize(CsvSample::from(sample)).unwrap();
        }
    }
}

#[derive(Debug, Serialize)]
struct CsvSample {
    x: f64,
    y: f64,
    z: f64,
    v: f64,
    vxy: f64,
    vx: f64,
    vy: f64,
    vz: f64,
}

impl From<Sample> for CsvSample {
    fn from(sample: Sample) -> CsvSample {
        CsvSample {
            x: sample.x,
            y: sample.y,
            z: sample.z,
            v: (sample.velocity[0].powi(2)
                + sample.velocity[1].powi(2)
                + sample.velocity[2].powi(2)).sqrt(),
            vxy: (sample.velocity[0].powi(2) + sample.velocity[1].powi(2)).sqrt(),
            vx: sample.velocity[0],
            vy: sample.velocity[1],
            vz: sample.velocity[2],
        }
    }
}
