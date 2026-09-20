//! OR-Tools CP-SAT backend (stub).

use super::{Problem, Solver};
use crate::domain::Plan;
use anyhow::{bail, Result};

pub struct CpSatSolver;

impl Solver for CpSatSolver {
    fn solve(&self, _problem: &Problem) -> Result<Plan> {
        bail!("cpsat backend: not implemented")
    }
}
