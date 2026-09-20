# Data sources and the catalog format

## catalog.csv

One row per purchasable product. Nutrients are per 100 g **edible raw**
food. Empty cells mean `None` for optional columns.

| Column | Type | Meaning |
|--------|------|---------|
| `id` | string | unique snake_case key, referenced by plans and `frida_map.csv` |
| `name` | string | display name (Danish, as on nemlig) |
| `ean` | string? | barcode, for Open Food Facts lookup |
| `price_dkk` | float | price per pack |
| `pack_qty` | float | pack size in `pack_unit` |
| `pack_unit` | `g` \| `ml` \| `pcs` | unit of `pack_qty`; `ml` is treated as grams |
| `grams_per_piece` | float? | required when `pack_unit = pcs` |
| `frida_id` | int? | Frida food id supplying the nutrients |
| `kcal` | float | energy per 100 g |
| `protein`, `fat`, `satfat`, `carb`, `sugar`, `fibre`, `salt` | float | grams per 100 g |
| `edible_fraction` | float in (0, 1] | share of pack weight that is edible after trimming |
| `cook_yield` | float > 0 | cooked weight / raw weight |
| `shelf_life_days` | int | days from purchase until unusable |
| `open_life_days` | int? | days after opening, if shorter |
| `slot` | enum | `protein`, `starch`, `veg`, `fat`, `dairy`, `sauce`, `snack`, `pantry` |
| `cuisine_tags` | `;`-separated | e.g. `any;asian;nordic`; `any` matches every template |
| `prep_min` | float | active minutes to prepare |
| `cook_min` | float | unattended minutes (oven, boiling) |
| `snapshot` | date | `YYYY-MM-DD` the price and pack size were observed |

Validation on load (`load_catalog`): unique ids, `edible_fraction` in
(0, 1], `cook_yield > 0`, pack > 0 grams, and label `kcal` within 25 % of
the macro-implied energy (4/9/4/2 kcal per gram of protein/fat/carb/fibre).

## Frida / DTU Fødevaredata (backbone)

[Frida](https://frida.fooddata.dk) is DTU's national food composition
database, covering generic foods with full nutrient profiles. It is the
default nutrition source. The mapping from catalog `id` to Frida food id is
hand-made in `data/frida_map.csv` (`id,frida_id,note`), because product
names on nemlig rarely match Frida names automatically. Frida's export is
downloaded once into `data/raw/` (git-ignored).

## Open Food Facts (gaps)

Branded products without a good Frida match are looked up by EAN via the
OFF API v2:

```
GET https://world.openfoodfacts.org/api/v2/product/{ean}?fields=product_name,nutriments,quantity
```

The `nutriments` object carries `energy-kcal_100g`, `proteins_100g`,
`fat_100g`, `saturated-fat_100g`, `carbohydrates_100g`, `sugars_100g`,
`fiber_100g`, `salt_100g`. OFF data is crowd-sourced; keep the 25 % kcal
sanity check.

## Nemlig (prices and pack sizes)

Nemlig is a single-page app. Its product JSON API is visible in browser
devtools (search and product endpoints under `/webapi/`). Nemlig's
`robots.txt` disallows crawlers, so:

- fetch only the products already in the catalog, never browse categories;
- cache responses under `data/cache/` (git-ignored);
- refresh at most weekly and record the date in `snapshot`.

The Apify actor "nemlig-scraper" is an alternative that yields the same
fields (name, price, quantity, EAN) without hand-rolling requests.
