extern crate chrono;
extern crate cpd;
extern crate failure;
extern crate las as las_rs;
extern crate nalgebra;
extern crate pbr;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate spade;
extern crate toml;

pub mod las;

mod config;
mod point;
mod sample;

pub use config::Config;
pub use point::Point;
pub use sample::{LowDensitySample, Sample};

use chrono::{DateTime, ParseResult, TimeZone, Utc};
use failure::Error;
use las::Reader;
use pbr::ProgressBar;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

/// Our version of the rtree.
pub type RTree = spade::rtree::RTree<Point>;

const PROGRESS_BAR_MAX_REFRESH_RATE_MS: u64 = 100;

pub fn process<P: AsRef<Path>, Q: AsRef<Path>>(
    config: Config,
    fixed: P,
    moving: Q,
) -> Result<Ape, Error> {
    println!(
        "Welcome to the ATLAS Processing Engine.\n\nConfiguration:\n{}",
        toml::ser::to_string_pretty(&config)?
    );

    let first = datetime_from_path(&moving)?;
    let second = datetime_from_path(&fixed)?;
    let duration = (second - first).to_std()?;
    println!(
        "First scan (moving): {}\nSecond scan (fixed): {}\nTime elapsed between scans: {}s\n",
        first,
        second,
        duration.as_secs()
    );

    let reader = Reader::new().add_path(moving).add_path(fixed);
    let mut rtrees = reader.read()?;
    let moving = rtrees.pop().unwrap();
    let fixed = rtrees.pop().unwrap();

    let sample_points = config.sample_points();
    println!(
        "\nCalculating velocities at {} points using {} workers:",
        sample_points.len(),
        config.threads
    );

    let mut progress_bar = ProgressBar::new(sample_points.len() as u64);
    progress_bar.message("Culling empty samples: ");
    let sample_points: Vec<_> = sample_points
        .into_iter()
        .filter(|point| {
            let fixed = config.lookup_in_circle(&fixed, &point);
            let moving = config.lookup_in_circle(&moving, &point);
            progress_bar.inc();
            !fixed.is_empty() && !moving.is_empty()
        }).collect();
    progress_bar.finish();

    println!("");
    let mut progress_bar = ProgressBar::new(sample_points.len() as u64);
    progress_bar.message("Culling low density samples: ");
    let mut low_density_samples = Vec::new();
    let sample_points: Vec<_> = sample_points
        .into_iter()
        .filter(|point| {
            let fixed = config.density(&fixed, &point);
            let moving = config.density(&moving, &point);
            progress_bar.inc();
            if fixed < config.min_density || moving < config.min_density {
                low_density_samples.push(LowDensitySample {
                    x: point.x(),
                    y: point.y(),
                    fixed: fixed,
                    moving: moving,
                });
                false
            } else {
                true
            }
        }).collect();
    progress_bar.finish();

    println!("");
    let mut progress_bar = ProgressBar::new(sample_points.len() as u64);
    progress_bar.message("Sampling velocities: ");
    progress_bar.tick();
    let sample_points = Arc::new(Mutex::new(sample_points));
    let fixed = Arc::new(fixed);
    let moving = Arc::new(moving);
    let (tx, rx) = mpsc::channel();
    for _ in 0..config.threads {
        let sample_points = Arc::clone(&sample_points);
        let fixed = fixed.clone();
        let moving = moving.clone();
        let tx = tx.clone();

        thread::spawn(move || loop {
            let sample_point = {
                let mut sample_points = sample_points.lock().unwrap();
                sample_points.pop()
            };
            if let Some(sample_point) = sample_point {
                let sample = Sample::new(config, &fixed, &moving, sample_point, duration);
                tx.send(sample).unwrap();
            } else {
                return;
            }
        });
    }
    drop(tx);
    let mut samples = Vec::new();
    for sample in rx {
        let sample = sample?;
        samples.push(sample);
        progress_bar.inc();
    }
    progress_bar.finish();
    Ok(Ape {
        samples: samples,
        low_density_samples: low_density_samples,
    })
}

#[derive(Debug, Serialize)]
pub struct Ape {
    samples: Vec<Sample>,
    low_density_samples: Vec<LowDensitySample>,
}

fn datetime_from_path<P: AsRef<Path>>(path: P) -> ParseResult<DateTime<Utc>> {
    let file_name = path
        .as_ref()
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(String::new);
    Utc.datetime_from_str(&file_name, "%y%m%d_%H%M%S.las")
}
