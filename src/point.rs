use nalgebra::Point3;
use spade::{PointN, TwoDimensional};

/// Our custom point type.
///
/// We want to do our spatial searches in 2D but we want to run CPD on 3D points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point(Point3<f64>);

impl Point {
    /// Creates a new point.
    ///
    /// # Examples
    ///
    /// ```
    /// use ape::Point;
    /// let point = Point::new(1., 2., 3.);
    /// ```
    pub fn new(x: f64, y: f64, z: f64) -> Point {
        Point(Point3::new(x, y, z))
    }
}

impl PointN for Point {
    type Scalar = f64;

    fn dimensions() -> usize {
        2
    }

    fn from_value(value: Self::Scalar) -> Self {
        Point(Point3::from_value(value))
    }

    fn nth(&self, index: usize) -> &Self::Scalar {
        self.0.nth(index)
    }

    fn nth_mut(&mut self, index: usize) -> &mut Self::Scalar {
        self.0.nth_mut(index)
    }
}

impl TwoDimensional for Point {}
