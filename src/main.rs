extern crate ape;
extern crate chrono;
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate serde_json;
extern crate walkdir;

use ape::incl::{Inclination, Stats};
use ape::utils;
use chrono::{DateTime, TimeZone, Utc};
use clap::ArgMatches;
use walkdir::WalkDir;

lazy_static! {
    pub static ref SCANNER_SWAP: DateTime<Utc> = Utc.ymd(2016, 8, 12).and_hms(0, 0, 0);
}

#[allow(unused_variables)]
fn main() {
    use clap::App;

    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("incl") {
        if let Some(matches) = matches.subcommand_matches("extract") {
            #[cfg(target_os = "linux")] incl_extract(matches);
            #[cfg(not(target_os = "linux"))]
            panic!("ape-incl-extract not supported on non-linux systems");
        } else if let Some(matches) = matches.subcommand_matches("cat") {
            incl_cat(matches);
        } else if let Some(matches) = matches.subcommand_matches("stats") {
            incl_stats(matches);
        } else if let Some(matches) = matches.subcommand_matches("timeseries") {
            incl_timeseries(matches);
        }
    }
}

#[cfg(target_os = "linux")]
pub fn incl_extract(matches: &ArgMatches) {
    use ape::incl;
    let infile = matches.value_of("INFILE").unwrap();
    let outfile = matches.value_of("OUTFILE").unwrap();
    incl::linux::extract(infile, outfile, matches.is_present("sync-to-pps")).unwrap()
}

pub fn incl_cat(matches: &ArgMatches) {
    let inclinations = Inclination::vec_from_path(matches.value_of("INFILE").unwrap()).unwrap();
    println!("time,roll,pitch");
    for inclination in inclinations {
        println!(
            "{},{},{}",
            inclination.time,
            inclination.roll,
            inclination.pitch
        );
    }
}

pub fn incl_stats(matches: &ArgMatches) {
    let stats = Stats::from_path(matches.value_of("INFILE").unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
}

pub fn incl_timeseries(matches: &ArgMatches) {
    use chrono::{Datelike, Timelike};
    let directory = matches.value_of("DIRECTORY").unwrap();
    println!("ordinal,year,hour,name,mean,stddev");
    for entry in WalkDir::new(directory) {
        let entry = entry.unwrap();
        if entry.path().extension().map(|e| e == "incl").unwrap_or(
            false,
        )
        {
            let stats = Stats::from_path(entry.path()).unwrap();
            let datetime = utils::riegl_datetime_from_path(entry.path()).unwrap();
            let hour = datetime.hour();
            if hour % 6 == 0 {
                let ordinal = datetime.ordinal();
                let year = datetime.year();
                let year = if year == 2016 {
                    if datetime < *SCANNER_SWAP {
                        "2016-a"
                    } else {
                        "2016-b"
                    }
                } else {
                    "2015"
                };
                println!(
                    "{},{},{},{},{},{}",
                    ordinal,
                    year,
                    hour,
                    "roll",
                    stats.roll.mean,
                    stats.roll.stddev
                );
                println!(
                    "{},{},{},{},{},{}",
                    ordinal,
                    year,
                    hour,
                    "pitch",
                    stats.pitch.mean,
                    stats.pitch.stddev
                );
            }
        }
    }
}
