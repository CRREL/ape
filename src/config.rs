use cpd::Matrix;
use failure::Error;
use nalgebra::U3;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use {Point, RTree};

/// Processing engine configuration.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct Config {
    /// The minimum x coordinate of the sample grid.
    pub minx: i32,

    /// The minimum y coordiate of the sample grid.
    pub miny: i32,

    /// The maximum x coordinate of the sample grid.
    pub maxx: i32,

    /// The maximum y coordinate of the sample grid.
    pub maxy: i32,

    /// The step size between sample coordinates.
    pub step: usize,

    /// The number of threads to use for CPD calculations.
    pub threads: usize,

    /// The minimum number point density that will permit CPD to be run.
    pub min_density: f64,

    /// The number of points to be used for the CPD calculation.
    pub num_points: usize,

    /// The maximum number of iterations to do of CPD.
    pub max_iterations: Option<u64>,
}

impl Config {
    /// Creates a configuration from a TOML file.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::Config;
    /// let config = Config::from_path("src/config.toml").unwrap();
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Config, Error> {
        let mut file = File::open(path)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        let config: Config = toml::de::from_str(&string)?;
        Ok(config)
    }

    /// Return a grid of sample points, as determined by this configuration.
    ///
    /// The grid is just a vector of points, with x and y values set and z set to zero. The points
    /// are centered in the middle of the "rectangles" defined by the min/max coordinates and the
    /// steps.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::Config;
    /// let config = Config::from_path("src/config.toml").unwrap();
    /// let sample_points = config.sample_points();
    /// ```
    pub fn sample_points(&self) -> Vec<Point> {
        let mut points = Vec::new();
        for x in (self.minx..self.maxx).step_by(self.step) {
            for y in (self.miny..self.maxy).step_by(self.step) {
                let step = self.step as f64;
                let x = f64::from(x) + step / 2.;
                let y = f64::from(y) + step / 2.;
                points.push(Point::new(x, y, 0.));
            }
        }
        points
    }

    /// Returns the nearest neighbors from the provided rtree, centered around the point, as a
    /// matrix.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::{Config, RTree, Point};
    /// let config = Config::from_path("src/config.toml").unwrap();
    /// let rtree = RTree::new();
    /// let point = Point::new(1., 2., 3.);
    /// let neighbors = config.nearest_neighbors(&rtree, &point);
    /// ```
    pub fn nearest_neighbors(&self, rtree: &RTree, point: &Point) -> Matrix<U3> {
        let points = rtree.nearest_n_neighbors(point, self.num_points);
        let mut matrix = Matrix::<U3>::zeros(points.len());
        for (i, point) in points.iter().enumerate() {
            matrix[(i, 0)] = point.x();
            matrix[(i, 1)] = point.y();
            matrix[(i, 2)] = point.z();
        }
        matrix
    }

    /// Looks up the points in a circle around a point.
    pub fn lookup_in_circle<'a>(&self, rtree: &'a RTree, point: &Point) -> Vec<&'a Point> {
        rtree.lookup_in_circle(point, &self.radius2())
    }

    pub fn density(&self, rtree: &RTree, point: &Point) -> f64 {
        use std::f64::consts::PI;
        let area = PI * self.radius2();
        self.lookup_in_circle(rtree, point).len() as f64 / area
    }

    fn radius2(&self) -> f64 {
        self.step as f64 * self.step as f64
    }
}
