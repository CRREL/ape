extern crate bincode;
#[macro_use]
extern crate clap;
#[cfg(target_os = "linux")]
extern crate scanlib;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;

use bincode::Infinite;
use clap::ArgMatches;
use serde_json::Value;

#[allow(unused_variables)]
fn main() {
    use clap::App;
    #[cfg(target_os = "linux")] linux::incl(matches);

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("incl") {
        #[cfg(target_os = "linux")] linux::incl(matches);
        #[cfg(not(target_os = "linux"))]
        panic!("ape-incl not supported on non-linux systems");
    } else if let Some(matches) = matches.subcommand_matches("incl-stats") {
        incl_stats(matches);
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Inclination {
    time: f64,
    roll: f32,
    pitch: f32,
}

#[derive(Debug)]
struct Stats(Vec<f64>);

fn incl_stats(matches: &ArgMatches) {
    use std::fs::File;

    let infile = matches.value_of("INFILE").unwrap();
    let inclinations: Vec<Inclination> =
        bincode::deserialize_from(&mut File::open(infile).unwrap(), Infinite).unwrap();
    let mut roll = Stats::new();
    let mut pitch = Stats::new();
    for inclination in inclinations {
        roll.add(inclination.roll);
        pitch.add(inclination.pitch);
    }
    let stats = json!({
        "roll": roll.as_json(),
        "pitch": pitch.as_json(),
    });
    println!("{}", serde_json::to_string(&stats).unwrap());
}

impl Stats {
    fn new() -> Stats {
        Stats(Vec::new())
    }

    fn add<T: Into<f64>>(&mut self, n: T) {
        self.0.push(n.into());
    }

    fn as_json(&self) -> Value {
        use std::f64;

        let mut sum = 0.;
        let mut sum2 = 0.;
        let count = self.0.len();
        let mut max = f64::NEG_INFINITY;
        let mut min = f64::INFINITY;
        for &n in self.0.iter() {
            sum += n;
            sum2 += n.powi(2);
            min = min.min(n);
            max = max.max(n);
        }
        let variance = sum2 / count as f64;
        json!({
            "mean": sum / count as f64,
            "stddev": variance.sqrt(),
            "variance": variance,
            "count": count,
            "min": min,
            "max": max,
        })
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
