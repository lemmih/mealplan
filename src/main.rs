use anyhow::Result;
use clap::{Parser, Subcommand};
use mealplan::data::{load_catalog, load_targets, load_templates};
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
    }
    Ok(())
}
