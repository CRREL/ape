use chrono::{DateTime, TimeZone, Utc};
use failure::Error;
use std::path::Path;

#[derive(Debug, Fail)]
#[fail(display = "No file stem for {}", path)]
pub struct MissingFileStem {
    path: String,
}

pub fn riegl_datetime_from_path<P: AsRef<Path>>(path: P) -> Result<DateTime<Utc>, Error> {
    let file_stem = path.as_ref().file_stem().ok_or(MissingFileStem {
        path: path.as_ref().to_path_buf().display().to_string(),
    })?;
    let datetime = Utc.datetime_from_str(
        file_stem.to_string_lossy().as_ref(),
        "%y%m%d_%H%M%S",
    )?;
    Ok(datetime)
}
