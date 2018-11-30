use cpd::{rigid::Transform, utils, Rigid, Run, Runner};
use failure::Error;
use nalgebra::{Point3, U3};
use std::time::Duration;
use {Config, Point, RTree};

/// A sample of the glacier's velocity.
#[derive(Debug, Serialize, Deserialize)]
pub struct Sample {
    pub x: f64,
    pub y: f64,
    pub fixed_density: f64,
    pub moving_density: f64,
    pub xmin: f64,
    pub xmax: f64,
    pub ymin: f64,
    pub ymax: f64,
    pub run: Run<U3, Transform<U3>>,
    pub displacement: Point3<f64>,
    pub velocity: Point3<f64>,
}

/// A sample of the glacier's velocity.
#[derive(Debug, Serialize, Deserialize)]
pub struct LowDensitySample {
    pub x: f64,
    pub y: f64,
    pub fixed: f64,
    pub moving: f64,
}

impl Sample {
    /// Samples the data at the provided point.
    pub fn new(
        config: Config,
        fixed: &RTree,
        moving: &RTree,
        point: Point,
        duration: Duration,
    ) -> Result<Sample, Error> {
        let fixed_density = config.density(fixed, &point);
        let moving_density = config.density(moving, &point);
        let fixed = config.nearest_neighbors(fixed, &point);
        let moving = config.nearest_neighbors(moving, &point);
        let mean = utils::mean(&moving);
        let x = mean[0];
        let y = mean[1];
        let mut runner = Runner::new();
        if let Some(max_iterations) = config.max_iterations {
            runner.max_iterations = max_iterations;
        }
        let rigid = Rigid::new();
        let run = runner.run(&rigid, &fixed, &moving)?;
        let displacement = Point3::from(utils::mean(&(&run.points - &moving)));
        let xmin = fixed.column(0).amin().min(moving.column(0).amin());
        let xmax = fixed.column(0).amax().max(moving.column(0).amax());
        let ymin = fixed.column(1).amin().min(moving.column(1).amin());
        let ymax = fixed.column(1).amax().max(moving.column(1).amax());
        let run = run;
        let velocity = displacement * 3600. / duration.as_secs() as f64;
        Ok(Sample {
            x: x,
            y: y,
            fixed_density: fixed_density,
            moving_density: moving_density,
            xmin: xmin,
            xmax: xmax,
            ymin: ymin,
            ymax: ymax,
            run: run,
            displacement: displacement,
            velocity: velocity,
        })
    }
}
