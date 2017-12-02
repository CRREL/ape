extern crate chrono;
extern crate cpd;
#[macro_use]
extern crate failure;
extern crate las;
#[macro_use]
extern crate log;
extern crate nalgebra;
#[macro_use]
extern crate serde_derive;

pub mod velocities;
mod vector;

use chrono::{DateTime, Utc};
use failure::Error;
use las::Point;
use nalgebra::{Dynamic, MatrixMN, MatrixN, Projective3, U3, U4};
use std::path::Path;
pub use vector::Vector;

/// An error returned if the dat files doesn't contain 16 entries.
#[derive(Debug, Fail)]
#[fail(display = "Invalid matrix length: {}", _0)]
pub struct InvalidMatrixLen(usize);

/// The path cannot be turned into a datetime.
#[derive(Debug, Fail)]
#[fail(display = "Date and time from path: {}", _0)]
pub struct DateTimeFromPath(String);

/// Reads a .dat file and returns the underlying matrix.
///
/// # Examples
///
/// ```
/// let matrix = ape::matrix_from_path("data/sop.dat").unwrap();
/// assert_eq!(1001.7951549705150000, matrix[(0, 3)]);
/// ```
pub fn matrix_from_path<P: AsRef<Path>>(path: P) -> Result<Projective3<f64>, Error> {
    use nalgebra::{MatrixN, U4};
    use std::fs::File;
    use std::io::Read;

    let mut file = File::open(path)?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    let numbers = string
        .split_whitespace()
        .map(|s| s.parse::<f64>())
        .collect::<Result<Vec<_>, _>>()?;
    if numbers.len() != 16 {
        return Err(InvalidMatrixLen(numbers.len()).into());
    }
    let matrix = MatrixN::<f64, U4>::from_iterator(numbers.into_iter());
    Ok(Projective3::from_matrix_unchecked(matrix.transpose()))
}

/// Returns a matrix from a las path.
pub fn matrix_from_las_path<P: AsRef<Path>>(path: P) -> Result<MatrixMN<f64, Dynamic, U3>, Error> {
    use las::Reader;
    let points = Reader::from_path(path)?
        .points()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(matrix_from_points(&points))
}

/// Creates a dat string from a matrix.
///
/// # Examples
///
/// ```
/// let matrix = ape::matrix_from_path("data/sop.dat").unwrap();
/// let string = ape::string_from_matrix(&matrix);
/// ```
pub fn string_from_matrix(matrix: &MatrixN<f64, U4>) -> String {
    let mut string = String::new();
    for i in 0..4 {
        let row = matrix.row(i);
        string.push_str(&format!("{} {} {} {}\n", row[0], row[1], row[2], row[3]));
    }
    string
}

/// Turns las points into a matrix.
pub fn matrix_from_points(points: &Vec<Point>) -> MatrixMN<f64, Dynamic, U3> {
    let mut matrix = MatrixMN::<f64, Dynamic, U3>::zeros(points.len());
    for (i, point) in points.iter().enumerate() {
        matrix[(i, 0)] = point.x;
        matrix[(i, 1)] = point.y;
        matrix[(i, 2)] = point.z;
    }
    matrix
}

/// Returns the center of gravity of this matrix as a vector.
pub fn center_of_gravity(matrix: &MatrixMN<f64, Dynamic, U3>) -> Vector {
    (0..3)
        .map(|d| {
            matrix.column(d).iter().sum::<f64>() / matrix.nrows() as f64
        })
        .collect()
}

/// Calculates a date time from a path.
pub fn datetime_from_path<P: AsRef<Path>>(path: P) -> Result<DateTime<Utc>, Error> {
    use chrono::TimeZone;

    if let Some(file_stem) = path.as_ref().file_stem().map(|s| s.to_string_lossy()) {
        Utc.datetime_from_str(&file_stem[0..13], "%y%m%d_%H%M%S")
            .map_err(Error::from)
    } else {
        Err(DateTimeFromPath(path.as_ref().display().to_string()).into())
    }
}
