//! Weekly meal planning as a constraint program.
//!
//! Recipes are not fixed inputs. The solver composes meals from ingredients
//! (products in a catalog) under structural constraints (a coherent meal built
//! from a template of slots) and numeric ones (energy, macronutrient shares,
//! saturated fat, salt, cooking time, wastage).
//!
//! * [`domain`] — core types: nutrients, products, templates, targets, plans.
//! * [`data`] — loaders for the catalog CSV, targets and templates TOML, plus
//!   stubs for the external data sources (Frida, Nemlig, Open Food Facts).
//! * [`solve`] — the [`solve::Problem`] definition, the [`solve::Solver`]
//!   trait and feature-gated backends.

pub mod data;
pub mod domain;
pub mod solve;
