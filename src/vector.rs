use std::iter::FromIterator;
use std::ops::Div;

/// A three-dimensional vector.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vector {
    pub fn xy(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

impl Div<f64> for Vector {
    type Output = Vector;

    fn div(self, other: f64) -> Vector {
        Vector {
            x: self.x / other,
            y: self.y / other,
            z: self.z / other,
        }
    }
}

impl FromIterator<f64> for Vector {
    fn from_iter<T>(iter: T) -> Vector
    where
        T: IntoIterator<Item = f64>,
    {
        let mut iter = iter.into_iter();
        let vector = Vector {
            x: iter.next().unwrap(),
            y: iter.next().unwrap(),
            z: iter.next().unwrap(),
        };
        assert_eq!(None, iter.next());
        vector
    }
}
