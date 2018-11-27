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

pub use config::Config;
pub use point::Point;

use failure::Error;
use las::Reader;
use pbr::MultiBar;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type RTree = spade::rtree::RTree<Point>;

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
    multi_bar.println("");
    multi_bar.println("Workers:");
    let sample_points = Arc::new(Mutex::new(sample_points));
    let fixed = Arc::new(fixed);
    let moving = Arc::new(moving);
    let (tx, rx) = mpsc::channel();
    let radius = config.step as f64 * config.step as f64;
    for _ in 0..config.threads {
        let sample_points = Arc::clone(&sample_points);
        let fixed = fixed.clone();
        let moving = moving.clone();
        let tx = tx.clone();
        let mut progress_bar = multi_bar.create_bar(config.max_iterations);

        thread::spawn(move || loop {
            let sample_point = {
                let mut sample_points = sample_points.lock().unwrap();
                sample_points.pop()
            };
            if let Some(sample_point) = sample_point {
                progress_bar.message("Lookup in circle: ");
                progress_bar.tick();
                let fixed_in_circle = fixed.lookup_in_circle(&sample_point, &radius);
                let moving_in_circle = moving.lookup_in_circle(&sample_point, &radius);
                let status = if fixed_in_circle.len() < config.min_points_in_circle
                    || moving_in_circle.len() < config.min_points_in_circle
                {
                    Status::TooFewPointsInCircle {
                        fixed: fixed_in_circle.len(),
                        moving: moving_in_circle.len(),
                    }
                } else {
                    unimplemented!()
                };

                let cell = Cell {
                    x: sample_point.x(),
                    y: sample_point.y(),
                    status: status,
                };
                tx.send(cell).unwrap();
            } else {
                return;
            }
        });
    }
    drop(tx);
    thread::spawn(move || multi_bar.listen());
    let mut cells = Vec::new();
    for cell in rx {
        cells.push(cell);
        progress_bar.inc();
    }
    Ok(Ape { cells: cells })
}

#[derive(Debug, Serialize)]
pub struct Ape {
    cells: Vec<Cell>,
}

#[derive(Debug, Serialize)]
pub struct Cell {
    x: f64,
    y: f64,
    status: Status,
}

#[derive(Debug, Serialize)]
pub enum Status {
    TooFewPointsInCircle { fixed: usize, moving: usize },
}
