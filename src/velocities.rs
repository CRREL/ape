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
    let before = Grid::from_path(&path)?;
    let after_path = moving_path(&path)?;
    let after = Grid::from_path(after_path)?;
    let rigid = Runner::new()
        .normalize(Normalize::SameScale)
        .sigma2(SIGMA2)
        .rigid()
        .scale(false);
    let mut args = Vec::new();
    for (&(r, c), before) in &before.map {
        if let Some(after) = after.map.get(&(r, c)) {
            let before = points_to_matrix(before);
            let before_cog_z = cog_z(&before);
            let after = points_to_matrix(after);
            let after_cog_z = cog_z(&after);
            if before.nrows() >= MIN_POINTS && before.nrows() <= MAX_POINTS &&
                after.nrows() >= MIN_POINTS && after.nrows() <= MAX_POINTS &&
                before_cog_z > MIN_COG_HEIGHT && after_cog_z > MIN_COG_HEIGHT
            {
                args.push(Arg {
                    r: r,
                    c: c,
                    before: before,
                    after: after,
                });
            }
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

fn cog_z(matrix: &Matrix<U3>) -> f64 {
    matrix.column(2).iter().sum::<f64>() / matrix.nrows() as f64
}

struct Arg {
    r: i64,
    c: i64,
    before: Matrix<U3>,
    after: Matrix<U3>,
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
                    arg.before.nrows(),
                    arg.after.nrows(),
                    args.len()
                );
                arg
            } else {
                break;
            }
        };
        let run = rigid.register(&arg.after, &arg.before)?;
        if run.converged {
            let point = center_of_gravity(&arg.before);
            let displacement = (0..3)
                .map(|d| {
                    (run.moved.column(d) - arg.before.column(d))
                        .iter()
                        .sum::<f64>() / arg.before.nrows() as f64
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
                before_points: arg.before.nrows(),
                after_points: arg.after.nrows(),
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

struct Grid {
    map: HashMap<(i64, i64), Vec<Point>>,
}

impl NoMovingPath {
    fn new<P: AsRef<Path>>(path: P) -> NoMovingPath {
        NoMovingPath(path.as_ref().display().to_string())
    }
}

impl Grid {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Grid, Error> {
        let mut map = HashMap::new();
        for point in Reader::from_path(path)?.points() {
            let point = point?;
            let c = point.x as i64 / GRID_SIZE;
            let r = point.y as i64 / GRID_SIZE;
            map.entry((r, c)).or_insert_with(Vec::new).push(point);
        }
        Ok(Grid { map: map })
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

fn points_to_matrix(points: &Vec<Point>) -> Matrix<U3> {
    let mut matrix = Matrix::<U3>::zeros(points.len());
    for (i, point) in points.iter().enumerate() {
        matrix[(i, 0)] = point.x;
        matrix[(i, 1)] = point.y;
        matrix[(i, 2)] = point.z;
    }
    matrix
}

fn center_of_gravity(matrix: &Matrix<U3>) -> Point3<f64> {
    use cpd::Vector;
    let mut point = Vector::<U3>::zeros();
    for d in 0..3 {
        point[d] = matrix.column(d).iter().sum::<f64>() / matrix.nrows() as f64;
    }
    Point3::from_coordinates(point)
}
