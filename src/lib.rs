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

/// Run the ATLAS processing engine.
pub fn process<P: AsRef<Path>, Q: AsRef<Path>>(
    config: Config,
    fixed: P,
    moving: Q,
) -> Result<Grid, Error> {
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

    Ok(Grid::new(config, fixed, moving))
}

#[derive(Debug, Default, Deserialize)]
pub struct Config {}

#[derive(Debug, Serialize)]
pub struct Grid {}

struct Reader {
    progress_bar: ProgressBar<Pipe>,
    reader: las::Reader<BufReader<File>>,
}

impl Grid {
    fn new(config: Config, fixed: RTree<Point3<f64>>, moving: RTree<Point3<f64>>) -> Grid {
        unimplemented!()
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
