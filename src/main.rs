extern crate ape;
#[macro_use]
extern crate clap;
extern crate cpd;
extern crate env_logger;
extern crate serde_json;

use ape::velocities;
use clap::App;
use cpd::{Normalize, Runner};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};

const FORMAT_STR: &'static str = "%y%m%d_%H%M%S";

fn main() {
    env_logger::init().unwrap();
    let yaml = load_yaml!("cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    if let Some(matches) = matches.subcommand_matches("sop") {
        let sop = ape::matrix_from_path(matches.value_of("SOP").unwrap()).unwrap();
        let adjustment = ape::matrix_from_path(matches.value_of("ADJUSTMENT").unwrap()).unwrap();
        let sop = adjustment * sop;
        println!("{}", ape::string_from_matrix(sop.matrix()));
    } else if let Some(matches) = matches.subcommand_matches("datetime") {
        let infile = matches.value_of("INFILE").unwrap();
        println!("{}", ape::datetime_from_path(infile).unwrap());
    } else if let Some(matches) = matches.subcommand_matches("pairs") {
        let infile = matches.value_of("INFILE").unwrap();
        let interval = matches
            .value_of("INTERVAL")
            .unwrap()
            .parse::<f64>()
            .unwrap();
        let buffer = matches
            .value_of("buffer")
            .unwrap_or("1")
            .parse::<f64>()
            .unwrap();
        let datetimes = BufReader::new(File::open(infile).unwrap())
            .lines()
            .map(|l| ape::datetime_from_path(l.unwrap()).unwrap())
            .collect::<Vec<_>>();
        for &datetime in datetimes.iter() {
            if let Some(other) = datetimes.iter().find(|other| {
                let duration = other.signed_duration_since(datetime).num_minutes() as f64 / 60. -
                    interval;
                duration.abs() < buffer
            })
            {
                println!(
                    "{} {}",
                    datetime.format(FORMAT_STR),
                    other.format(FORMAT_STR)
                );
            }
        }
    } else if let Some(matches) = matches.subcommand_matches("magic-bucket-config") {
        let sop = ape::matrix_from_path(matches.value_of("SOP").unwrap()).unwrap();
        let adjustment = ape::matrix_from_path(matches.value_of("ADJUSTMENT").unwrap()).unwrap();
        let pop = ape::matrix_from_path(matches.value_of("POP").unwrap()).unwrap();
        println!(
            "{}",
            serde_json::to_string_pretty(&ape::magic_bucket_config(&sop, &adjustment, &pop))
                .unwrap()
        );
    } else if let Some(matches) = matches.subcommand_matches("cpd") {
        let rigid = Runner::new()
            .sigma2(value_t!(matches, "sigma2", f64).ok())
            .normalize(Normalize::SameScale)
            .rigid()
            .scale(false);
        if let Some(matches) = matches.subcommand_matches("simple") {
            let fixed = ape::matrix_from_las_path(matches.value_of("FIXED").unwrap()).unwrap();
            let moving = ape::matrix_from_las_path(matches.value_of("MOVING").unwrap()).unwrap();
            let run = rigid.register(&fixed, &moving).unwrap();
            if run.converged {
                let transform3 = run.transform.as_transform3();
                println!("{}", ape::string_from_matrix(transform3.matrix()));
            } else {
                panic!("Run did not converge");
            }
        } else if let Some(matches) = matches.subcommand_matches("velocities") {
            if let Some(matches) = matches.subcommand_matches("create") {
                let before = matches.value_of("BEFORE").unwrap();
                let after = matches.value_of("AFTER").unwrap();
                let grid_size = value_t!(matches, "grid-size", i64).unwrap_or(100);
                let grid = velocities::Builder::new(before, after, grid_size)
                    .unwrap()
                    .min_points(value_t!(matches, "min-points", usize).unwrap_or(250))
                    .ngrow(value_t!(matches, "ngrow", usize).unwrap_or(1))
                    .into_grid();
                let mut file = File::create(matches.value_of("OUTFILE").unwrap()).unwrap();
                let velocities =
                    grid.calculate_velocities(value_t!(matches, "threads", usize).ok(), rigid)
                        .into_iter()
                        .filter_map(|v| v.ok())
                        .collect::<Vec<_>>();
                let string = serde_json::to_string(&velocities).unwrap();
                write!(file, "{}", string).unwrap();
            } else if let Some(matches) = matches.subcommand_matches("to-csv") {
                let infile = File::open(matches.value_of("INFILE").unwrap()).unwrap();
                let velocities: Vec<velocities::Velocity> = serde_json::from_reader(infile)
                    .unwrap();
                let max_iterations = matches.value_of("max-iterations").map(|s| {
                    s.parse::<usize>().unwrap()
                });
                let max_velocity = matches.value_of("max-velocity").map(|s| {
                    s.parse::<f64>().unwrap()
                });
                let min_height = matches.value_of("min-height").map(
                    |s| s.parse::<f64>().unwrap(),
                );
                println!("x,y,z,grid_size,iterations,vx,vy,vz,vxy,v");
                for velocity in velocities {
                    if max_iterations.map(|m| velocity.iterations < m).unwrap_or(
                        true,
                    ) &&
                        max_velocity
                            .map(|m| velocity.velocity.magnitude() < m)
                            .unwrap_or(true) &&
                        min_height
                            .map(|m| velocity.center_of_gravity.z > m)
                            .unwrap_or(true)
                    {
                        println!("{},{},{},{},{},{},{},{},{},{}",
                                 velocity.center_of_gravity.x,
                                 velocity.center_of_gravity.y,
                                 velocity.center_of_gravity.z,
                                 velocity.grid_size,
                                 velocity.iterations,
                                 velocity.velocity.x,
                                 velocity.velocity.y,
                                 velocity.velocity.z,
                                 velocity.velocity.xy(),
                                 velocity.velocity.magnitude(),
                                 );
                    }
                }
            } else if let Some(matches) = matches.subcommand_matches("line") {
                use std::collections::HashMap;
                use std::fs;
                use ape::Vector;

                let directory = matches.value_of("DIRECTORY").unwrap();
                let northing = matches
                    .value_of("NORTHING")
                    .unwrap()
                    .parse::<f64>()
                    .unwrap();
                let mut map = HashMap::new();
                for (velocity, datetime) in
                    fs::read_dir(directory)
                        .unwrap()
                        .filter_map(|r| {
                            r.ok().and_then(|dir_entry| if dir_entry
                                .path()
                                .extension()
                                .map(|e| e == "json")
                                .unwrap_or(false)
                            {
                                serde_json::from_reader::<_, Vec<velocities::Velocity>>(
                                    File::open(dir_entry.path()).unwrap(),
                                ).ok()
                                    .map(
                                        |v| (v, ape::datetime_from_path(dir_entry.path()).unwrap()),
                                    )
                            } else {
                                None
                            })
                        })
                        .flat_map(|(v, datetime)| {
                            v.into_iter().filter(|v| v.y == northing).map(move |v| {
                                (v, datetime)
                            })
                        })
                {
                    let entry = map.entry(velocity.x as i64).or_insert_with(Vec::new);
                    entry.push((velocity, datetime));
                }
                println!("datetime,x,vx,vy,vz,vxy,v,dvx,dvy,dvz,dvxy,dv");
                for (_, cell) in map {
                    let mean = Vector::mean(&cell.iter().map(|&(ref v, _)| v.velocity).collect());
                    for (velocity, datetime) in cell {
                        println!("{},{},{},{},{},{},{},{},{},{},{},{}",
                                 datetime,
                                 velocity.x,
                                 velocity.velocity.x,
                                 velocity.velocity.y,
                                 velocity.velocity.z,
                                 velocity.velocity.xy(),
                                 velocity.velocity.magnitude(),
                                 velocity.velocity.x - mean.x,
                                 velocity.velocity.y - mean.y,
                                 velocity.velocity.z - mean.z,
                                 velocity.velocity.xy() - mean.xy(),
                                 velocity.velocity.magnitude() - mean.magnitude(),
                                 )
                    }

                }
            }
        }
    } else {
        panic!("Invalid command");
    }
}
