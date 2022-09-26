use anyhow::anyhow;
use nom::Finish;
use std::io::{stdin, Read};

use crate::dimacs::Dimacs;
use core::solvers::{build_command, parse_solver};
use core::{CnfClause, CnfSat, EvaluationResult};

mod dimacs;

#[derive(Eq, PartialEq, Copy, Clone)]
enum VariableValue {
    None,
    True,
    False,
    Either,
    Backbone(bool),
}

fn main() -> Result<(), anyhow::Error> {
    let args = std::env::args().collect();
    let solver = parse_solver(args);
    eprintln!("Using solver {solver:?}");

    let mut input = String::new();
    stdin().read_to_string(&mut input)?;

    let dimacs = match dimacs::parse(&input).finish() {
        Ok((_, dimacs)) => dimacs,
        Err(err) => {
            return Err(anyhow!(
                "Failed to parse dimacs: {}",
                nom::error::convert_error(input.as_str(), err)
            ));
        }
    };

    let (mut sat, vars) = dimacs_to_sat(dimacs);

    eprintln!(
        "Input CNF: {} vars, {} clauses",
        sat.variable_count(),
        sat.clause_count()
    );

    let mut assignments: Vec<_> = vars.iter().map(|_| VariableValue::None).collect();

    #[derive(Eq, PartialEq, Copy, Clone, Debug)]
    enum State {
        FirstRun,
        Searching {
            candidate_index: usize,
            candidate_value: bool,
        },
    }

    let mut state = State::FirstRun;

    loop {
        if let State::Searching {
            candidate_index,
            candidate_value,
        } = state
        {
            let mut clause = CnfClause::new();
            // Try to add negated literal, if adding it is UNSAT -> this is a backbone.
            clause.set(vars[candidate_index], !candidate_value);
            sat.add_clause(clause);
        }

        let result = sat.evaluate(build_command(&solver));
        match result {
            EvaluationResult::Sat { model, time, .. } => {
                eprintln!("Finished in {time:?}, SAT");
                for (i, &var) in vars.iter().enumerate() {
                    let result_bool = model
                        .get_result_by_id(var)
                        .expect("Var missing in solver output");

                    let result = match result_bool {
                        true => VariableValue::True,
                        false => VariableValue::False,
                    };

                    assignments[i] = match assignments[i] {
                        // For first result, just store the result
                        VariableValue::None => {
                            assert_eq!(state, State::FirstRun);
                            result
                        }
                        // If both values have been seen already, the state is not changed.
                        VariableValue::Either => VariableValue::Either,
                        // If this is a backbone already, keep it in that state.
                        backbone @ VariableValue::Backbone(value) => {
                            assert_eq!(value, result_bool);
                            backbone
                        }
                        // If this result matches all seen values, keep it, otherwise swap to Either.
                        value @ _ => {
                            if result == value {
                                value
                            } else {
                                VariableValue::Either
                            }
                        }
                    }
                }
            }

            EvaluationResult::Unsat { time, .. } => {
                eprintln!("Finished in {time:?}, UNSAT");
                match state {
                    State::FirstRun => {
                        return Err(anyhow!("Unsatisfiable CNF input provided."));
                    }
                    State::Searching {
                        candidate_index,
                        candidate_value,
                    } => {
                        assignments[candidate_index] = VariableValue::Backbone(candidate_value);
                    }
                }
            }
        }

        if let State::Searching { .. } = state {
            // Remove the clause we added for checking the backbone.
            sat.pop_clause();
        }

        state = match state {
            State::FirstRun => match find_backbone_candidate(0, &assignments) {
                None => break,
                Some((index, value)) => State::Searching {
                    candidate_value: value,
                    candidate_index: index,
                },
            },
            State::Searching {
                candidate_index, ..
            } => match find_backbone_candidate(candidate_index + 1, &assignments) {
                None => break,
                Some((index, value)) => State::Searching {
                    candidate_value: value,
                    candidate_index: index,
                },
            },
        }
    }

    let backbones: Vec<_> = assignments
        .iter()
        .enumerate()
        .filter(|(_, x)| matches!(x, VariableValue::Backbone(_)))
        .map(|(i, x)| match x {
            VariableValue::Backbone(true) => format!("{}", i + 1),
            VariableValue::Backbone(false) => format!("-{}", i + 1),
            _ => unreachable!(),
        })
        .collect();

    println!("Found {} backbones:", backbones.len());
    println!("{}", backbones.join(" "));

    Ok(())
}

fn find_backbone_candidate(
    current_index: usize,
    values: &[VariableValue],
) -> Option<(usize, bool)> {
    let next_index = values
        .iter()
        .enumerate()
        .skip(current_index)
        .find(|(_, &x)| x == VariableValue::False || x == VariableValue::True)
        .map(|(i, _)| i)?;

    Some((
        next_index,
        match values[next_index] {
            VariableValue::True => true,
            VariableValue::False => false,
            _ => unreachable!(),
        },
    ))
}

fn dimacs_to_sat(dimacs: Dimacs) -> (CnfSat, Vec<usize>) {
    let mut cnf = CnfSat::new();

    let vars: Vec<_> = (0..dimacs.variable_count())
        .map(|var| {
            let name = format!("{}", var);
            cnf.create_variable(&name);
            cnf.get_variable(&name)
        })
        .collect();

    for dimacs_clause in dimacs.clauses() {
        let mut clause = CnfClause::new();
        for literal in dimacs_clause.literals() {
            match literal {
                dimacs::Literal::Positive(variable) => {
                    clause.set(vars[(variable - 1) as usize], true);
                }
                dimacs::Literal::Negative(variable) => {
                    clause.set(vars[(variable - 1) as usize], false);
                }
            };
        }

        cnf.add_clause(clause);
    }

    (cnf, vars)
}
