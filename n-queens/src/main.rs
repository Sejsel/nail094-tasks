use std::fs::File;
use itertools::iproduct;
use std::process::Command;
use std::io::Write;

use core::{CnfSat, EvaluationResult, SatModel};

#[derive(Debug)]
enum Solver {
    None { generated_cnfs: usize },
    Kissat,
    Cadical,
    Oxisat,
    Glucose,
    GlucoseSyrup { threads: usize },
}

fn main() -> Result<(), anyhow::Error> {
    let args: Vec<_> = std::env::args().collect();
    let solver = match args.get(1).map(|x| x.as_str()) {
        None => Solver::Kissat,
        Some("cnf") => {
            let generated_cnfs = args.get(2).and_then(|x| x.parse::<usize>().ok()).unwrap_or(1);
            Solver::None { generated_cnfs }
        },
        Some("cadical") => Solver::Cadical,
        Some("oxisat") => Solver::Oxisat,
        Some("glucose") => Solver::Glucose,
        Some("glucose-syrup") => {
            let threads = args.get(2).and_then(|x| x.parse::<usize>().ok()).unwrap_or(1);
            Solver::GlucoseSyrup { threads }
        },
        _ => Solver::Kissat
    };

    eprintln!("Using solver {solver:?}");

    for n in 1.. {
        let mut sat = CnfSat::new();
        add_queen_vars(&mut sat, n);
        add_queen_restrictions(&mut sat, n);

        let solver = match solver {
            Solver::Kissat => Command::new("../solvers/kissat"),
            Solver::Cadical => Command::new("../solvers/cadical"),
            Solver::Oxisat => Command::new("../solvers/oxisat"),
            Solver::Glucose => {
                let mut command = Command::new("../solvers/glucose");
                command.arg("-model");
                command
            },
            Solver::GlucoseSyrup { threads } => {
                let mut command = Command::new("../solvers/glucose-syrup");
                command.arg("-model").arg(format!("-nthreads={}", threads));
                command
            }
            Solver::None { generated_cnfs } => {
                let mut f = File::create(format!("{}.cnf", n)).unwrap();
                write!(f, "{}", sat.to_dimacs()).unwrap();
                if n > generated_cnfs {
                    break;
                } else {
                    continue;
                }
            }
        };

        println!(
            "Starting {n}, {} vars, {} clauses",
            sat.variable_count(),
            sat.clause_count()
        );

        let result = sat.evaluate(solver);
        match result {
            EvaluationResult::Sat { model, time, .. } => {
                println!("Finished {n} in {time:?}, model:");
                println!("{}", queen_map_from_model(&model, n));
            }
            EvaluationResult::Unsat { time, .. } => {
                println!("Finished {n} in {time:?}, UNSAT");
            }
        }
    }

    Ok(())
}

fn queen_pos(queen: usize, x: usize, y: usize) -> String {
    format!("queen_x{}_y{}_q{}", x, y, queen)
}

fn add_queen_vars(sat: &mut CnfSat, n: usize) {
    for queen in 0..n {
        for y in 0..n {
            for x in 0..n {
                sat.create_variable(&queen_pos(queen, x, y));
            }
        }
    }
}

fn add_queen_restrictions(sat: &mut CnfSat, n: usize) {
    // Each queen is on exactly one position
    for queen in 0..n {
        let vars: Vec<_> = iproduct!(0..n, 0..n)
            .map(|(x, y)| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();
        sat.ensure_at_least_one_set(&vars);
        sat.ensure_max_one_set(&vars);
    }

    // No row has two queens
    for y in 0..n {
        let vars: Vec<_> = iproduct!(0..n, 0..n)
            .map(|(x, queen)| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();
        sat.ensure_max_one_set(&vars);
    }

    // No column has two queens
    for x in 0..n {
        let vars: Vec<_> = iproduct!(0..n, 0..n)
            .map(|(y, queen)| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();
        sat.ensure_max_one_set(&vars);
    }

    // \ Diagonals starting at x=0
    for y_start in 0..n {
        let x_range = 0..n - y_start;
        let y_range = y_start..n;

        let vars: Vec<_> = iproduct!(0..n, x_range.zip(y_range))
            .map(|(queen, (x, y))| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();

        sat.ensure_max_one_set(&vars);
    }

    // \ Diagonals starting at y=0, skipping x=0 (included in previous loop)
    for x_start in 1..n {
        let x_range = x_start..n;
        let y_range = 0..n - x_start;

        let vars: Vec<_> = iproduct!(0..n, x_range.zip(y_range))
            .map(|(queen, (x, y))| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();
        sat.ensure_max_one_set(&vars);
    }

    // / Diagonals starting at x=0
    for y_start in 0..n {
        let y_range = (0..=y_start).rev();
        let x_range = 0..=y_start;

        let vars: Vec<_> = iproduct!(0..n, x_range.zip(y_range))
            .map(|(queen, (x, y))| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();
        sat.ensure_max_one_set(&vars);
    }

    // / Diagonals starting at y=n-1, skipping x=0
    for x_start in 1..n {
        let x_range = x_start..n;
        let y_range = (x_start..n).rev();

        let vars: Vec<_> = iproduct!(0..n, x_range.zip(y_range))
            .map(|(queen, (x, y))| sat.get_variable(&queen_pos(queen, x, y)))
            .collect();
        sat.ensure_max_one_set(&vars);
    }
}

fn queen_map_from_model(model: &SatModel, n: usize) -> String {
    let mut output = String::new();
    for y in 0..n {
        for x in 0..n {
            let mut queen_placed = false;

            for queen in 0..n {
                let result = model.get_result_by_name(&queen_pos(queen, x, y)).unwrap();

                if result {
                    output.push('Q');
                    queen_placed = true;
                }
            }

            if !queen_placed {
                output.push('.');
            }
        }
        output.push('\n');
    }

    output
}
