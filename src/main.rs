use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use mealplan::data::nemlig;
use mealplan::data::{load_catalog, load_targets, load_templates, save_catalog};
use mealplan::domain::{PackUnit, Slot};
use mealplan::solve::{default_solver, Problem};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "mealplan",
    version,
    about = "Weekly meal planning as a constraint program"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Validate the catalog and print price per edible gram and slot per product.
    Check {
        #[arg(long, default_value = "data/samples/catalog.csv")]
        catalog: PathBuf,
    },
    /// Build a plan with the enabled solver backend and print it as JSON.
    Plan {
        #[arg(long, default_value = "data/samples/catalog.csv")]
        catalog: PathBuf,
        #[arg(long, default_value = "data/samples/targets.toml")]
        targets: PathBuf,
        #[arg(long, default_value = "data/samples/templates.toml")]
        templates: PathBuf,
        #[arg(long, default_value_t = 7)]
        days: u32,
    },
    /// Query nemlig.com (cached weekly under data/cache/nemlig).
    Nemlig {
        #[arg(long, default_value = "data/cache/nemlig", global = true)]
        cache: PathBuf,
        #[command(subcommand)]
        cmd: NemligCmd,
    },
}

#[derive(Subcommand)]
enum NemligCmd {
    /// Search products by free text.
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        take: u32,
    },
    /// Fetch one product and print it as JSON.
    Product { id: u32 },
    /// Print a catalog.csv row for a nemlig product, with nutrition from the label.
    Row {
        nemlig_id: u32,
        /// Catalog id for the new row.
        #[arg(long)]
        id: String,
        #[arg(long, value_parser = parse_slot)]
        slot: Slot,
        #[arg(long, default_value = "any")]
        cuisine: String,
        #[arg(long, default_value_t = 1.0)]
        edible_fraction: f64,
        #[arg(long, default_value_t = 1.0)]
        cook_yield: f64,
        #[arg(long, default_value_t = 0.0)]
        prep_min: f64,
        #[arg(long, default_value_t = 0.0)]
        cook_min: f64,
        /// Required when the pack is counted in pieces.
        #[arg(long)]
        grams_per_piece: Option<f64>,
    },
    /// Refresh price, pack size, shelf life and snapshot date for every catalog
    /// product that has a nemlig_id. Rewrites the CSV in place.
    Refresh {
        #[arg(long, default_value = "data/samples/catalog.csv")]
        catalog: PathBuf,
        /// Only report, do not write.
        #[arg(long)]
        dry_run: bool,
    },
}

fn parse_slot(s: &str) -> Result<Slot, String> {
    Slot::ALL
        .iter()
        .copied()
        .find(|slot| slot.to_string() == s)
        .ok_or_else(|| format!("unknown slot {s:?}; expected one of protein, starch, veg, fat, dairy, sauce, snack, pantry"))
}

fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

fn pack_unit(u: nemlig::PackUnit) -> PackUnit {
    match u {
        nemlig::PackUnit::G => PackUnit::G,
        nemlig::PackUnit::Ml => PackUnit::Ml,
        nemlig::PackUnit::Pcs => PackUnit::Pcs,
    }
}

fn unit_str(u: PackUnit) -> &'static str {
    match u {
        PackUnit::G => "g",
        PackUnit::Ml => "ml",
        PackUnit::Pcs => "pcs",
    }
}

fn opt(v: Option<f64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Check { catalog } => {
            let products = load_catalog(&catalog)?;
            println!(
                "{:<16} {:<8} {:>10} {:>12}",
                "id", "slot", "edible g", "DKK/edible g"
            );
            for p in &products {
                println!(
                    "{:<16} {:<8} {:>10.0} {:>12.4}",
                    p.id,
                    p.slot,
                    p.edible_grams_per_pack(),
                    p.price_per_edible_gram()
                );
            }
            println!("{} products OK", products.len());
        }
        Cmd::Plan {
            catalog,
            targets,
            templates,
            days,
        } => {
            let problem = Problem {
                catalog: load_catalog(&catalog)?,
                templates: load_templates(&templates)?,
                targets: load_targets(&targets)?,
                days,
            };
            let plan = default_solver()?.solve(&problem)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Cmd::Nemlig { cache, cmd } => run_nemlig(cache, cmd)?,
    }
    Ok(())
}

fn run_nemlig(cache: PathBuf, cmd: NemligCmd) -> Result<()> {
    let mut client = nemlig::Client::new(cache);
    match cmd {
        NemligCmd::Search { query, take } => {
            let hits = client.search(&query, take)?;
            println!(
                "{:<8} {:<40} {:<32} {:>8} {:>10} stock",
                "id", "name", "description", "DKK", "unit"
            );
            for h in &hits {
                println!(
                    "{:<8} {:<40.40} {:<32.32} {:>8.2} {:>10} {}",
                    h.id,
                    h.name,
                    h.description,
                    h.price_dkk,
                    format!("{:.2} {}", h.unit_price, h.unit_price_label),
                    if h.in_stock { "yes" } else { "no" }
                );
            }
        }
        NemligCmd::Product { id } => {
            let p = client.product(id)?;
            println!("{}", serde_json::to_string_pretty(&p)?);
        }
        NemligCmd::Row {
            nemlig_id,
            id,
            slot,
            cuisine,
            edible_fraction,
            cook_yield,
            prep_min,
            cook_min,
            grams_per_piece,
        } => {
            let p = client.product(nemlig_id)?;
            let pack = p
                .pack
                .ok_or_else(|| anyhow!("could not parse pack size from {:?}", p.description))?;
            let unit = pack_unit(pack.unit);
            if unit == PackUnit::Pcs && grams_per_piece.is_none() {
                return Err(anyhow!("pack is {} pcs; pass --grams-per-piece", pack.qty));
            }
            let n = p.nutrition.clone().unwrap_or_default();
            if p.nutrition.is_none() {
                eprintln!(
                    "warning: no nutrition label on nemlig for {nemlig_id}; fill kcal.. by hand"
                );
            } else if n.per_100 != "g" {
                eprintln!(
                    "warning: label is per 100 {}; treated as per 100 g",
                    n.per_100
                );
            }
            // Same column order as data/samples/catalog.csv.
            let row = [
                id,
                p.name.clone(),
                String::new(),
                p.price_dkk.to_string(),
                pack.qty.to_string(),
                unit_str(unit).to_string(),
                opt(grams_per_piece),
                String::new(),
                nemlig_id.to_string(),
                opt(n.kcal),
                opt(n.protein),
                opt(n.fat),
                opt(n.satfat),
                opt(n.carb),
                opt(n.sugar),
                opt(n.fibre),
                opt(n.salt),
                edible_fraction.to_string(),
                cook_yield.to_string(),
                p.sale_before_last_sales_date.max(1).to_string(),
                String::new(),
                slot.to_string(),
                cuisine,
                prep_min.to_string(),
                cook_min.to_string(),
                today(),
            ];
            let mut w = csv::WriterBuilder::new()
                .has_headers(false)
                .from_writer(std::io::stdout());
            w.write_record(row)?;
            w.flush()?;
        }
        NemligCmd::Refresh { catalog, dry_run } => {
            let mut products = load_catalog(&catalog)?;
            let mut changed = 0;
            for prod in products.iter_mut() {
                let Some(nid) = prod.nemlig_id else { continue };
                let np = client.product(nid)?;
                let mut notes = Vec::new();
                if (np.price_dkk - prod.price_dkk).abs() > 0.005 {
                    notes.push(format!(
                        "price {:.2} -> {:.2}",
                        prod.price_dkk, np.price_dkk
                    ));
                    prod.price_dkk = np.price_dkk;
                }
                if let Some(pack) = np.pack {
                    let unit = pack_unit(pack.unit);
                    if unit != prod.pack_unit || (pack.qty - prod.pack_qty).abs() > 0.005 {
                        notes.push(format!(
                            "pack {} {} -> {} {}",
                            prod.pack_qty,
                            unit_str(prod.pack_unit),
                            pack.qty,
                            unit_str(unit)
                        ));
                        prod.pack_qty = pack.qty;
                        prod.pack_unit = unit;
                    }
                }
                let shelf = np.sale_before_last_sales_date;
                if shelf > 0 && shelf != prod.shelf_life_days {
                    notes.push(format!("shelf {} -> {}", prod.shelf_life_days, shelf));
                    prod.shelf_life_days = shelf;
                }
                if let Some(n) = &np.nutrition {
                    if let Some(k) = n.kcal {
                        if (k - prod.nutrients.kcal).abs() / k.max(1.0) > 0.05 {
                            notes.push(format!(
                                "label kcal {} vs catalog {} (not changed)",
                                k, prod.nutrients.kcal
                            ));
                        }
                    }
                }
                if notes.is_empty() {
                    println!("{:<16} unchanged", prod.id);
                } else {
                    changed += 1;
                    println!("{:<16} {}", prod.id, notes.join("; "));
                }
                prod.snapshot = today();
            }
            if dry_run {
                println!("dry run: {changed} products would change");
            } else {
                save_catalog(&catalog, &products)?;
                println!("wrote {} ({changed} products changed)", catalog.display());
            }
        }
    }
    Ok(())
}
