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
use std::io::Write;

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
                        .unwrap();
                let string = serde_json::to_string(&velocities).unwrap();
                write!(file, "{}", string).unwrap();
            } else if let Some(matches) = matches.subcommand_matches("to-csv") {
                let infile = File::open(matches.value_of("INFILE").unwrap()).unwrap();
                let velocities: Vec<velocities::Velocity> = serde_json::from_reader(infile)
                    .unwrap();
                println!("x,y,z,grid_size,iterations,vx,vy,vz,vxy,v");
                for velocity in velocities {
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
        }
    }
}
