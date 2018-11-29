use pbr::ProgressBar;
use std::io::Write;
use {Config, Point, RTree};

/// A sample of the glacier's velocity.
#[derive(Debug, Serialize)]
pub struct Sample {
    x: f64,
    y: f64,
}

impl Sample {
    /// Samples the data at the provided point.
    pub fn new<W: Write>(
        config: Config,
        fixed: &RTree,
        moving: &RTree,
        point: Point,
        progress_bar: &mut ProgressBar<W>,
    ) -> Sample {
        Sample {
            x: point.x(),
            y: point.y(),
        }
    }
}
