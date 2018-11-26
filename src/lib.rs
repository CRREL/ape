extern crate failure;
extern crate las;
extern crate nalgebra;
extern crate pbr;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate spade;
extern crate toml;

use failure::Error;
use nalgebra::Point3;
use pbr::{MultiBar, Pipe, ProgressBar};
use spade::rtree::RTree;
use std::fs::File;
use std::io::{BufReader, Read, Stdout};
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const PROGRESS_BAR_MAX_REFRESH_RATE_MS: u64 = 100;

pub fn process<P: AsRef<Path>, Q: AsRef<Path>>(
    config: Config,
    fixed: P,
    moving: Q,
) -> Result<Ape, Error> {
    println!("Running the ATLAS Processing Engine with configuration:");
    println!("{}", toml::ser::to_string_pretty(&config)?);
    let (fixed, moving) = read_las_files(fixed, moving)?;

    let fixed = Arc::new(fixed);
    let moving = Arc::new(moving);
    let mut handles = Vec::new();
    for i in 0..config.threads {
        let fixed = Arc::clone(&fixed);
        let moving = Arc::clone(&moving);
        let handle = thread::spawn(move || create_worker(i, fixed, moving));
        handles.push(handle);
    }
    for handle in handles {
        handle.join().unwrap();
    }
    Ok(Ape {})
}

#[derive(Debug, Default, Serialize)]
pub struct Ape {}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
    step: usize,
    threads: usize,
}

struct Reader {
    progress_bar: ProgressBar<Pipe>,
    reader: las::Reader<BufReader<File>>,
}

impl Config {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let config: Config = toml::de::from_str(&string)?;
        Ok(config)
    }
}

impl Reader {
    fn new<P: AsRef<Path>>(
        path: P,
        multi_bar: &mut MultiBar<Stdout>,
    ) -> Result<Reader, las::Error> {
        let reader = las::Reader::from_path(&path)?;
        let mut progress_bar = multi_bar.create_bar(reader.header().number_of_points());
        progress_bar.set_max_refresh_rate(Some(Duration::from_millis(
            PROGRESS_BAR_MAX_REFRESH_RATE_MS,
        )));
        progress_bar.message(&format!(
            "{}: ",
            path.as_ref()
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(String::new)
        ));
        Ok(Reader {
            progress_bar: progress_bar,
            reader: reader,
        })
    }

    fn build(&mut self) -> Result<RTree<Point3<f64>>, las::Error> {
        let mut rtree = RTree::new();
        for point in self.reader.points() {
            let point = point?;
            let point = Point3::new(point.x, point.y, point.z);
            rtree.insert(point);
            self.progress_bar.inc();
        }
        self.progress_bar.finish();
        Ok(rtree)
    }
}
fn read_las_files<P: AsRef<Path>, Q: AsRef<Path>>(
    fixed: P,
    moving: Q,
) -> Result<(RTree<Point3<f64>>, RTree<Point3<f64>>), las::Error> {
    let mut multi_bar = MultiBar::new();
    multi_bar.println("Reading las files into RTrees");
    let mut fixed = Reader::new(fixed, &mut multi_bar)?;
    let fixed = thread::spawn(move || fixed.build());
    let mut moving = Reader::new(moving, &mut multi_bar)?;
    let moving = thread::spawn(move || moving.build());
    thread::spawn(move || {
        multi_bar.listen();
    });
    let fixed = fixed.join().unwrap()?;
    let moving = moving.join().unwrap()?;
    Ok((fixed, moving))
}

fn create_worker(_id: usize, _fixed: Arc<RTree<Point3<f64>>>, _moving: Arc<RTree<Point3<f64>>>) {
    unimplemented!()
}
