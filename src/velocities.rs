use chrono::{DateTime, Duration, Utc};
use cpd::{Matrix, Normalize, Rigid, Runner, U3};
use failure::Error;
use las::{Point, Reader};
use nalgebra::Point3;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

const GRID_SIZE: i64 = 100;
const INTERVAL: f64 = 6.;
const MIN_COG_HEIGHT: f64 = 40.;
const THREADS: usize = 7;
const MIN_POINTS: usize = 1000;
const MAX_POINTS: usize = 10000;
const SIGMA2: f64 = 0.01;

#[derive(Debug, Fail)]
#[fail(display = "No moving path for path: {}", _0)]
struct NoMovingPath(String);

pub fn velocities<P: AsRef<Path>>(path: P) -> Result<Vec<Velocity>, Error> {
    let mut before = Grid::from_path(&path)?;
    let after_path = moving_path(&path)?;
    let mut after = Grid::from_path(after_path)?;
    let rigid = Runner::new()
        .normalize(Normalize::SameScale)
        .sigma2(SIGMA2)
        .rigid()
        .scale(false);
    let mut args = Vec::new();
    before.grow_cells(&mut after);
    for ((r, c), before) in before.map {
        if let Some(after) = after.map.remove(&(r, c)) {
            args.push(Arg {
                r: r,
                c: c,
                before: before,
                after: after,
            });
        }
    }
    let args = Arc::new(Mutex::new(args));
    let mut handles = Vec::new();
    for i in 0..THREADS {
        let args = args.clone();
        let rigid = rigid.clone();
        let path = path.as_ref().to_path_buf();
        let handle = thread::spawn(move || worker(i, rigid, path, args));
        handles.push(handle);
    }
    let mut velocities = Vec::new();
    for handle in handles {
        velocities.extend(handle.join().unwrap()?);
    }
    Ok(velocities)

}

struct Arg {
    r: i64,
    c: i64,
    before: Cell,
    after: Cell,
}

fn worker(
    id: usize,
    rigid: Rigid,
    path: PathBuf,
    args: Arc<Mutex<Vec<Arg>>>,
) -> Result<Vec<Velocity>, Error> {
    let mut velocities = Vec::new();
    loop {
        let arg = {
            let mut args = args.lock().unwrap();
            if let Some(arg) = args.pop() {
                println!(
                    "#{}: Running grid cell ({}, {}) with {} before points and {} after points, {} cells remaining",
                    id,
                    arg.r,
                    arg.c,
                    arg.before.len(),
                    arg.after.len(),
                    args.len()
                );
                arg
            } else {
                break;
            }
        };
        let run = rigid.register(&arg.after.matrix(), &arg.before.matrix())?;
        if run.converged {
            let point = arg.before.center_of_gravity();
            let before = arg.before.matrix();
            let displacement = (0..3)
                .map(|d| {
                    (run.moved.column(d) - before.column(d)).iter().sum::<f64>() /
                        before.nrows() as f64
                })
                .collect::<Vec<_>>();
            let displacement = Point3::new(displacement[0], displacement[1], displacement[2]);
            let velocity = displacement / INTERVAL;
            velocities.push(Velocity {
                center_of_gravity: Vector {
                    x: point.coords[0],
                    y: point.coords[1],
                    z: point.coords[2],
                },
                datetime: datetime_from_path(&path)? + Duration::hours(INTERVAL as i64 / 2),
                before_points: arg.before.len(),
                after_points: arg.after.len(),
                iterations: run.iterations,
                velocity: Vector {
                    x: velocity[0],
                    y: velocity[1],
                    z: velocity[2],
                },
                x: (arg.c * GRID_SIZE) as f64,
                y: (arg.r * GRID_SIZE) as f64,
                grid_size: GRID_SIZE,
            });
        }
    }
    println!("Worker #{} is done", id);
    Ok(velocities)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Velocity {
    pub after_points: usize,
    pub before_points: usize,
    pub center_of_gravity: Vector,
    pub datetime: DateTime<Utc>,
    pub grid_size: i64,
    pub iterations: usize,
    pub velocity: Vector,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug)]
struct Cell {
    points: Vec<Point>,
    grid_size: i64,
}

#[derive(Debug)]
struct Grid {
    map: HashMap<(i64, i64), Cell>,
}

impl NoMovingPath {
    fn new<P: AsRef<Path>>(path: P) -> NoMovingPath {
        NoMovingPath(path.as_ref().display().to_string())
    }
}

impl Cell {
    fn new() -> Cell {
        Cell {
            points: Vec::new(),
            grid_size: GRID_SIZE,
        }
    }

    fn len(&self) -> usize {
        self.points.len()
    }

    fn matrix(&self) -> Matrix<U3> {
        let mut matrix = Matrix::<U3>::zeros(self.points.len());
        for (i, point) in self.points.iter().enumerate() {
            matrix[(i, 0)] = point.x;
            matrix[(i, 1)] = point.y;
            matrix[(i, 2)] = point.z;
        }
        matrix
    }

    fn center_of_gravity(&self) -> Point3<f64> {
        use cpd::Vector;
        let mut point = Vector::<U3>::zeros();
        let matrix = self.matrix();
        for d in 0..3 {
            point[d] = matrix.column(d).iter().sum::<f64>() / self.points.len() as f64;
        }
        Point3::from_coordinates(point)
    }

    fn push(&mut self, point: Point) {
        self.points.push(point);
    }
}

impl Grid {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Grid, Error> {
        let mut map = HashMap::new();
        for point in Reader::from_path(path)?.points() {
            let point = point?;
            let c = point.x as i64 / GRID_SIZE;
            let r = point.y as i64 / GRID_SIZE;
            map.entry((r, c)).or_insert_with(Cell::new).push(point);
        }
        map.retain(|_, cell| {
            cell.len() <= MAX_POINTS && cell.center_of_gravity()[2] >= MIN_COG_HEIGHT
        });
        Ok(Grid { map: map })
    }

    fn grow_cells(&mut self, other: &mut Grid) {
        let min_r = self.map.keys().map(|&(r, _)| r).min().unwrap();
        let max_r = self.map.keys().map(|&(r, _)| r).max().unwrap();
        let min_c = self.map.keys().map(|&(_, c)| c).min().unwrap();
        let max_c = self.map.keys().map(|&(_, c)| c).max().unwrap();
        for r in min_r..(max_r + 1) {
            for c in min_c..(max_c + 1) {
                if let Some(npoints) = self.map.get(&(r, c)).map(|v| v.len()) {
                    if npoints < MIN_POINTS {
                        panic!("Need to join");
                    }
                }
            }
        }
        unimplemented!()
    }
}

fn moving_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, Error> {
    let fixed = datetime_from_path(path.as_ref())?;
    let error = NoMovingPath::new(path.as_ref());
    path.as_ref()
        .parent()
        .and_then(|parent| parent.read_dir().ok())
        .and_then(|read_dir| {
            read_dir.filter_map(|r| r.ok()).find(|dir_entry| {
                is_the_moving_path(fixed, dir_entry.path())
            })
        })
        .map(|dir_entry| dir_entry.path().to_path_buf())
        .ok_or(error.into())
}

pub fn datetime_from_path<P: AsRef<Path>>(path: P) -> Result<DateTime<Utc>, Error> {
    use chrono::TimeZone;

    if let Some(file_name) = path.as_ref().file_name().and_then(|f| f.to_str()) {
        let datetime = Utc.datetime_from_str(&file_name[0..13], "%y%m%d_%H%M%S")?;
        Ok(datetime)
    } else {
        Err(NoMovingPath::new(path.as_ref()).into())
    }
}

fn is_the_moving_path<P: AsRef<Path>>(fixed: DateTime<Utc>, path: P) -> bool {
    datetime_from_path(path)
        .map(|moving| {
            let duration = moving.signed_duration_since(fixed);
            duration > Duration::hours(0) && duration < Duration::hours(INTERVAL as i64 + 1)
        })
        .unwrap_or(false)
}
