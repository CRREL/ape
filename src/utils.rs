use {Error, Result};
use chrono::{DateTime, TimeZone, Utc};
use std::path::Path;

pub fn riegl_datetime_from_path<P: AsRef<Path>>(path: P) -> Result<DateTime<Utc>> {
    let file_stem = path.as_ref().file_stem().ok_or(Error::MissingFileStem)?;
    Utc.datetime_from_str(file_stem.to_string_lossy().as_ref(), "%y%m%d_%H%M%S")
        .map_err(Error::from)
}
