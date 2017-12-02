use Vector;
use chrono::{DateTime, Duration, Utc};
use cpd::Rigid;
use failure::Error;
use las::Point;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// The velocity calculation did not converge.
#[derive(Debug, Fail)]
#[fail(display = "Did not converge")]
pub struct DidNotConverge {}

/// Calculate velocities over a large area using rigid cpd.
#[derive(Debug)]
pub struct Builder {
    after: Vec<Point>,
    before: Vec<Point>,
    datetime: DateTime<Utc>,
    duration: Duration,
    grid_size: i64,
    min_points: usize,
    ngrow: usize,
}

/// A grid of cells, used to calculate velocities.
#[derive(Debug)]
pub struct Grid {
    data: HashMap<(i64, i64), Cell>,
    datetime: DateTime<Utc>,
    duration: Duration,
}

/// A cell in a grid.
#[derive(Debug)]
pub struct Cell {
    after: Vec<Point>,
    before: Vec<Point>,
    coordinates: (i64, i64),
    grid_size: i64,
}

/// A velocity measurement.
#[derive(Debug, Serialize, Deserialize)]
pub struct Velocity {
    /// The number of points in the after matrix.
    pub after_points: usize,

    /// The number of points in the before matrix.
    pub before_points: usize,

    /// The center of gravity of the point.
    pub center_of_gravity: Vector,

    /// The date and time of this velocity measurement.
    pub datetime: DateTime<Utc>,

    /// The size of the grid of the cell used for this velocity.
    pub grid_size: i64,

    /// The number of iterations it took.
    pub iterations: usize,

    /// The mean displacement between the points, divided by the number of hours in between scans.
    pub velocity: Vector,

    /// The lower-left corner of the velocity cell, x.
    pub x: i64,

    /// The lower-left corner of the velocity cell, y.
    pub y: i64,
}

struct Worker {
    id: usize,
}

impl Builder {
    /// Create new velocities from two input las files.
    pub fn new<P: AsRef<Path>, Q: AsRef<Path>>(
        before: P,
        after: Q,
        grid_size: i64,
    ) -> Result<Builder, Error> {
        use las::Reader;

        let before_datetime = super::datetime_from_path(&before)?;
        let after_datetime = super::datetime_from_path(&after)?;
        let duration = after_datetime.signed_duration_since(before_datetime);
        let datetime = before_datetime + duration;
        let before = Reader::from_path(before)?
            .points()
            .collect::<Result<Vec<_>, _>>()?;
        let after = Reader::from_path(after)?
            .points()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Builder {
            after: after,
            before: before,
            datetime: datetime,
            duration: duration,
            grid_size: grid_size,
            min_points: 0,
            ngrow: 0,
        })
    }

    /// Sets the number of times this grid should grow.
    pub fn ngrow(mut self, ngrow: usize) -> Builder {
        self.ngrow = ngrow;
        self
    }

    /// Sets the minimum number of points allowed in a cell.
    pub fn min_points(mut self, min_points: usize) -> Builder {
        self.min_points = min_points;
        self
    }

    /// Creates a grid from this builder.
    pub fn into_grid(self) -> Grid {
        let mut data: HashMap<(i64, i64), Cell> = HashMap::new();
        let grid_size = self.grid_size;
        let min_points = self.min_points;
        let grid_coordinates =
            |point: &Point| (point.y as i64 / grid_size, point.x as i64 / grid_size);
        for point in self.before {
            let coordinates = grid_coordinates(&point);
            data.entry(coordinates)
                .or_insert_with(|| Cell::new(coordinates, grid_size))
                .before
                .push(point);
        }
        for point in self.after {
            let coordinates = grid_coordinates(&point);
            data.entry(coordinates)
                .or_insert_with(|| Cell::new(coordinates, grid_size))
                .after
                .push(point);
        }
        let mut grid = Grid {
            data: data,
            datetime: self.datetime,
            duration: self.duration,
        };
        for _ in 0..self.ngrow {
            info!("Growing cells");
            let n = grid.grow(min_points);
            info!("{} cells grown", n);
        }
        let before = grid.data.len();
        grid.cull(min_points);
        let after = grid.data.len();
        info!("{} cells accepted, {} cells culled", after, before - after);
        grid
    }
}

impl Grid {
    /// Grow any under-populated grid cells.
    pub fn grow(&mut self, min_points: usize) -> usize {
        let mut count = 0;
        for coordinate in self.coordinates() {
            if self.cell(coordinate)
                .map(|c| c.is_too_small(min_points))
                .unwrap_or(false)
            {
                self.grow_cell(coordinate);
                count += 1;
            }
        }
        count
    }

    /// Calculates velocities for each cell in this grid.
    pub fn calculate_velocities<T: Into<Option<usize>>>(
        self,
        num_threads: T,
        rigid: Rigid,
    ) -> Result<Vec<Velocity>, Error> {
        use std::thread;

        let num_threads = num_threads.into().unwrap_or(1);
        assert!(num_threads > 0);
        let mut handles = Vec::new();
        let grid = Arc::new(Mutex::new(self));
        for i in 0..num_threads {
            let grid = grid.clone();
            let rigid = rigid.clone();
            let handle = thread::spawn(move || {
                let worker = Worker { id: i };
                worker.start(grid, rigid)
            });
            handles.push(handle);
        }
        let mut velocities = Vec::new();
        for handle in handles {
            let v = handle.join().unwrap();
            velocities.extend(v?);
        }
        Ok(velocities)
    }

    fn cull(&mut self, min_points: usize) {
        self.data.retain(|_, cell| !cell.is_too_small(min_points))
    }

    fn cell(&self, coordinate: (i64, i64)) -> Option<&Cell> {
        self.data.get(&coordinate)
    }

    fn pop(&mut self) -> Option<Cell> {
        if let Some(&key) = self.data.keys().next() {
            self.data.remove(&key)
        } else {
            None
        }
    }

    fn coordinates(&self) -> Vec<(i64, i64)> {
        let mut coordinates = self.data.keys().map(|&k| k).collect::<Vec<_>>();
        coordinates.sort();
        coordinates
    }

    fn grow_cell(&mut self, coordinate: (i64, i64)) {
        let mut cell = self.data.remove(&coordinate).expect(
            "grow_cell called but cell does not exist",
        );
        cell.grid_size *= 2;
        let (r, c) = coordinate;
        for k in [(r + 1, c), (r, c + 1), (r + 1, c + 1)].into_iter() {
            while self.data
                .get(k)
                .map(|c| c.grid_size < cell.grid_size / 2)
                .unwrap_or(false)
            {
                self.grow_cell(*k);
            }
            if let Some(other) = self.data.remove(k) {
                cell.consume(other);
            }
        }
        self.data.insert(coordinate, cell);
    }
}

impl Cell {
    fn new(coordinates: (i64, i64), grid_size: i64) -> Cell {
        Cell {
            after: Vec::new(),
            before: Vec::new(),
            coordinates: coordinates,
            grid_size: grid_size,
        }
    }

    fn is_too_small(&self, min_points: usize) -> bool {
        self.before.len() < min_points || self.after.len() < min_points
    }

    fn consume(&mut self, other: Cell) {
        assert_eq!(self.grid_size, other.grid_size * 2);
        self.before.extend(other.before);
        self.after.extend(other.after);
    }

    fn calculate_velocity(
        &self,
        rigid: &Rigid,
        datetime: DateTime<Utc>,
        duration: Duration,
    ) -> Result<Velocity, Error> {
        let before = super::matrix_from_points(&self.before);
        let after = super::matrix_from_points(&self.after);
        let run = rigid.register(&after, &before)?;
        let displacement = run.moved - &before;
        if run.converged {
            Ok(Velocity {
                after_points: after.nrows(),
                before_points: before.nrows(),
                center_of_gravity: super::center_of_gravity(&before),
                datetime: datetime,
                grid_size: self.grid_size,
                iterations: run.iterations,
                x: self.coordinates.0 * self.grid_size,
                y: self.coordinates.1 * self.grid_size,
                velocity: super::center_of_gravity(&displacement) / duration.num_hours() as f64,
            })
        } else {
            Err(DidNotConverge {}.into())
        }
    }
}

impl Worker {
    fn start(&self, grid: Arc<Mutex<Grid>>, rigid: Rigid) -> Result<Vec<Velocity>, Error> {
        let (datetime, duration) = {
            let grid = grid.lock().unwrap();
            (grid.datetime, grid.duration)
        };
        let mut velocities = Vec::new();
        while let Some(cell) = {
            let mut grid = grid.lock().unwrap();
            grid.pop()
        }
        {
            info!(
                "#{}: Got cell ({}, {}), size: {}, before: {}, after: {}",
                self.id,
                cell.coordinates.0,
                cell.coordinates.1,
                cell.grid_size,
                cell.before.len(),
                cell.after.len(),
            );
            velocities.push(cell.calculate_velocity(&rigid, datetime, duration)?);
        }
        info!("#{} is done", self.id);
        Ok(velocities)
    }
}
