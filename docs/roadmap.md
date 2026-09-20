# Roadmap

1. **Domain, loaders, samples** — types, CSV/TOML loaders with validation,
   six-product sample catalog, `check` command. *(this scaffold)*
2. **~80-product catalog** with Frida ids and `data/frida_map.csv`.
3. **CP-SAT backend** — variables and the supply/cost skeleton; `plan`
   returns a real shopping list.
4. **Numeric constraints for one day** — energy, macro shares, saturated
   fat, salt.
5. **Templates, slots and time** — one template per meal, slot item and
   gram bounds, active and total time budgets.
6. **Wastage and shelf life across the week** — priced waste in the
   objective, `in = 0` past shelf life.
7. **Nemlig price refresh** — cached, weekly, tiny volume. *(done: `nemlig search|product|row|refresh`)*
8. **Leftovers, variety, cuisine** — batch cooking, soft variety terms,
   cuisine coherence.
