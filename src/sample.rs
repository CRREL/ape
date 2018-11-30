use cpd::{rigid::Transform, utils, Rigid, Run, Runner};
use failure::Error;
use nalgebra::{Point3, U3};
use std::time::Duration;
use {Config, Point, RTree};

/// A sample of the glacier's velocity.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Sample {
    x: f64,
    y: f64,
    fixed_density: f64,
    moving_density: f64,
    cpd: Option<Cpd>,
}

/// A CPD run.
#[derive(Debug, Serialize, Deserialize)]
pub struct Cpd {
    xmin: f64,
    xmax: f64,
    ymin: f64,
    ymax: f64,
    run: Run<U3, Transform<U3>>,
    displacement: Point3<f64>,
    velocity: Point3<f64>,
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
        let mut sample = Sample {
            x: point.x(),
            y: point.y(),
            ..Default::default()
        };
        sample.fixed_density = config.density(fixed, &point);
        sample.moving_density = config.density(moving, &point);
        if sample.fixed_density < config.min_density || sample.moving_density < config.min_density {
            return Ok(sample);
        }
        let fixed = config.nearest_neighbors(fixed, &point);
        let moving = config.nearest_neighbors(moving, &point);
        let mut runner = Runner::new();
        if let Some(max_iterations) = config.max_iterations {
            runner.max_iterations = max_iterations;
        }
        let rigid = Rigid::new();
        let run = runner.run(&rigid, &fixed, &moving)?;
        let displacement = Point3::from(utils::mean(&(&run.points - &moving)));
        sample.cpd = Some(Cpd {
            xmin: fixed.column(0).amin().min(moving.column(0).amin()),
            xmax: fixed.column(0).amax().max(moving.column(0).amax()),
            ymin: fixed.column(1).amin().min(moving.column(1).amin()),
            ymax: fixed.column(1).amax().max(moving.column(1).amax()),
            run: run,
            displacement: displacement,
            velocity: displacement * 3600. / duration.as_secs() as f64,
        });
        Ok(sample)
    }
}
