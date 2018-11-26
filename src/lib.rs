extern crate failure;
extern crate las;
extern crate nalgebra;
extern crate pbr;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate spade;

use failure::Error;
use nalgebra::Point3;
use pbr::{MultiBar, Pipe, ProgressBar};
use spade::rtree::RTree;
use std::fs::File;
use std::io::{BufReader, Stdout};
use std::path::Path;
use std::thread;
use std::time::Duration;

const PROGRESS_BAR_MAX_REFRESH_RATE_MS: u64 = 100;

pub fn process<P: AsRef<Path>, Q: AsRef<Path>>(
    config: Config,
    fixed: P,
    moving: Q,
) -> Result<Ape, Error> {
    let (fixed, moving) = read_las_files(fixed, moving)?;
    let cells = Cells::new(config);
    unimplemented!()
}

#[derive(Debug, Default, Serialize)]
pub struct Ape {}

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    minx: i32,
    miny: i32,
    maxx: i32,
    maxy: i32,
    step: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct Cells(Vec<Cell>);

#[derive(Debug, Default, Serialize)]
pub struct Cell {
    /// The x coordinate of the center of the cell.
    x: f64,

    /// The y coordinate of the center of the cell.
    y: f64,
}

struct Reader {
    progress_bar: ProgressBar<Pipe>,
    reader: las::Reader<BufReader<File>>,
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

impl Cells {
    fn new(config: Config) -> Cells {
        let mut cells = Vec::new();
        for y in (config.miny..config.maxy).step_by(config.step) {
            for x in (config.minx..config.maxx).step_by(config.step) {
                cells.push(Cell::new(x, y, config.step));
            }
        }
        Cells(cells)
    }
}

impl Cell {
    fn new(x: i32, y: i32, step: usize) -> Cell {
        Cell {
            x: f64::from(x) + step as f64 / 2.,
            y: f64::from(y) + step as f64 / 2.,
        }
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
