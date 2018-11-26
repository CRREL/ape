extern crate las;
extern crate nalgebra;
extern crate spade;

use nalgebra::Point3;
use spade::rtree::RTree;
use std::path::Path;

/// An ATLAS processing engine.
#[derive(Debug)]
pub struct Ape {
    fixed: RTree<Point3<f64>>,
    moving: RTree<Point3<f64>>,
}

impl Ape {
    /// Creates a new processing engine from two las files.
    ///
    /// # Examples
    ///
    /// ```
    /// let ape = ape::Ape("fixtures/fixed.las", "fixtures/moving.las");
    /// ```
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(fixed: P, moving: Q) -> Result<Ape, las::Error> {
        Ok(Ape {
            fixed: las_to_rtree(fixed)?,
            moving: las_to_rtree(moving)?,
        })
    }
}

fn las_to_rtree<P: AsRef<Path>>(path: P) -> Result<RTree<Point3<f64>>, las::Error> {
    let reader = las::Reader::from_path(path)?;
    unimplemented!()
}
