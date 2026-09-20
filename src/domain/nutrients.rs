use serde::{Deserialize, Serialize};
use std::ops::{Add, Mul};

/// Nutrient content per 100 g of edible, raw food.
///
/// Units: `kcal` in kilocalories, everything else in grams.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Nutrients {
    pub kcal: f64,
    pub protein: f64,
    pub fat: f64,
    pub satfat: f64,
    pub carb: f64,
    pub sugar: f64,
    pub fibre: f64,
    pub salt: f64,
}

/// Atwater factors as used on EU labels (Regulation 1169/2011, Annex XIV).
pub const KCAL_PER_G_PROTEIN: f64 = 4.0;
pub const KCAL_PER_G_FAT: f64 = 9.0;
pub const KCAL_PER_G_CARB: f64 = 4.0;
pub const KCAL_PER_G_FIBRE: f64 = 2.0;

impl Nutrients {
    /// Energy implied by the macronutrients alone (protein, fat, carbohydrate,
    /// fibre). Used to sanity-check label `kcal` against the macros.
    pub fn kcal_from_macros(&self) -> f64 {
        self.protein * KCAL_PER_G_PROTEIN
            + self.fat * KCAL_PER_G_FAT
            + self.carb * KCAL_PER_G_CARB
            + self.fibre * KCAL_PER_G_FIBRE
    }
}

impl Add for Nutrients {
    type Output = Nutrients;

    fn add(self, o: Nutrients) -> Nutrients {
        Nutrients {
            kcal: self.kcal + o.kcal,
            protein: self.protein + o.protein,
            fat: self.fat + o.fat,
            satfat: self.satfat + o.satfat,
            carb: self.carb + o.carb,
            sugar: self.sugar + o.sugar,
            fibre: self.fibre + o.fibre,
            salt: self.salt + o.salt,
        }
    }
}

impl Mul<f64> for Nutrients {
    type Output = Nutrients;

    fn mul(self, k: f64) -> Nutrients {
        Nutrients {
            kcal: self.kcal * k,
            protein: self.protein * k,
            fat: self.fat * k,
            satfat: self.satfat * k,
            carb: self.carb * k,
            sugar: self.sugar * k,
            fibre: self.fibre * k,
            salt: self.salt * k,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_scale() {
        let a = Nutrients {
            kcal: 100.0,
            protein: 10.0,
            ..Default::default()
        };
        let b = Nutrients {
            kcal: 50.0,
            fat: 5.0,
            ..Default::default()
        };
        let c = (a + b) * 2.0;
        assert_eq!(c.kcal, 300.0);
        assert_eq!(c.protein, 20.0);
        assert_eq!(c.fat, 10.0);
    }

    #[test]
    fn macro_energy() {
        let n = Nutrients {
            protein: 10.0,
            fat: 10.0,
            carb: 10.0,
            fibre: 5.0,
            ..Default::default()
        };
        assert_eq!(n.kcal_from_macros(), 40.0 + 90.0 + 40.0 + 10.0);
    }
}
