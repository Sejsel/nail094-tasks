use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(PartialEq, Eq, Hash)]
pub struct SatVariable {
    name: String,
    id: usize,
}

pub struct CnfClause {
    values: HashMap<usize, bool>,
}

pub struct CnfSat {
    variables: HashMap<String, SatVariable>,
    clauses: Vec<CnfClause>,
}

pub struct SatModel {
    results_by_name: HashMap<String, bool>,
    results_by_id: HashMap<usize, bool>,
}

pub enum EvaluationResult {
    Sat { dimacs: String, model: SatModel, time: Duration },
    Unsat { dimacs: String, time: Duration },
}


impl CnfClause {
    pub fn new() -> CnfClause {
        CnfClause {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, variable_id: usize, value: bool) {
        self.values.insert(variable_id, value);
    }
}

impl CnfSat {
    pub fn new() -> CnfSat {
        CnfSat {
            variables: HashMap::new(),
            clauses: Vec::new(),
        }
    }

    pub fn create_variable(&mut self, name: &str) {
        if self.variables.contains_key(name) {
            panic!("The variable name has to be unique.");
        }
        let variable = SatVariable {
            name: name.to_string(),
            id: self.variables.len(),
        };
        self.variables.insert(name.to_string(), variable);
    }

    pub fn add_clause(&mut self, clause: CnfClause) {
        self.clauses.push(clause);
    }

    pub fn get_variable_by_id(&self, id: usize) -> Option<&SatVariable> {
        // TODO: This is not particularly effective.
        let (_, var) = self.variables.iter().find(|(_, var)| var.id == id)?;
        Some(var)
    }
    pub fn get_variable(&self, name: &str) -> usize {
        self.variables[name].id
    }

    pub fn ensure_at_least_one_set(&mut self, variables: &[usize]) {
        // At least one variable is chosen is simply encoded as
        // v1 ∨ v2 ∨ ... ∨ vN
        let mut clause = CnfClause::new();
        for &variable in variables {
            clause.set(variable, true);
        }
        self.add_clause(clause);
    }

    pub fn ensure_max_one_set(&mut self, variables: &[usize]) {
        // At most one variable is chosen is encoded as "There is no pair of variables that are both true"
        // That is ∀ v1, v2: ¬v1 ∨ ¬v2
        for (i, variable1) in variables.iter().enumerate() {
            for variable2 in variables[i + 1..].iter() {
                let mut pair_clause = CnfClause::new();
                pair_clause.set(*variable1, false);
                pair_clause.set(*variable2, false);
                self.add_clause(pair_clause);
            }
        }
    }

    pub fn variable_count(&self) -> usize {
        self.variables.len()
    }

    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    // DIMACS
    //   line oriented
    // c comment
    // p cnf <#variables> <#clauses>
    // index1 -index2 index3 index4 -index5
    pub fn to_dimacs(&self) -> String {
        let mut dimacs = String::new();
        let _ = writeln!(
            dimacs,
            "p cnf {} {}",
            self.variables.len(),
            self.clauses.len()
        );
        for clause in &self.clauses {
            let values = clause
                .values
                .iter()
                .map(|(id, value)| {
                    if *value {
                        format!("{}", id + 1)
                    } else {
                        format!("-{}", id + 1)
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");

            let _ = writeln!(dimacs, "{} 0", values);
        }
        dimacs
    }

    pub fn result_from_dimacs(&self, dimacs: &str) -> Result<SatModel, ()> {
        let mut satisfiable = false;
        let mut model = Vec::new();
        for line in dimacs.lines() {
            if line.starts_with('s') {
                satisfiable = !line.contains("UNSATISFIABLE")
            }
            if line.starts_with('v') {
                let model_description = line.trim()[2..].split_whitespace();
                for literal in model_description {
                    let val: i64 = literal.parse().unwrap();
                    if val == 0 {
                        continue;
                    }

                    let id = val.unsigned_abs() as usize - 1;
                    let set_true = val > 0;
                    model.push((id, set_true));
                }
            }
        }

        if satisfiable {
            Ok(SatModel::from_vec(self, &model))
        } else {
            Err(())
        }
    }

    pub fn evaluate(&self, mut solver_command: Command) -> EvaluationResult {
        let mut solver = solver_command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("Failed to run solver");

        let input = self.to_dimacs();
        solver
            .stdin
            .as_mut()
            .expect("Failed to use glucose's stdin")
            .write_all(input.as_bytes())
            .expect("Failed to write glucose's input");

        let start_time = Instant::now();
        let output = solver
            .wait_with_output()
            .expect("Failed to get output from glucose");
        let elapsed_time = start_time.elapsed();

        let dimacs_output = String::from_utf8(output.stdout).expect("Non-UTF8 output from glucose");

        match self.result_from_dimacs(&dimacs_output) {
            Ok(model) => EvaluationResult::Sat {
                dimacs: dimacs_output,
                model,
                time: elapsed_time,
            },
            Err(_) => EvaluationResult::Unsat {
                dimacs: dimacs_output,
                time: elapsed_time,
            },
        }
    }
}

impl SatModel {
    pub fn from_vec(sat: &CnfSat, model: &Vec<(usize, bool)>) -> SatModel {
        let mut results_by_name = HashMap::new();
        let mut results_by_id = HashMap::new();

        for (id, value) in model {
            let name = sat.get_variable_by_id(*id).unwrap().name.to_string();
            results_by_id.insert(*id, *value);
            results_by_name.insert(name, *value);
        }
        SatModel {
            results_by_name,
            results_by_id,
        }
    }

    pub fn get_result_by_id(&self, id: &usize) -> Option<bool> {
        let value = self.results_by_id.get(id)?;
        Some(*value)
    }

    pub fn get_result_by_name(&self, name: &str) -> Option<bool> {
        let value = self.results_by_name.get(name)?;
        Some(*value)
    }
}
