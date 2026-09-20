//! Nemlig.com price and pack-size adapter (stub).
//!
//! Nemlig is a single-page app whose product JSON API is visible in browser
//! devtools. Its `robots.txt` disallows crawlers, so this module will only
//! ever fetch the handful of products already in the catalog, cache results
//! under `data/cache/` and refresh at most weekly. The Apify actor
//! "nemlig-scraper" is an alternative source for the same fields.
