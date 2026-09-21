//! Loaders and external data-source adapters.

mod catalog;
pub mod frida;
pub mod nemlig;
pub mod off;

pub use catalog::{load_catalog, load_targets, load_templates, save_catalog, CatalogError};
