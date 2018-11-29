use {Config, Point, RTree};

/// A sample of the glacier's velocity.
#[derive(Debug, Serialize)]
pub struct Sample {
    x: f64,
    y: f64,
}

impl Sample {
    /// Samples the data at the provided point.
    pub fn new(config: Config, fixed: &RTree, moving: &RTree, point: Point) -> Sample {
        let fixed = config.nearest_neighbors(fixed, &point);
        let moving = config.nearest_neighbors(moving, &point);
        Sample {
            x: point.x(),
            y: point.y(),
        }
    }
}
