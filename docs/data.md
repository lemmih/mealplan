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
| `nemlig_id` | int? | Nemlig product number; lets `nemlig refresh` update price and pack |
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

## Nemlig (prices, pack sizes, label nutrition)

Nemlig is a single-page app. Three anonymous endpoints cover everything the
catalog needs (implemented in `src/data/nemlig.rs`):

| Step | Endpoint | Notes |
|------|----------|-------|
| token | `GET https://www.nemlig.com/webapi/Token` | anonymous bearer, ~5 min lifetime |
| search | `GET https://webapi.prod.knl.nemlig.it/searchgateway/api/search?query=…&take=N&skip=0&recipeCount=0&timestamp=0&timeslotUtc=0&deliveryZoneId=1&includeFavorites=0` | needs the bearer; `timestamp`/`timeslotUtc` are required but accept dummies |
| product | `GET https://www.nemlig.com/{slug}-{id}?GetAsJson=1` | `HEAD /x-{id}` 301-redirects to the right slug |

The product JSON (`content[0]` with `TemplateName = productdetailspot`)
carries `Price`, `UnitPriceCalc`/`UnitPriceLabel`, a free-text
`Description` such as `280 g / Gårdkylling / Danske Familiegårde` (pack size
is parsed from its first segment), `Labels`, `SaleBeforeLastSalesDate`
(Nemlig's guaranteed remaining shelf life in days at delivery, capped at
90 and 0 when unknown) and `DeclarationLabel`, an HTML table with the
per-100 g (or per-100 ml) nutrition label. The structured `Declarations`
block is always zero in practice. EAN codes are not exposed.

Nemlig's `robots.txt` allows product pages; it only disallows `/?search=`,
`/sitecore/`, PDFs and `/webapi/order/`. The client still keeps volume tiny:

- every token, search, slug and product response is cached under
  `data/cache/nemlig/` (git-ignored) for 7 days;
- requests are spaced at least 750 ms apart;
- `refresh` only touches products that already carry a `nemlig_id`.

CLI:

```bash
mealplan nemlig search "skyr naturel" --take 5
mealplan nemlig product 5056391
mealplan nemlig row 5056391 --id kyllingebryst --slot protein --cook-yield 0.75 --prep-min 5 --cook-min 15
mealplan nemlig refresh --catalog data/samples/catalog.csv [--dry-run]
```

`row` prints a ready catalog line with nutrition from the label (many fresh
products have no label on Nemlig; fill those from Frida). `refresh` updates
`price_dkk`, `pack_qty`/`pack_unit`, `shelf_life_days` and `snapshot` in
place and reports label kcal that disagree with the catalog by more than
5 % without changing them.

The Apify actor "nemlig-scraper" remains an alternative if the endpoints
above change.
