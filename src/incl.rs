use {Error, Result};
use bincode::{self, Infinite};
use std::fs::File;
use std::path::Path;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Inclination {
    time: f64,
    roll: f32,
    pitch: f32,
}

#[derive(Debug, Default, Serialize)]
pub struct Stats {
    pub roll: Metrics,
    pub pitch: Metrics,
}

#[derive(Debug, Default, Serialize)]
pub struct Metrics {
    pub mean: f32,
    pub stddev: f32,
}

impl Inclination {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Vec<Inclination>> {
        let mut file = File::open(path)?;
        bincode::deserialize_from(&mut file, Infinite).map_err(Error::from)
    }
}

impl Stats {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Stats> {
        let inclinations = Inclination::from_path(path)?;
        Ok(Stats {
            roll: Metrics::new(inclinations.iter().map(|i| i.roll)),
            pitch: Metrics::new(inclinations.iter().map(|i| i.pitch)),
        })
    }
}

impl Metrics {
    fn new<I: IntoIterator<Item = f32>>(iter: I) -> Metrics {
        let mut sum = 0f64;
        let mut sum2 = 0f64;
        let mut count = 0.;
        for n in iter.into_iter() {
            sum += n as f64;
            sum2 += (n as f64).powi(2);
            count += 1.;
        }
        Metrics {
            mean: (sum / count) as f32,
            stddev: (sum2 / count).sqrt() as f32,
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::Inclination;
    use bincode;
    use scanlib;
    use std::path::Path;

    impl From<scanlib::Inclination> for Inclination {
        fn from(i: scanlib::Inclination) -> Inclination {
            Inclination {
                time: i.time,
                roll: i.roll as f32,
                pitch: i.pitch as f32,
            }
        }
    }

    fn read_inclinations<P: AsRef<Path>>(path: P, sync_to_pps: bool) -> Vec<Inclination> {
        scanlib::inclinations_from_path(path, sync_to_pps)
            .unwrap()
            .into_iter()
            .map(|i| i.into())
            .collect()
    }

    fn write_inclinations<P: AsRef<Path>>(inclinations: Vec<Inclination>, path: P) {
        use std::fs::File;
        use bincode::Infinite;

        let mut write = File::create(path).unwrap();
        bincode::serialize_into(&mut write, &inclinations, Infinite).unwrap();
    }
}
