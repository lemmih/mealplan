use serde::{Deserialize, Serialize};

/// Per-meal energy window in kcal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Energy {
    pub kcal_min: f64,
    pub kcal_max: f64,
}

/// Macronutrient bounds as shares of energy (0–1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Macros {
    pub protein_min: f64,
    pub fat_max: f64,
    pub carb_max: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Limits {
    /// Saturated fat as a share of energy.
    pub satfat_share_max: f64,
    pub salt_g_max_meal: f64,
    pub salt_g_max_day: f64,
}

/// Minutes as `[active, total]` per meal kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeBudgets {
    pub weekday_dinner: [f64; 2],
    pub weekend_dinner: [f64; 2],
    pub lunch: [f64; 2],
    pub breakfast: [f64; 2],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variety {
    pub max_same_template_per_week: u32,
    pub max_same_protein_per_week: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Waste {
    /// Weight of priced waste relative to purchase cost in the objective.
    pub price_weight: f64,
}

/// Everything in `targets.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Targets {
    pub energy: Energy,
    pub macros: Macros,
    pub limits: Limits,
    pub time: TimeBudgets,
    pub variety: Variety,
    pub waste: Waste,
}
