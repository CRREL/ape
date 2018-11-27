//! Read las data for import and use.

use las_rs::{Error, Reader as LasReader};
use pbr::MultiBar;
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use {Point, RTree};

const PROGRESS_BAR_MAX_REFRESH_RATE_MS: u64 = 100;

/// A multi-threaded las reader.
///
/// Reads las files into rtrees.
pub struct Reader {
    paths: Vec<PathBuf>,
}

impl Reader {
    /// Creates a new reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::las::Reader;
    /// let reader = Reader::new();
    /// ```
    pub fn new() -> Reader {
        Reader { paths: Vec::new() }
    }

    /// Adds a new path to this reader.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::las::Reader;
    /// let reader = Reader::new().add_path("infile.las");
    /// ```
    pub fn add_path<P: AsRef<Path>>(mut self, path: P) -> Reader {
        self.paths.push(path.as_ref().to_path_buf());
        self
    }

    /// Reads the las files into rtrees, returning the rtrees.
    ///
    /// One thread per las file.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use ape::las::Reader;
    /// let reader = Reader::new().add_path("one.las").add_path("two.las");
    /// let rtrees = reader.read().unwrap();
    /// assert_eq!(2, rtrees.len());
    /// ```
    pub fn read(&self) -> Result<Vec<RTree>, Error> {
        let mut multi_bar = MultiBar::new();
        multi_bar.println(&format!("Reading {} las files:", self.paths.len()));
        let mut handles = Vec::new();
        for path in &self.paths {
            let mut reader = LasReader::from_path(path)?;
            let mut progress_bar = multi_bar.create_bar(reader.header().number_of_points());
            progress_bar.set_max_refresh_rate(Some(Duration::from_millis(
                PROGRESS_BAR_MAX_REFRESH_RATE_MS,
            )));
            progress_bar.message(&format!(
                "{}: ",
                path.file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(String::new)
            ));
            let handle: JoinHandle<Result<RTree, Error>> = thread::spawn(move || {
                let mut rtree = RTree::new();
                for point in reader.points() {
                    let point = point?;
                    let point = Point::new(point.x, point.y, point.z);
                    rtree.insert(point);
                    progress_bar.inc();
                }
                progress_bar.finish();
                Ok(rtree)
            });
            handles.push(handle);
        }
        thread::spawn(move || multi_bar.listen());
        let mut rtrees = Vec::new();
        for handle in handles {
            rtrees.push(handle.join().unwrap()?);
        }
        Ok(rtrees)
    }
}
