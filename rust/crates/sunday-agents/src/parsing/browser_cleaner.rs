//! Smart content cleaner for browser text extraction.
//! Strips common UI noise (navigation, filters, calendar grids, etc.)

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashSet;

// Pre-compiled regex patterns
static CALENDAR_DAY_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{1,2}$").unwrap());
static HOTEL_TYPE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(Hotels|Apartments|Villas|Hostels|Ryokans|Guest houses|Chalets|Capsule hotels|Homestays|Holiday homes|Love hotels)\s*$").unwrap()
});
static SHORT_NUMBER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d{1,4}$").unwrap());
static STARS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+ stars?$").unwrap());
static RATING_LABEL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(Superb|Very good|Good|Pleasant): \d\+$").unwrap()
});
static SCORED_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"Scored \d+\.\d+").unwrap());
static RATING_BLOCK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d\.\d\n.+\n(Superb|Very good|Good|Exceptional)").unwrap()
});
static PRICE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(AUD|USD|JPY|EUR|฿|¥)\s*[\d,]+").unwrap()
});
static LISTING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"properties found|Opens in new window").unwrap()
});

// Static sets for fast lookups
fn noise_exact_set() -> &'static HashSet<&'static str> {
    static SET: Lazy<HashSet<&str>> = Lazy::new(|| {
        [
            "Skip to main content", "List your property", "Register",
            "Sign in", "Stays", "Flights", "Flight + Hotel", "Car rental",
            "Attractions", "Airport taxis", "Home", "Show on map",
            "Filter by:", "Popular filters", "Smart filters",
            "What are you looking for?", "Find properties",
            "Property type", "Property rating", "Facilities",
            "Room facilities", "Review score", "Neighbourhood",
            "Travel group", "Brands", "Fun things to do",
            "Entire places", "Certifications", "Property accessibility",
            "Room accessibility", "Show all 13", "Show all 14",
            "Show all 20", "Show all 25", "List", "Grid",
            "Find high-quality hotels and holiday rentals",
            "I'm travelling for work", "Check-in date", "Check-out date",
        ].iter().cloned().collect()
    });
    &SET
}

fn calendar_reset_set() -> &'static HashSet<&'static str> {
    static SET: Lazy<HashSet<&str>> = Lazy::new(|| {
        ["1 day", "2 days", "3 days", "7 days"].iter().cloned().collect()
    });
    &SET
}

fn currency_set() -> &'static HashSet<&'static str> {
    static SET: Lazy<HashSet<&str>> = Lazy::new(|| {
        ["AUD", "USD", "EUR", "GBP", "JPY"].iter().cloned().collect()
    });
    &SET
}

fn accessibility_keywords() -> &'static [&'static str] {
    static KW: &[&str] = &[
        "wheelchair", "grab rails", "tactile signs", "braille",
        "shower chair", "roll-in shower", "lowered sink", "raised toilet",
        "emergency cord", "elevator", "ground floor", "adapted bath",
        "walk-in shower", "auditory guidance", "sustainability",
        "massage chair", "bicycle rental", "entire homes",
        "pets allowed", "adults only", "lgbtq",
    ];
    &KW
}

fn brand_names() -> &'static [&'static str] {
    static BRANDS: &[&str] = &[
        "APA Hotels", "Tokyu Stay", "LiveMax", "WHG HOTELS",
        "Citadines", "Pan Pacific", "Toyoko Inn", "Hilton",
        "Daiwa Roynet", "Iconia",
    ];
    &BRANDS
}

/// Clean browser-extracted text by removing UI noise.
/// Returns (cleaned_text, has_ratings, has_prices, has_listings).
pub fn clean_browser_text(text: &str) -> (String, bool, bool, bool) {
    let noise_exact = noise_exact_set();
    let calendar_reset = calendar_reset_set();
    let currencies = currency_set();
    let access_kw = accessibility_keywords();
    let brands = brand_names();

    let lines: Vec<&str> = text.split('\n').collect();
    let mut cleaned = Vec::with_capacity(lines.len());
    let mut skip_mode = false;

    for line in &lines {
        let stripped = line.trim();
        if stripped.is_empty() {
            continue;
        }

        // Calendar grid skip mode
        if stripped == "Su" || stripped == "Mo" || stripped == "Tu"
            || stripped == "We" || stripped == "Th" || stripped == "Fr" || stripped == "Sa" {
            skip_mode = true;
            continue;
        }
        if skip_mode && CALENDAR_DAY_RE.is_match(stripped) {
            continue;
        }
        if skip_mode && (stripped == "Calendar" || stripped == "I'm flexible" || stripped == "Exact dates") {
            continue;
        }
        if stripped == "days" || calendar_reset.contains(stripped) {
            skip_mode = false;
            continue;
        }

        // Exact noise matches
        if noise_exact.contains(stripped) {
            continue;
        }

        // Hotel type filter counts
        if HOTEL_TYPE_RE.is_match(stripped) {
            continue;
        }
        // Short numbers (filter counts)
        if SHORT_NUMBER_RE.is_match(stripped) && stripped.len() <= 4 {
            continue;
        }
        // Star ratings
        if STARS_RE.is_match(stripped) {
            continue;
        }
        // Rating labels like "Superb: 9+"
        if RATING_LABEL_RE.is_match(stripped) {
            continue;
        }

        // Accessibility/facility keywords
        let lower = stripped.to_lowercase();
        if access_kw.iter().any(|kw| lower.contains(kw)) {
            continue;
        }

        // Brand names
        if brands.iter().any(|brand| stripped.contains(brand)) {
            continue;
        }

        // Currency buttons
        if currencies.contains(stripped) {
            continue;
        }

        skip_mode = false;
        cleaned.push(stripped);
    }

    let content = cleaned.join("\n");

    // Detect structured data patterns
    let has_ratings = SCORED_RE.is_match(&content) || RATING_BLOCK_RE.is_match(&content);
    let has_prices = PRICE_RE.is_match(&content);
    let has_listings = LISTING_RE.is_match(&content);

    (content, has_ratings, has_prices, has_listings)
}
