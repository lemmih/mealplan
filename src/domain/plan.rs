use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MealKind {
    Breakfast,
    Lunch,
    Dinner,
}

/// Identifies one meal in the planning horizon: day index (0-based) and kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MealId {
    pub day: u32,
    pub kind: MealKind,
}

/// One product used in one meal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Portion {
    pub product_id: String,
    /// Edible raw grams.
    pub grams: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Meal {
    pub id: MealId,
    /// Template name, or `None` for a meal that is not cooked (skipped).
    pub template: Option<String>,
    pub portions: Vec<Portion>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShoppingLine {
    pub product_id: String,
    pub packs: u32,
}

/// Solver output: meals over the horizon plus the shopping list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub days: u32,
    pub meals: Vec<Meal>,
    pub shopping: Vec<ShoppingLine>,
    pub cost_dkk: f64,
    pub waste_dkk: f64,
}
