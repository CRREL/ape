use failure::Error;
use std::path::Path;

pub fn velocities<P: AsRef<Path>>(_path: P) -> Result<Vec<Velocity>, Error> {
    unimplemented!()
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Velocity {}
