//! Loading and validating the product catalog, targets and templates.

use crate::domain::{Nutrients, PackUnit, Product, Slot, Targets, Template};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Label kcal may deviate this much (relative) from macro-implied kcal.
pub const KCAL_TOLERANCE: f64 = 0.25;

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error("product {id}: edible_fraction must be in (0, 1], got {value}")]
    EdibleFraction { id: String, value: f64 },
    #[error("product {id}: cook_yield must be > 0, got {value}")]
    CookYield { id: String, value: f64 },
    #[error(
        "product {id}: pack must be > 0 grams (pack_qty {qty} {unit:?}, grams_per_piece {gpp:?})"
    )]
    Pack {
        id: String,
        qty: f64,
        unit: PackUnit,
        gpp: Option<f64>,
    },
    #[error("product {id}: label kcal {label} is more than {tol:.0}% from macro-implied kcal {implied:.1}")]
    Kcal {
        id: String,
        label: f64,
        implied: f64,
        tol: f64,
    },
    #[error("duplicate product id {0}")]
    DuplicateId(String),
}

/// Flat CSV row. The `csv` crate cannot deserialise `#[serde(flatten)]`
/// numeric fields (they arrive as strings), so nutrients are listed
/// explicitly here and folded into [`Product`] afterwards.
#[derive(Debug, Serialize, Deserialize)]
struct ProductRow {
    id: String,
    name: String,
    ean: Option<String>,
    price_dkk: f64,
    pack_qty: f64,
    pack_unit: PackUnit,
    grams_per_piece: Option<f64>,
    frida_id: Option<u32>,
    nemlig_id: Option<u32>,
    kcal: f64,
    protein: f64,
    fat: f64,
    satfat: f64,
    carb: f64,
    sugar: f64,
    fibre: f64,
    salt: f64,
    edible_fraction: f64,
    cook_yield: f64,
    shelf_life_days: u32,
    open_life_days: Option<u32>,
    slot: Slot,
    cuisine_tags: String,
    prep_min: f64,
    cook_min: f64,
    snapshot: String,
}

impl From<ProductRow> for Product {
    fn from(r: ProductRow) -> Self {
        Product {
            id: r.id,
            name: r.name,
            ean: r.ean.filter(|s| !s.is_empty()),
            price_dkk: r.price_dkk,
            pack_qty: r.pack_qty,
            pack_unit: r.pack_unit,
            grams_per_piece: r.grams_per_piece,
            frida_id: r.frida_id,
            nemlig_id: r.nemlig_id,
            nutrients: Nutrients {
                kcal: r.kcal,
                protein: r.protein,
                fat: r.fat,
                satfat: r.satfat,
                carb: r.carb,
                sugar: r.sugar,
                fibre: r.fibre,
                salt: r.salt,
            },
            edible_fraction: r.edible_fraction,
            cook_yield: r.cook_yield,
            shelf_life_days: r.shelf_life_days,
            open_life_days: r.open_life_days,
            slot: r.slot,
            cuisine_tags: r
                .cuisine_tags
                .split(';')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect(),
            prep_min: r.prep_min,
            cook_min: r.cook_min,
            snapshot: r.snapshot,
        }
    }
}

impl From<&Product> for ProductRow {
    fn from(p: &Product) -> Self {
        ProductRow {
            id: p.id.clone(),
            name: p.name.clone(),
            ean: p.ean.clone(),
            price_dkk: p.price_dkk,
            pack_qty: p.pack_qty,
            pack_unit: p.pack_unit,
            grams_per_piece: p.grams_per_piece,
            frida_id: p.frida_id,
            nemlig_id: p.nemlig_id,
            kcal: p.nutrients.kcal,
            protein: p.nutrients.protein,
            fat: p.nutrients.fat,
            satfat: p.nutrients.satfat,
            carb: p.nutrients.carb,
            sugar: p.nutrients.sugar,
            fibre: p.nutrients.fibre,
            salt: p.nutrients.salt,
            edible_fraction: p.edible_fraction,
            cook_yield: p.cook_yield,
            shelf_life_days: p.shelf_life_days,
            open_life_days: p.open_life_days,
            slot: p.slot,
            cuisine_tags: p.cuisine_tags.join(";"),
            prep_min: p.prep_min,
            cook_min: p.cook_min,
            snapshot: p.snapshot.clone(),
        }
    }
}

/// Write products back to `catalog.csv` in the canonical column order.
pub fn save_catalog(path: impl AsRef<Path>, products: &[Product]) -> Result<()> {
    let path = path.as_ref();
    let mut w =
        csv::Writer::from_path(path).with_context(|| format!("writing {}", path.display()))?;
    for p in products {
        w.serialize(ProductRow::from(p))?;
    }
    w.flush()?;
    Ok(())
}

/// Validate one product's invariants.
pub fn validate_product(p: &Product) -> Result<(), CatalogError> {
    let id = p.id.clone();
    if p.edible_fraction.is_nan() || p.edible_fraction <= 0.0 || p.edible_fraction > 1.0 {
        return Err(CatalogError::EdibleFraction {
            id,
            value: p.edible_fraction,
        });
    }
    if p.cook_yield.is_nan() || p.cook_yield <= 0.0 {
        return Err(CatalogError::CookYield {
            id,
            value: p.cook_yield,
        });
    }
    let pack_g = p.grams_per_pack();
    if pack_g.is_nan() || pack_g <= 0.0 {
        return Err(CatalogError::Pack {
            id,
            qty: p.pack_qty,
            unit: p.pack_unit,
            gpp: p.grams_per_piece,
        });
    }
    let implied = p.nutrients.kcal_from_macros();
    let label = p.nutrients.kcal;
    let scale = label.max(implied).max(1.0);
    if (label - implied).abs() / scale > KCAL_TOLERANCE {
        return Err(CatalogError::Kcal {
            id,
            label,
            implied,
            tol: KCAL_TOLERANCE * 100.0,
        });
    }
    Ok(())
}

/// Load and validate `catalog.csv`.
pub fn load_catalog(path: impl AsRef<Path>) -> Result<Vec<Product>> {
    let path = path.as_ref();
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)
        .with_context(|| format!("opening catalog {}", path.display()))?;
    let mut products = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for (i, row) in rdr.deserialize::<ProductRow>().enumerate() {
        let row = row.with_context(|| format!("{}: row {}", path.display(), i + 2))?;
        let p: Product = row.into();
        if !seen.insert(p.id.clone()) {
            return Err(CatalogError::DuplicateId(p.id).into());
        }
        validate_product(&p)?;
        products.push(p);
    }
    Ok(products)
}

/// Load `targets.toml`.
pub fn load_targets(path: impl AsRef<Path>) -> Result<Targets> {
    let path = path.as_ref();
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

#[derive(Debug, Deserialize)]
struct TemplatesFile {
    template: Vec<Template>,
}

/// Load `templates.toml` (an array of `[[template]]` tables).
pub fn load_templates(path: impl AsRef<Path>) -> Result<Vec<Template>> {
    let path = path.as_ref();
    let text =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let file: TemplatesFile =
        toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
    for t in &file.template {
        for (slot, spec) in &t.slots {
            anyhow::ensure!(
                spec.items[0] <= spec.items[1],
                "template {}: slot {slot} items min > max",
                t.name
            );
            anyhow::ensure!(
                spec.grams[0] <= spec.grams[1],
                "template {}: slot {slot} grams lo > hi",
                t.name
            );
        }
    }
    Ok(file.template)
}
