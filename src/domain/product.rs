use super::Nutrients;
use serde::{Deserialize, Serialize};

/// The structural role a product plays in a meal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Slot {
    Protein,
    Starch,
    Veg,
    Fat,
    Dairy,
    Sauce,
    Snack,
    Pantry,
}

impl Slot {
    pub const ALL: [Slot; 8] = [
        Slot::Protein,
        Slot::Starch,
        Slot::Veg,
        Slot::Fat,
        Slot::Dairy,
        Slot::Sauce,
        Slot::Snack,
        Slot::Pantry,
    ];
}

impl std::fmt::Display for Slot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Slot::Protein => "protein",
            Slot::Starch => "starch",
            Slot::Veg => "veg",
            Slot::Fat => "fat",
            Slot::Dairy => "dairy",
            Slot::Sauce => "sauce",
            Slot::Snack => "snack",
            Slot::Pantry => "pantry",
        };
        f.pad(s)
    }
}

/// Unit of `pack_qty`. Millilitres are treated as grams (density 1.0) until a
/// per-product density is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackUnit {
    G,
    Ml,
    Pcs,
}

/// A purchasable product: one row of the catalog.
///
/// Nutrients are per 100 g of edible raw food and are flattened into the row
/// (columns `kcal`, `protein`, ... sit next to `price_dkk` etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    pub ean: Option<String>,
    pub price_dkk: f64,
    pub pack_qty: f64,
    pub pack_unit: PackUnit,
    pub grams_per_piece: Option<f64>,
    pub frida_id: Option<u32>,
    #[serde(flatten)]
    pub nutrients: Nutrients,
    /// Share of the pack that is edible after trimming, in (0, 1].
    pub edible_fraction: f64,
    /// Cooked weight divided by raw weight (rice ≈ 2.8, chicken ≈ 0.75).
    pub cook_yield: f64,
    pub shelf_life_days: u32,
    /// Days the product keeps once opened, if shorter than `shelf_life_days`.
    pub open_life_days: Option<u32>,
    pub slot: Slot,
    pub cuisine_tags: Vec<String>,
    pub prep_min: f64,
    pub cook_min: f64,
    /// Date (YYYY-MM-DD) the price and pack size were observed.
    pub snapshot: String,
}

impl Product {
    /// Raw grams in one pack, before trimming.
    pub fn grams_per_pack(&self) -> f64 {
        match self.pack_unit {
            PackUnit::G | PackUnit::Ml => self.pack_qty,
            PackUnit::Pcs => self.pack_qty * self.grams_per_piece.unwrap_or(0.0),
        }
    }

    /// Edible raw grams in one pack.
    pub fn edible_grams_per_pack(&self) -> f64 {
        self.grams_per_pack() * self.edible_fraction
    }

    /// Price in DKK per edible raw gram.
    pub fn price_per_edible_gram(&self) -> f64 {
        self.price_dkk / self.edible_grams_per_pack()
    }

    /// True if the product is tagged with `cuisine` or with the wildcard `any`.
    pub fn has_cuisine(&self, cuisine: &str) -> bool {
        self.cuisine_tags.iter().any(|t| t == cuisine || t == "any")
    }
}
