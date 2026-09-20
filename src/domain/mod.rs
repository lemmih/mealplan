//! Core domain types.

mod nutrients;
mod plan;
mod product;
mod targets;
mod template;

pub use nutrients::Nutrients;
pub use plan::{Meal, MealId, MealKind, Plan, Portion, ShoppingLine};
pub use product::{PackUnit, Product, Slot};
pub use targets::{Energy, Limits, Macros, Targets, TimeBudgets, Variety, Waste};
pub use template::{SlotSpec, Template};
