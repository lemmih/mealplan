//! MIP backend via good_lp + HiGHS (stub).

use super::{Problem, Solver};
use crate::domain::Plan;
use anyhow::{bail, Result};

pub struct MipSolver;

impl Solver for MipSolver {
    fn solve(&self, _problem: &Problem) -> Result<Plan> {
        bail!("mip backend: not implemented")
    }
}
