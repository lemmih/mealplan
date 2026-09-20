use super::Slot;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Bounds for one slot within a template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotSpec {
    /// `[min, max]` number of distinct products filling this slot.
    pub items: [u32; 2],
    /// `[lo, hi]` grams (edible raw) per product placed in this slot.
    pub grams: [f64; 2],
}

/// A meal structure: which slots a meal has and how much goes in each.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    /// Cuisine the meal must be coherent with; `any` disables the check.
    pub cuisine: String,
    /// Fixed active minutes for assembling this kind of meal, on top of
    /// per-product prep time.
    pub base_min: f64,
    pub slots: BTreeMap<Slot, SlotSpec>,
}
