use failure::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

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

    /// The maximum number of iterations for each CPD run.
    pub max_iterations: u64,
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
    /// The grid is just a vector of (x, y) tuples. The points are centered in the middle of the
    /// "rectangles" defined by the min/max coordinates and the steps.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::Config;
    /// let config = Config::from_path("src/config.toml").unwrap();
    /// let sample_points = config.sample_points();
    /// ```
    pub fn sample_points(&self) -> Vec<(f64, f64)> {
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
