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

pub use config::Config;

use failure::Error;
use las::Reader;
use nalgebra::Point3;
use pbr::MultiBar;
use std::path::Path;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type RTree = spade::rtree::RTree<Point3<f64>>;

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
    multi_bar.println("Individual workers:");
    let sample_points = Arc::new(Mutex::new(sample_points));
    let fixed = Arc::new(fixed);
    let moving = Arc::new(moving);
    let (tx, rx) = mpsc::channel();
    for _ in 0..config.threads {
        let sample_points = Arc::clone(&sample_points);
        let fixed = fixed.clone();
        let moving = moving.clone();
        let tx = tx.clone();
        let progress_bar = multi_bar.create_bar(config.max_iterations);
        thread::spawn(move || loop {
            let sample_point = {
                let mut sample_points = sample_points.lock().unwrap();
                sample_points.pop()
            };
            if let Some((x, y)) = sample_point {
                let cell = Cell { x: x, y: y };
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
}
