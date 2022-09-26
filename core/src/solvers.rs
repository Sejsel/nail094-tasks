use std::process::Command;

#[derive(Debug)]
pub enum Solver {
    Kissat,
    Cadical,
    Oxisat,
    OxisatDpll,
    Glucose,
    GlucoseSyrup { threads: usize },
}

pub fn parse_solver(args: Vec<String>) -> Solver {
    match args.get(1).map(|x| x.as_str()) {
        None => Solver::Kissat,
        Some("cadical") => Solver::Cadical,
        Some("oxisat") => Solver::Oxisat,
        Some("oxisat-dpll") => Solver::OxisatDpll,
        Some("glucose") => Solver::Glucose,
        Some("glucose-syrup") => {
            let threads = args.get(2).and_then(|x| x.parse::<usize>().ok()).unwrap_or(1);
            Solver::GlucoseSyrup { threads }
        },
        _ => Solver::Kissat
    }
}

pub fn build_command(solver: &Solver) -> Command {
    match solver {
        Solver::Kissat => Command::new("../solvers/kissat"),
        Solver::Cadical => Command::new("../solvers/cadical"),
        Solver::Oxisat => {
            let mut command = Command::new("../solvers/oxisat");
            command.arg("cdcl");
            command
        }
        Solver::OxisatDpll => {
            let mut command = Command::new("../solvers/oxisat");
            command.arg("dpll");
            command
        }
        Solver::Glucose => {
            let mut command = Command::new("../solvers/glucose");
            command.arg("-model");
            command
        }
        Solver::GlucoseSyrup { threads } => {
            let mut command = Command::new("../solvers/glucose-syrup");
            command.arg("-model").arg(format!("-nthreads={}", threads));
            command
        }
    }
}