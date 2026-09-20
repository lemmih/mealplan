//! Frida / DTU Fødevaredata adapter (stub).
//!
//! Frida (<https://frida.fooddata.dk>) is the Danish national food composition
//! database and is the nutrition backbone for generic foods. The mapping from
//! catalog products to Frida food ids is maintained by hand in
//! `data/frida_map.csv`. Frida publishes a full export as XLSX/CSV; this module
//! will read that export from `data/raw/` and fill `Nutrients` for products
//! that carry a `frida_id`.
