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
pub use sample::Sample;

use failure::Error;
use las::Reader;
use pbr::MultiBar;
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

    let reader = Reader::new().add_path(fixed).add_path(moving);
    let mut rtrees = reader.read()?;
    let moving = rtrees.pop().unwrap();
    let fixed = rtrees.pop().unwrap();

    println!("");
    let sample_points = config.sample_points();
    let mut multi_bar = MultiBar::new();
    multi_bar.println(&format!(
        "Calculating velocities at {} points using {} workers:",
        sample_points.len(),
        config.threads
    ));
    let mut progress_bar = multi_bar.create_bar(sample_points.len() as u64);
    progress_bar.message("Overall progress: ");
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
                let sample = Sample::new(config, &fixed, &moving, sample_point);
                tx.send(sample).unwrap();
            } else {
                return;
            }
        });
    }
    drop(tx);
    thread::spawn(move || multi_bar.listen());
    let mut samples = Vec::new();
    for sample in rx {
        let sample = sample?;
        if let Some(sample) = sample {
            samples.push(sample);
        }
        progress_bar.inc();
    }
    progress_bar.finish();
    Ok(Ape { samples: samples })
}

#[derive(Debug, Serialize)]
pub struct Ape {
    samples: Vec<Sample>,
}
