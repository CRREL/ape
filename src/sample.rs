use cpd::rigid::Transform;
use failure::Error;
use nalgebra::U3;
use std::f64::consts::PI;
use {Config, Point, RTree};

/// A sample of the glacier's velocity.
#[derive(Debug, Serialize)]
pub enum Sample {
    NoPoints {
        x: f64,
        y: f64,
        fixed: usize,
        moving: usize,
    },
    DensityTooLow {
        x: f64,
        y: f64,
        fixed: f64,
        moving: f64,
    },
    Complete {
        x: f64,
        y: f64,
    },
}

impl Sample {
    /// Samples the data at the provided point.
    pub fn new(
        config: Config,
        fixed: &RTree,
        moving: &RTree,
        point: Point,
    ) -> Result<Sample, Error> {
        let radius2 = config.step as f64 * config.step as f64;
        let fixed_in_circle = fixed.lookup_in_circle(&point, &radius2).len();
        let moving_in_circle = fixed.lookup_in_circle(&point, &radius2).len();
        if fixed_in_circle == 0 || moving_in_circle == 0 {
            return Ok(Sample::NoPoints {
                x: point.x(),
                y: point.y(),
                fixed: fixed_in_circle,
                moving: moving_in_circle,
            });
        }
        let area = PI * radius2;
        let fixed_density = fixed_in_circle as f64 / area;
        let moving_density = moving_in_circle as f64 / area;
        if fixed_density < config.min_density || moving_density < config.min_density {
            return Ok(Sample::DensityTooLow {
                x: point.x(),
                y: point.y(),
                fixed: fixed_density,
                moving: moving_density,
            });
        }
        let fixed = config.nearest_neighbors(fixed, &point);
        let moving = config.nearest_neighbors(moving, &point);
        //let run = cpd::rigid(&fixed, &moving)?;
        Ok(Sample::Complete {
            x: point.x(),
            y: point.y(),
        })
    }

    pub fn has_no_points(&self) -> bool {
        match *self {
            Sample::NoPoints { .. } => true,
            _ => false,
        }
    }
}
