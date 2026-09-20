//! Problem definition and solver backends.

use crate::domain::{MealKind, Plan, Product, Targets, Template};
use anyhow::Result;

#[cfg(feature = "cpsat")]
pub mod cpsat;
#[cfg(feature = "mip")]
pub mod mip;

/// Meal kinds planned per day, in order.
pub const MEAL_KINDS: [MealKind; 3] = [MealKind::Breakfast, MealKind::Lunch, MealKind::Dinner];

/// Granularity of the integer `use[p,d,m]` variables, in grams.
pub const GRAM_UNIT: f64 = 5.0;

/// Everything a backend needs to build a model.
#[derive(Debug, Clone)]
pub struct Problem {
    pub catalog: Vec<Product>,
    pub templates: Vec<Template>,
    pub targets: Targets,
    /// Planning horizon in days; day 0 is the shopping day.
    pub days: u32,
}

pub trait Solver {
    fn solve(&self, problem: &Problem) -> Result<Plan>;
}

/// The backend selected by cargo features. Errors if none is enabled.
pub fn default_solver() -> Result<Box<dyn Solver>> {
    #[cfg(feature = "cpsat")]
    {
        return Ok(Box::new(cpsat::CpSatSolver));
    }
    #[cfg(all(feature = "mip", not(feature = "cpsat")))]
    {
        return Ok(Box::new(mip::MipSolver));
    }
    #[allow(unreachable_code)]
    Err(anyhow::anyhow!(
        "no solver backend enabled; build with --features cpsat or --features mip"
    ))
}
