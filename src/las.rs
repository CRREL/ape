//! Read las data for import and use.

use las_rs::Error;
use std::path::{Path, PathBuf};
use RTree;

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
        unimplemented!()
    }
}
