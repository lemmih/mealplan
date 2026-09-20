//! Nemlig.com price, pack-size and label-nutrition adapter.
//!
//! Nemlig is a single-page app. Three anonymous endpoints are enough:
//!
//! 1. `GET https://www.nemlig.com/webapi/Token` — a short-lived (about five
//!    minutes) anonymous bearer token.
//! 2. `GET https://webapi.prod.knl.nemlig.it/searchgateway/api/search?query=…`
//!    with that bearer — product search. The `timestamp`/`timeslotUtc`
//!    parameters are required but accept dummy values.
//! 3. `GET https://www.nemlig.com/{slug}-{id}?GetAsJson=1` — the product page
//!    as JSON: price, unit price, pack description, labels and the nutrition
//!    table (as an HTML `DeclarationLabel`; the structured `Declarations`
//!    block is empty in practice). A wrong slug 301-redirects to the right
//!    one, dropping the query string, so we resolve the slug first.
//!
//! Nemlig's `robots.txt` allows product pages and only disallows
//! `/?search=`, `/sitecore/`, PDFs and `/webapi/order/`. We still keep volume
//! tiny: every response is cached under `data/cache/nemlig/` for
//! [`CACHE_TTL_DAYS`] and requests are spaced by [`REQUEST_DELAY`].
//! EAN codes are not exposed by these endpoints.

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const TOKEN_URL: &str = "https://www.nemlig.com/webapi/Token";
pub const SEARCH_URL: &str = "https://webapi.prod.knl.nemlig.it/searchgateway/api/search";
pub const SITE_URL: &str = "https://www.nemlig.com";
pub const USER_AGENT: &str = "mealplan/0.1 (+https://github.com/lemmih/mealplan)";
pub const CACHE_TTL_DAYS: i64 = 7;
pub const REQUEST_DELAY: Duration = Duration::from_millis(750);

/// Unit of a parsed pack size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PackUnit {
    G,
    Ml,
    Pcs,
}

/// Pack size parsed from Nemlig's free-text `Description` ("280 g / Brand").
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Pack {
    pub qty: f64,
    pub unit: PackUnit,
}

/// Nutrition table from the product label, per 100 g or 100 ml.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LabelNutrition {
    /// `"g"` or `"ml"`, from the "Næringsindhold pr. 100 g" heading.
    pub per_100: String,
    pub kcal: Option<f64>,
    pub fat: Option<f64>,
    pub satfat: Option<f64>,
    pub carb: Option<f64>,
    pub sugar: Option<f64>,
    pub fibre: Option<f64>,
    pub protein: Option<f64>,
    pub salt: Option<f64>,
}

/// One search hit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchHit {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub price_dkk: f64,
    pub unit_price: f64,
    pub unit_price_label: String,
    pub url: String,
    pub in_stock: bool,
}

/// A product page, trimmed to what the catalog needs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NemligProduct {
    pub id: u32,
    pub name: String,
    pub brand: String,
    pub description: String,
    pub pack: Option<Pack>,
    pub price_dkk: f64,
    pub unit_price: f64,
    pub unit_price_label: String,
    pub category: String,
    pub sub_category: String,
    pub labels: Vec<String>,
    /// Nemlig's guaranteed days of shelf life at delivery.
    pub sale_before_last_sales_date: u32,
    pub nutrition: Option<LabelNutrition>,
    pub url: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Cached<T> {
    fetched_at: DateTime<Utc>,
    value: T,
}

/// HTTP client with on-disk cache.
pub struct Client {
    agent: ureq::Agent,
    cache_dir: PathBuf,
    token: Option<String>,
    last_request: Option<std::time::Instant>,
}

impl Client {
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .user_agent(USER_AGENT)
            .timeout_global(Some(Duration::from_secs(30)))
            .build()
            .into();
        Client {
            agent,
            cache_dir: cache_dir.into(),
            token: None,
            last_request: None,
        }
    }

    fn throttle(&mut self) {
        if let Some(t) = self.last_request {
            let elapsed = t.elapsed();
            if elapsed < REQUEST_DELAY {
                std::thread::sleep(REQUEST_DELAY - elapsed);
            }
        }
        self.last_request = Some(std::time::Instant::now());
    }

    fn cache_path(&self, kind: &str, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        self.cache_dir.join(kind).join(format!("{safe}.json"))
    }

    fn read_cache<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Option<T> {
        let text = std::fs::read_to_string(path).ok()?;
        let c: Cached<T> = serde_json::from_str(&text).ok()?;
        let age = Utc::now() - c.fetched_at;
        (age.num_days() < CACHE_TTL_DAYS).then_some(c.value)
    }

    fn write_cache<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let c = Cached {
            fetched_at: Utc::now(),
            value,
        };
        std::fs::write(path, serde_json::to_string_pretty(&c)?)?;
        Ok(())
    }

    fn token(&mut self) -> Result<String> {
        if let Some(t) = &self.token {
            return Ok(t.clone());
        }
        #[derive(Deserialize)]
        struct Tok {
            access_token: String,
        }
        self.throttle();
        let tok: Tok = self
            .agent
            .get(TOKEN_URL)
            .call()
            .context("fetching anonymous token")?
            .body_mut()
            .read_json()
            .context("parsing token")?;
        self.token = Some(tok.access_token.clone());
        Ok(tok.access_token)
    }

    /// Search products by free text. Results are cached per `(query, take)`.
    pub fn search(&mut self, query: &str, take: u32) -> Result<Vec<SearchHit>> {
        let path = self.cache_path("search", &format!("{query}-{take}"));
        if let Some(hits) = self.read_cache::<Vec<SearchHit>>(&path) {
            return Ok(hits);
        }
        let token = self.token()?;
        self.throttle();
        let raw: serde_json::Value = self
            .agent
            .get(SEARCH_URL)
            .query("query", query)
            .query("take", take.to_string())
            .query("skip", "0")
            .query("recipeCount", "0")
            .query("timestamp", "0")
            .query("timeslotUtc", "0")
            .query("deliveryZoneId", "1")
            .query("includeFavorites", "0")
            .header("Authorization", &format!("Bearer {token}"))
            .call()
            .with_context(|| format!("searching nemlig for {query:?}"))?
            .body_mut()
            .read_json()
            .context("parsing search response")?;
        let hits = parse_search(&raw)?;
        self.write_cache(&path, &hits)?;
        Ok(hits)
    }

    /// Resolve the canonical `/{slug}-{id}` path for a product number.
    fn resolve_path(&mut self, id: u32) -> Result<String> {
        let path = self.cache_path("slug", &id.to_string());
        if let Some(p) = self.read_cache::<String>(&path) {
            return Ok(p);
        }
        self.throttle();
        let url = format!("{SITE_URL}/x-{id}");
        let resp = self
            .agent
            .head(&url)
            .config()
            .max_redirects(0)
            .max_redirects_will_error(false)
            .http_status_as_error(false)
            .build()
            .call();
        let location = match resp {
            Ok(r) if r.status().is_redirection() => r
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
                .ok_or_else(|| anyhow!("redirect without location for product {id}"))?,
            Ok(r) => return Err(anyhow!("product {id}: unexpected status {}", r.status())),
            Err(e) => return Err(anyhow!("resolving product {id}: {e}")),
        };
        let location = location.split('?').next().unwrap_or(&location).to_string();
        if !location.ends_with(&format!("-{id}")) {
            return Err(anyhow!("product {id}: unexpected redirect to {location}"));
        }
        self.write_cache(&path, &location)?;
        Ok(location)
    }

    /// Fetch one product page. Cached for [`CACHE_TTL_DAYS`].
    pub fn product(&mut self, id: u32) -> Result<NemligProduct> {
        let path = self.cache_path("product", &id.to_string());
        if let Some(p) = self.read_cache::<NemligProduct>(&path) {
            return Ok(p);
        }
        let slug = self.resolve_path(id)?;
        self.throttle();
        let raw: serde_json::Value = self
            .agent
            .get(format!("{SITE_URL}{slug}"))
            .query("GetAsJson", "1")
            .call()
            .with_context(|| format!("fetching product {id}"))?
            .body_mut()
            .read_json()
            .context("parsing product page")?;
        let p = parse_product(&raw, id)?;
        self.write_cache(&path, &p)?;
        Ok(p)
    }
}

fn parse_search(raw: &serde_json::Value) -> Result<Vec<SearchHit>> {
    let items = raw
        .pointer("/Products/Products")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("search response has no Products.Products array"))?;
    items
        .iter()
        .map(|p| {
            Ok(SearchHit {
                id: str_field(p, "Id")?.parse().context("product Id")?,
                name: str_field(p, "Name")?.to_string(),
                description: p["Description"].as_str().unwrap_or("").to_string(),
                price_dkk: p["Price"]
                    .as_f64()
                    .ok_or_else(|| anyhow!("missing Price"))?,
                unit_price: p["UnitPriceCalc"].as_f64().unwrap_or(0.0),
                unit_price_label: p["UnitPriceLabel"].as_str().unwrap_or("").to_string(),
                url: str_field(p, "Url")?.to_string(),
                in_stock: p
                    .pointer("/Availability/IsAvailableInStock")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn str_field<'a>(v: &'a serde_json::Value, key: &str) -> Result<&'a str> {
    v[key]
        .as_str()
        .ok_or_else(|| anyhow!("missing string field {key}"))
}

fn parse_product(raw: &serde_json::Value, id: u32) -> Result<NemligProduct> {
    let content = raw["content"]
        .as_array()
        .ok_or_else(|| anyhow!("product page has no content array"))?;
    let c = content
        .iter()
        .find(|c| c["TemplateName"].as_str() == Some("productdetailspot"))
        .ok_or_else(|| anyhow!("product {id}: no productdetailspot in page"))?;
    let description = c["Description"].as_str().unwrap_or("").to_string();
    let strings = |key: &str| -> Vec<String> {
        c[key]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    Ok(NemligProduct {
        id,
        name: str_field(c, "Name")?.to_string(),
        brand: c["Brand"].as_str().unwrap_or("").to_string(),
        pack: parse_pack(&description),
        description,
        price_dkk: c["Price"]
            .as_f64()
            .ok_or_else(|| anyhow!("product {id}: missing Price"))?,
        unit_price: c["UnitPriceCalc"].as_f64().unwrap_or(0.0),
        unit_price_label: c["UnitPriceLabel"].as_str().unwrap_or("").to_string(),
        category: c["Category"].as_str().unwrap_or("").to_string(),
        sub_category: c["SubCategory"].as_str().unwrap_or("").to_string(),
        labels: strings("Labels"),
        sale_before_last_sales_date: c["SaleBeforeLastSalesDate"].as_u64().unwrap_or(0) as u32,
        nutrition: c["DeclarationLabel"]
            .as_str()
            .and_then(parse_declaration_label),
        url: format!("{SITE_URL}/{}", c["Url"].as_str().unwrap_or("")),
        fetched_at: Utc::now(),
    })
}

/// Parse the leading "280 g", "1 kg", "0,50 l", "6 stk" of a description.
pub fn parse_pack(description: &str) -> Option<Pack> {
    let first = description.split('/').next()?.trim();
    let mut parts = first.split_whitespace();
    let qty: f64 = parts.next()?.replace(',', ".").parse().ok()?;
    let unit = parts.next()?.trim_end_matches('.').to_ascii_lowercase();
    let (qty, unit) = match unit.as_str() {
        "g" => (qty, PackUnit::G),
        "kg" => (qty * 1000.0, PackUnit::G),
        "ml" => (qty, PackUnit::Ml),
        "cl" => (qty * 10.0, PackUnit::Ml),
        "dl" => (qty * 100.0, PackUnit::Ml),
        "l" | "ltr" => (qty * 1000.0, PackUnit::Ml),
        "stk" | "pk" => (qty, PackUnit::Pcs),
        _ => return None,
    };
    Some(Pack { qty, unit })
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&#230;", "æ")
        .replace("&#248;", "ø")
        .replace("&#229;", "å")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Parse a number like "1.1 g", "459 kJ / 108 kcal" (takes the kcal part),
/// "<0.5 g" or "0,3 g".
fn parse_amount(text: &str, unit: &str) -> Option<f64> {
    let text = text.trim();
    let segment = if unit == "kcal" {
        text.split('/').find(|s| s.contains("kcal"))?
    } else {
        text
    };
    let num: String = segment
        .trim()
        .trim_start_matches('<')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',')
        .collect();
    num.replace(',', ".").parse().ok()
}

/// Parse Nemlig's `DeclarationLabel` HTML nutrition table.
pub fn parse_declaration_label(html: &str) -> Option<LabelNutrition> {
    let mut n = LabelNutrition::default();
    let text = strip_tags(html);
    let per_100 = text.find("pr. 100")?;
    n.per_100 = text[per_100 + 7..]
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect::<String>()
        .to_ascii_lowercase();
    if n.per_100.is_empty() {
        n.per_100 = "g".to_string();
    }
    for row in html.split("<tr>").skip(1) {
        let cells: Vec<String> = row
            .split("<td")
            .skip(1)
            .map(|c| {
                strip_tags(c.split_once('>').map(|x| x.1).unwrap_or(""))
                    .trim()
                    .to_string()
            })
            .collect();
        if cells.len() < 2 {
            continue;
        }
        let (label, value) = (cells[0].to_lowercase(), cells[1].as_str());
        let slot = match label.as_str() {
            l if l.starts_with("energi") => &mut n.kcal,
            l if l.starts_with("fedt") => &mut n.fat,
            l if l.starts_with("heraf mæt") => &mut n.satfat,
            l if l.starts_with("kulhydrat") => &mut n.carb,
            l if l.starts_with("heraf sukker") => &mut n.sugar,
            l if l.starts_with("kostfib") => &mut n.fibre,
            l if l.starts_with("protein") => &mut n.protein,
            l if l.starts_with("salt") => &mut n.salt,
            _ => continue,
        };
        let unit = if label.starts_with("energi") {
            "kcal"
        } else {
            "g"
        };
        *slot = parse_amount(value, unit);
    }
    (n.kcal.is_some() || n.protein.is_some()).then_some(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs() {
        assert_eq!(
            parse_pack("280 g / Gårdkylling / Danske Familiegårde"),
            Some(Pack {
                qty: 280.0,
                unit: PackUnit::G
            })
        );
        assert_eq!(
            parse_pack("1 kg"),
            Some(Pack {
                qty: 1000.0,
                unit: PackUnit::G
            })
        );
        assert_eq!(
            parse_pack("0,50 l / Arla"),
            Some(Pack {
                qty: 500.0,
                unit: PackUnit::Ml
            })
        );
        assert_eq!(
            parse_pack("6 stk"),
            Some(Pack {
                qty: 6.0,
                unit: PackUnit::Pcs
            })
        );
        assert_eq!(parse_pack("Simply Organics"), None);
    }

    #[test]
    fn declaration_label() {
        let html = "<p>Fersk kyllingebrystfilet.</p><table><thead><tr><td align=\"left\"><b>N&#230;ringsindhold pr. 100 g</b></td><td></td></tr></thead><tbody><tr><td align=\"left\">Energi</td><td align=\"right\">459 kJ / 108 kcal\n</td></tr><tr><td>Fedt</td><td>1.1 g\n</td></tr><tr><td>Heraf m&#230;ttede fedtsyrer</td><td>0.3 g</td></tr><tr><td>Kulhydrater</td><td>0 g</td></tr><tr><td>Heraf sukkerarter</td><td></td></tr><tr><td>Kostfibre</td><td>&lt;0.5 g</td></tr><tr><td>Protein</td><td>24 g</td></tr><tr><td>Salt</td><td>0,13 g</td></tr></tbody></table>";
        let n = parse_declaration_label(html).unwrap();
        assert_eq!(n.per_100, "g");
        assert_eq!(n.kcal, Some(108.0));
        assert_eq!(n.fat, Some(1.1));
        assert_eq!(n.satfat, Some(0.3));
        assert_eq!(n.carb, Some(0.0));
        assert_eq!(n.sugar, None);
        assert_eq!(n.fibre, Some(0.5));
        assert_eq!(n.protein, Some(24.0));
        assert_eq!(n.salt, Some(0.13));
    }

    #[test]
    fn declaration_label_ml() {
        let html = "<table><tr><td>N&#230;ringsindhold pr. 100 ml</td><td></td></tr><tr><td>Energi</td><td>455 kJ / 109 kcal</td></tr><tr><td>Protein</td><td>3.4 g</td></tr></table>";
        let n = parse_declaration_label(html).unwrap();
        assert_eq!(n.per_100, "ml");
        assert_eq!(n.kcal, Some(109.0));
    }
}
