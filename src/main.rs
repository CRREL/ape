extern crate bincode;
extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[cfg(target_os = "linux")]
extern crate scanlib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate walkdir;

use std::path::Path;
use chrono::{DateTime, Utc, TimeZone};

lazy_static! {
    static ref SCANNER_SWAP: DateTime<Utc> = Utc.ymd(2016, 8, 12).and_hms(0, 0, 0);
}

#[allow(unused_variables)]
fn main() {
    use clap::App;
    #[cfg(target_os = "linux")] linux::incl(matches);

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("incl") {
        if let Some(matches) = matches.subcommand_matches("extract") {
            #[cfg(target_os = "linux")] incl::linux::extract(matches);
            #[cfg(not(target_os = "linux"))]
            panic!("ape-incl-extract not supported on non-linux systems");
        } else if let Some(matches) = matches.subcommand_matches("stats") {
            incl::stats(matches);
        } else if let Some(matches) = matches.subcommand_matches("timeseries") {
            incl::timeseries(matches);
        }
    }
}

mod incl {
    use bincode::{self, Infinite};
    use clap::ArgMatches;
    use serde_json::{self, Value};
    use std::path::Path;
    use walkdir::WalkDir;
    use chrono::{Timelike, Datelike};

    #[derive(Clone, Copy, Debug, Serialize, Deserialize)]
    struct Inclination {
        time: f64,
        roll: f32,
        pitch: f32,
    }

    #[derive(Debug, Default)]
    struct Stats(Vec<f32>);

    pub fn stats(matches: &ArgMatches) {
        let mut roll = Stats::default();
        let mut pitch = Stats::default();
        for inclination in Inclination::read_from(matches.value_of("INFILE").unwrap()) {
            roll.add(inclination.roll);
            pitch.add(inclination.pitch);
        }
        let stats = json!({
            "roll": roll.as_json(),
            "pitch": pitch.as_json(),
        });
        println!("{}", serde_json::to_string(&stats).unwrap());
    }

    pub fn timeseries(matches: &ArgMatches) {
        let directory = matches.value_of("DIRECTORY").unwrap();
        println!("ordinal,year,hour,name,mean,stddev");
        for entry in WalkDir::new(directory) {
            let entry = entry.unwrap();
            if entry.path().extension().map(|e| e == "incl").unwrap_or(
                false,
            )
            {
                let inclinations = Inclination::read_from(entry.path());
                let datetime = super::riegl_datetime_from_path(entry.path());
                let hour = datetime.hour();
                if hour % 6 == 0 {
                    let ordinal = datetime.ordinal();
                    let year = datetime.year();
                    let year = if year == 2016 {
                        if datetime < *super::SCANNER_SWAP {
                            "2016-a"
                        } else {
                            "2016-b"
                        }
                    } else {
                        "2015"
                    };
                    let roll = Stats::new(inclinations.iter().map(|i| i.roll).collect());
                    let pitch = Stats::new(inclinations.iter().map(|i| i.pitch).collect());
                    println!("{},{},{},{},{},{}", ordinal, year, hour, "roll", roll.mean(), roll.stddev());
                    println!("{},{},{},{},{},{}", ordinal, year, hour, "pitch", pitch.mean(), pitch.stddev());
                }
            }
        }
    }

    impl Inclination {
        fn read_from<P: AsRef<Path>>(path: P) -> Vec<Inclination> {
            use std::fs::File;
            bincode::deserialize_from(&mut File::open(path).unwrap(), Infinite).unwrap()
        }
    }

    impl Stats {
        fn new(v: Vec<f32>) -> Stats {
            Stats(v)
        }

        fn add<T: Into<f32>>(&mut self, n: T) {
            self.0.push(n.into());
        }

        fn as_json(&self) -> Value {
            json!({
                "mean": self.mean(),
                "stddev": self.stddev(),
                "variance": self.variance(),
                "count": self.0.len(),
            })
        }

        fn mean(&self) -> f32 {
            self.0.iter().sum::<f32>() / self.0.len() as f32
        }

        fn variance(&self) -> f32 {
            self.0.iter().map(|n| n.powi(2)).sum::<f32>() / self.0.len() as f32
        }

        fn stddev(&self) -> f32 {
            self.variance().sqrt()
        }
    }

    #[cfg(target_os = "linux")]
    mod linux {
        use bincode;
        use scanlib;
        use std::path::Path;
        use super::Inclination;

        pub fn incl(matches: &ArgMatches) {
            let infile = matches.value_of("INFILE").unwrap();
            let outfile = matches.value_of("OUTFILE").unwrap();

            let inclinations = read_inclinations(infile, matches.is_present("sync-to-pps"));
            write_inclinations(inclinations, outfile);
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
    }
}

fn riegl_datetime_from_path<P: AsRef<Path>>(path: P) -> DateTime<Utc> {
    use chrono::TimeZone;
    Utc.datetime_from_str(
        path.as_ref()
            .file_stem()
            .expect("file stem")
            .to_string_lossy()
            .as_ref(),
        "%y%m%d_%H%M%S",
    ).unwrap()
}
