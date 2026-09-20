# Constraint model

## Sets

| Symbol | Meaning |
|--------|---------|
| `P` | products in the catalog |
| `D` | days in the horizon (day 0 is the single shopping day) |
| `M` | meal kinds: breakfast, lunch, dinner (`MEAL_KINDS`) |
| `S` | slots: protein, starch, veg, fat, dairy, sauce, snack, pantry |
| `T` | templates (a template is a map `S -> {items:[min,max], grams:[lo,hi]}`) |

Per-product data: nutrients per 100 g edible raw, `pack_p` edible grams per
pack, `price_p`, `prep_p` and `cook_p` minutes, `shelf_p` days, `slot_p`,
cuisine tags. Per-template: `base_t` minutes, `cuisine_t`, slot bounds.

## Variables

| Variable | Type | Meaning |
|----------|------|---------|
| `buy[p]` | int ≥ 0 | packs bought on day 0 |
| `use[p,d,m]` | int ≥ 0 | grams of `p` in meal `(d,m)`, in units of `GRAM_UNIT` (5 g) |
| `in[p,d,m]` | bool | `p` is part of meal `(d,m)` |
| `tpl[t,d,m]` | bool | meal `(d,m)` uses template `t` |
| `cook[d,m]` | bool | meal `(d,m)` is cooked at all |

Grams below are `GRAM_UNIT · use[p,d,m]`; nutrient coefficients are
per-gram values (per-100 g / 100).

## Numeric constraints (all linear)

- **Energy**: `kcal_min · cook ≤ Σ_p kcal_p · g[p,d,m] ≤ kcal_max · cook`
  for every meal.
- **Macro shares**, denominators cleared. With `E = Σ_p kcal_p · g` the
  energy of the meal:
  - protein: `4 · Σ_p protein_p · g ≥ protein_min · E`
  - fat: `9 · Σ_p fat_p · g ≤ fat_max · E`
  - carb: `4 · Σ_p carb_p · g ≤ carb_max · E`
- **Saturated fat**: `Σ_p satfat_p · g ≤ satfat_share_max · E / 9`.
- **Salt**: `Σ_p salt_p · g ≤ salt_g_max_meal` per meal and
  `Σ_{m,p} salt_p · g ≤ salt_g_max_day` per day.
- **Supply**: `Σ_{d,m} g[p,d,m] ≤ buy[p] · pack_p` for each product.
- **Waste**: `waste_g[p] = buy[p] · pack_p − Σ_{d,m} g[p,d,m]`, priced at
  `price_p / pack_p` per gram.

## Time constraints

For each meal `(d,m)` with active budget `A[d,m]` and total budget `B[d,m]`
(from `[time]` in targets, keyed on meal kind and weekday/weekend):

- **Active**: `Σ_p prep_p · in[p,d,m] + Σ_t base_t · tpl[t,d,m] ≤ A[d,m]`.
- **Total**: `cook_p · in[p,d,m] ≤ B[d,m]` for each `p` — the meal's total
  time is the max over its products' cook times, expressed as one row per
  product instead of a max constraint.

## Structural constraints

- **One template per cooked meal**: `Σ_t tpl[t,d,m] = cook[d,m]`.
- **Slot item counts**: for each template `t`, slot `s`, meal `(d,m)`:
  `items_min[t,s] · tpl[t,d,m] ≤ Σ_{p: slot_p = s} in[p,d,m]
   ≤ items_max[t,s] + big · (1 − tpl[t,d,m])`; with CP-SAT this is written
  as a reified linear constraint enforced only if `tpl[t,d,m]`, no big-M.
  Products whose slot is absent from the chosen template are excluded
  (`in[p,d,m] = 0` when `tpl[t,d,m]` and `slot_p ∉ t`).
- **Grams follow membership**: `lo · in ≤ g[p,d,m] ≤ hi · in` where `lo`,
  `hi` come from the active template's slot bounds (again reified on `tpl`).
- **Cuisine coherence**: `in[p,d,m] = 0` whenever `tpl[t,d,m]` and `p` does
  not carry `cuisine_t` (or `any`).
- **Shelf life** (single shopping day): `in[p,d,m] = 0` for `d > shelf_p`.
  Open-life is phase 2 together with leftovers.
- **Variety**: `Σ_{d,m} tpl[t,d,m] ≤ max_same_template_per_week` for each
  `t`; `Σ_{d,m} in[p,d,m] ≤ max_same_protein_per_week` for each protein `p`.
- **Leftovers** (phase 2): a meal cooked once and eaten on later days;
  introduces batch variables and moves supply from per-meal to per-batch.

## Objective

Lexicographic, or a weighted sum with well-separated weights:

1. purchase cost `Σ_p price_p · buy[p]`
2. priced waste `price_weight · Σ_p (price_p / pack_p) · waste_g[p]`
3. active time `Σ_{d,m} (Σ_p prep_p · in + Σ_t base_t · tpl)`
4. variety as a soft term (penalise repeated templates/proteins beyond a
   lower comfort threshold).

## Why CP-SAT

The model mixes booleans (`in`, `tpl`, `cook`), small integers (`buy`,
`use`) and linear arithmetic over them. CP-SAT handles all three in one
model and supports *reification* natively (`OnlyEnforceIf`): slot bounds
and cuisine exclusions are attached to `tpl[t,d,m]` directly instead of
being encoded with big-M rows. That keeps the LP relaxation tight, avoids
numeric trouble from large constants, and lets the SAT core propagate the
structural part while the LP handles the numeric part. The `mip` backend
(good_lp + HiGHS) exists as a portable fallback that needs big-M
formulations for the same constraints.
