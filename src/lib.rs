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

use failure::Error;
use las::Reader;
use nalgebra::Point3;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{
    mpsc::{self, Sender},
    Arc, Mutex,
};
use std::thread;

type RTree = spade::rtree::RTree<Point3<f64>>;

pub fn process<P: AsRef<Path>, Q: AsRef<Path>>(
    config: Config,
    fixed: P,
    moving: Q,
) -> Result<Ape, Error> {
    println!("Running the ATLAS Processing Engine with configuration:");
    println!("{}", toml::ser::to_string_pretty(&config)?);
    let sample_points = config.sample_points();
    println!("{} sample points", sample_points.len());

    let reader = Reader::new().add_path(fixed).add_path(moving);
    let mut rtrees = reader.read()?;
    let moving = rtrees.pop().unwrap();
    let fixed = rtrees.pop().unwrap();

    println!("");
    println!("Calculating velocities with {} workers", config.threads);
    let sample_points = Arc::new(Mutex::new(sample_points));
    let fixed = Arc::new(fixed);
    let moving = Arc::new(moving);
    let (tx, rx) = mpsc::channel();
    for _ in 0..config.threads {
        let sample_points = Arc::clone(&sample_points);
        let fixed = fixed.clone();
        let moving = moving.clone();
        let tx = tx.clone();
        thread::spawn(move || create_worker(config, sample_points, fixed, moving, tx));
    }
    drop(tx);
    let mut cells = Vec::new();
    for cell in rx {
        cells.push(cell);
    }
    Ok(Ape { cells: cells })
}

#[derive(Debug, Serialize)]
pub struct Ape {
    cells: Vec<Cell>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Config {
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
    step: usize,
    threads: usize,
}

#[derive(Debug, Serialize)]
pub struct Cell {
    x: f64,
    y: f64,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let config: Config = toml::de::from_str(&string)?;
        Ok(config)
    }

    fn sample_points(&self) -> Vec<(f64, f64)> {
        let mut points = Vec::new();
        for x in (self.minx..self.maxx).step_by(self.step) {
            for y in (self.miny..self.maxy).step_by(self.step) {
                let step = self.step as f64;
                let x = f64::from(x) + step / 2.;
                let y = f64::from(y) + step / 2.;
                points.push((x, y));
            }
        }
        points
    }
}

impl Cell {
    fn new(_config: Config, _fixed: &RTree, _moving: &RTree, _x: f64, _y: f64) -> Cell {
        unimplemented!()
    }
}

fn create_worker(
    config: Config,
    sample_points: Arc<Mutex<Vec<(f64, f64)>>>,
    fixed: Arc<RTree>,
    moving: Arc<RTree>,
    tx: Sender<Cell>,
) {
    loop {
        let sample_point = {
            let mut sample_points = sample_points.lock().unwrap();
            sample_points.pop()
        };
        if let Some((x, y)) = sample_point {
            let cell = Cell::new(config, &fixed, &moving, x, y);
            tx.send(cell).unwrap();
        } else {
            return;
        }
    }
}
