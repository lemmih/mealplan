# mealplan

Weekly meal planning as a constraint program.

Recipes are **not** fixed inputs. The solver composes each meal from
ingredients under structural constraints (a coherent meal built from a
template of slots: protein, starch, veg, ...) and numeric ones (energy,
macronutrient shares, saturated fat, salt, active and total cooking time,
purchase cost and wastage). Prices and pack sizes come from
[nemlig.com](https://www.nemlig.com), nutrition for generic foods from
[Frida / DTU Fødevaredata](https://frida.fooddata.dk), and branded products
from [Open Food Facts](https://world.openfoodfacts.org) by EAN.

See [docs/model.md](docs/model.md) for the constraint model,
[docs/data.md](docs/data.md) for data sources and the catalog format, and
[docs/roadmap.md](docs/roadmap.md) for the plan.

## Layout

```
src/domain/   Nutrients, Product, Template, Targets, Plan
src/data/     catalog/targets/templates loaders; frida, nemlig, off stubs
src/solve/    Problem, Solver trait, feature-gated cpsat and mip backends
data/samples/ small catalog.csv, targets.toml, templates.toml
docs/         model, data, roadmap
```

## Usage

```bash
cargo run -- check --catalog data/samples/catalog.csv
cargo run --features mip -- plan --catalog data/samples/catalog.csv \
    --targets data/samples/targets.toml --templates data/samples/templates.toml --days 7
```

`check` validates the catalog and prints edible grams and price per edible
gram per product. `plan` calls the enabled backend and prints the plan as
JSON. Both backends are stubs for now and return "not implemented".

## Solver backends

No backend is enabled by default.

| Feature | Backend | Requirements |
|---------|---------|--------------|
| `cpsat` | OR-Tools CP-SAT via [`cp_sat`](https://crates.io/crates/cp_sat) 0.4.1 | Native OR-Tools + C++ compiler, see below |
| `mip`   | [`good_lp`](https://crates.io/crates/good_lp) 1.15 with HiGHS | None (HiGHS builds from source) |

### Installing OR-Tools for `cpsat` (macOS)

```bash
brew install or-tools
```

`cp_sat` looks for `include/ortools/sat/cp_model.h` under `$ORTOOLS_PREFIX`,
then `/usr/local`, `/usr`, `/opt/homebrew`, `/opt/ortools`. Recent OR-Tools
links protobuf dynamically, so build with:

```bash
RUSTFLAGS='-Clink-arg=-lprotobuf' cargo build --features cpsat
```

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```
