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
}
