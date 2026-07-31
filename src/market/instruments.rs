//! The instrument taxonomy and what each listing is fundamentally worth
//! (GDD 5.2, 5.4).
//!
//! Every listing is a claim on something that actually exists. A [`Board`] is
//! derived wholly from a [`Snapshot`] of the world: an entity the world carries
//! is listed, an entity it stops carrying is delisted, and the rest revalue.
//! Nothing here invents a security, which is Pillar 1 made mechanical — ask why
//! a listing is worth what it is and the answer is a named entity and a rule you
//! can read in `instruments.json`.
//!
//! **Fair value is not price.** It is what the fundamentals say the claim is
//! worth; the price M2 introduces drifts toward it and rarely arrives. Skill is
//! estimating this number before the market's drift closes the gap.

use crate::market::pricing::{self, MarketConfig, PriceMove};
use crate::world::fundamentals::{EntityRef, Field, Fundamentals, Snapshot};
use macroquad_toolkit::rng::SeededRng;
use macroquad_toolkit::series::Series;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// A weighted fundamental in a fair-value rule.
#[derive(Debug, Clone, Deserialize)]
pub struct Term {
    pub field: String,
    pub weight: f32,
}

/// A multiplier applied while a flag or status reads a given way.
#[derive(Debug, Clone, Deserialize)]
pub struct Modifier {
    pub field: String,
    /// The world's own variant name for a status, or `true`/`false` for a flag.
    pub when: String,
    pub factor: f32,
}

/// What a listing pays its holder each settlement (GDD 5.2 — yield is real
/// income, never a decorative ratio).
#[derive(Debug, Clone, Deserialize)]
pub struct Income {
    pub field: String,
    pub rate: f32,
}

/// One instrument class's rule, as `instruments.json` states it.
#[derive(Debug, Clone, Deserialize)]
pub struct ClassRule {
    pub id: String,
    pub label: String,
    pub ticker_prefix: String,
    pub base_value: f32,
    pub floor_value: f32,
    pub terms: Vec<Term>,
    /// Terms contributing `weight / max(fundamental, 1)` — for claims that grow
    /// dearer as the thing behind them grows rarer. A commodity prices its
    /// supply shocks entirely through these: fewer nodes still yielding, dearer
    /// the grain.
    pub scarcity: Vec<Term>,
    pub modifiers: Vec<Modifier>,
    pub volatility: f32,
    pub liquidity: f32,
    pub income: Option<Income>,
}

/// The taxonomy as loaded, with every field key already resolved.
#[derive(Debug, Clone, Deserialize)]
pub struct InstrumentRules {
    classes: Vec<ClassRule>,
}

impl InstrumentRules {
    /// Check a freshly parsed taxonomy against the closed set of listing classes
    /// and the field vocabulary.
    ///
    /// Both checks are the no-orphan-instrument rule of GDD 14 enforced at load:
    /// a class with no rule would be an entity nobody could ever trade, and a
    /// rule naming a field the world does not carry would price on a number that
    /// is always zero. Either is a configuration error worth failing loudly for,
    /// rather than a listing that quietly never appears.
    pub fn validate(&self) -> Result<(), String> {
        for class in EntityRef::LISTING_CLASSES {
            let matches = self.classes.iter().filter(|rule| rule.id == class).count();
            match matches {
                1 => {}
                0 => return Err(format!("instruments.json has no rule for '{class}'")),
                n => return Err(format!("instruments.json has {n} rules for '{class}'")),
            }
        }
        for rule in &self.classes {
            if !EntityRef::LISTING_CLASSES.contains(&rule.id.as_str()) {
                return Err(format!(
                    "instruments.json defines '{}', which is not an instrument class",
                    rule.id
                ));
            }
            if rule.ticker_prefix.is_empty() {
                return Err(format!("'{}' has no ticker prefix", rule.id));
            }
            let keys = rule
                .terms
                .iter()
                .chain(&rule.scarcity)
                .map(|term| &term.field)
                .chain(rule.modifiers.iter().map(|m| &m.field))
                .chain(rule.income.iter().map(|income| &income.field));
            for key in keys {
                if Field::from_key(key).is_none() {
                    return Err(format!(
                        "'{}' prices on '{key}', which is not a world field",
                        rule.id
                    ));
                }
            }
        }
        Ok(())
    }

    /// The rule for an entity's class, or `None` for an entity that is watched
    /// but never listed.
    pub fn for_entity(&self, entity: &EntityRef) -> Option<&ClassRule> {
        let class = entity.listing_class()?;
        self.classes.iter().find(|rule| rule.id == class)
    }
}

/// A tradeable claim on one world entity.
#[derive(Debug, Clone)]
pub struct Instrument {
    /// The board's short name for it, unique across the board.
    pub symbol: String,
    /// The entity's name in the world's own words.
    pub name: String,
    pub entity: EntityRef,
    /// The class label as the boards say it.
    pub class: String,
    /// What the fundamentals say the claim is worth, this settlement.
    pub fair_value: f32,
    /// What it actually trades at. A listing is born priced fairly and is
    /// almost never priced fairly again — the gap between this and
    /// `fair_value` is the entire game (GDD 5.4).
    pub price: f32,
    /// Where the price stood before the last turning.
    pub previous_close: f32,
    /// What holding one unit pays each settlement.
    pub income: f32,
    pub volatility: f32,
    pub liquidity: f32,
    /// Every price this listing has ever traded at, bounded but never losing an
    /// extreme — the spike that mattered survives a century of decimation.
    pub history: Series,
    /// Why the price last moved, decomposed (GDD 5.4).
    pub last_move: PriceMove,
    /// The world year the listing first appeared.
    pub listed_year: u32,
}

impl Instrument {
    /// Income as a fraction of fair value — the fantasy-native yield, computed
    /// from real per-settlement income rather than declared (GDD 0).
    pub fn yield_rate(&self) -> f32 {
        if self.fair_value <= 0.0 {
            return 0.0;
        }
        self.income / self.fair_value
    }

    /// How far the price sits from what the fundamentals say, as a fraction.
    /// Positive means the floor is paying over the odds. This is the number a
    /// broker is actually trading.
    pub fn premium(&self) -> f32 {
        if self.fair_value.abs() < f32::EPSILON {
            return 0.0;
        }
        (self.price - self.fair_value) / self.fair_value
    }
}

/// What one refresh of the board did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BoardChange {
    /// Symbols that appeared, because the world founded the thing behind them.
    pub listed: Vec<String>,
    /// Symbols that vanished, because the world lost the thing behind them.
    pub delisted: Vec<String>,
    /// Listings whose fair value moved.
    pub revalued: usize,
}

/// Fair value moves below this are not worth reporting as a revaluation — the
/// same reasoning as the delta's scalar epsilon, one level up.
const REVALUE_EPSILON: f32 = 0.001;

/// Everything currently listed on the exchange.
#[derive(Debug, Clone, Default)]
pub struct Board {
    listings: BTreeMap<EntityRef, Instrument>,
    /// Symbols in use, so no two listings can share one.
    symbols: BTreeSet<String>,
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bring the board in line with the world.
    ///
    /// The world decides what is listed, not the exchange: an entity the
    /// snapshot carries and the taxonomy can price becomes a listing, an entity
    /// it no longer carries is struck off. A delisted instrument is *gone* from
    /// the board — its history belongs to the records archive, which is what an
    /// Archivist licence is for (GDD 12, open question 6).
    pub fn refresh(
        &mut self,
        snapshot: &Snapshot,
        rules: &InstrumentRules,
        market: &MarketConfig,
    ) -> BoardChange {
        let mut change = BoardChange::default();

        for entry in snapshot.iter() {
            let Some(rule) = rules.for_entity(&entry.entity) else {
                continue;
            };
            let fair_value = fair_value(rule, entry);
            let income = income(rule, entry);

            match self.listings.get_mut(&entry.entity) {
                Some(listing) => {
                    if (listing.fair_value - fair_value).abs() > REVALUE_EPSILON {
                        change.revalued += 1;
                    }
                    // The world can rename a thing; the ticker outlives it, the
                    // way a real one outlives a change of letterhead.
                    listing.name.clone_from(&entry.name);
                    listing.fair_value = fair_value;
                    listing.income = income;
                }
                None => {
                    let symbol = self.mint_symbol(&rule.ticker_prefix, &entry.name);
                    change.listed.push(symbol.clone());
                    // A listing opens at fair value — the one moment in its life
                    // the floor and the fundamentals agree — and starts drifting
                    // apart on the next turning.
                    let price = fair_value.max(market.min_price);
                    let mut history = Series::new(market.history_capacity);
                    history.push(price);
                    self.listings.insert(
                        entry.entity.clone(),
                        Instrument {
                            symbol,
                            name: entry.name.clone(),
                            entity: entry.entity.clone(),
                            class: rule.label.clone(),
                            fair_value,
                            price,
                            previous_close: price,
                            income,
                            volatility: rule.volatility,
                            liquidity: rule.liquidity,
                            history,
                            last_move: PriceMove::unmoved(price),
                            listed_year: snapshot.year,
                        },
                    );
                }
            }
        }

        self.listings.retain(|entity, listing| {
            if snapshot.get(entity).is_some() {
                return true;
            }
            change.delisted.push(listing.symbol.clone());
            false
        });
        for symbol in &change.delisted {
            self.symbols.remove(symbol);
        }

        change
    }

    /// Move every price by one settlement (GDD 5.4).
    ///
    /// `shocks` and `flow` are keyed by entity and carry fractions of price; a
    /// listing absent from either simply takes no such term. Listings are
    /// settled in the board's own stable order and share one noise stream, so
    /// the same seed and the same orders reproduce the same century exactly —
    /// which is what the determinism test of GDD 14 asserts.
    pub fn settle(
        &mut self,
        shocks: &BTreeMap<EntityRef, f32>,
        flow: &BTreeMap<EntityRef, f32>,
        market: &MarketConfig,
        rng: &mut SeededRng,
    ) {
        for (entity, listing) in &mut self.listings {
            let moved = pricing::settle(
                listing.price,
                listing.fair_value,
                listing.volatility,
                shocks.get(entity).copied().unwrap_or(0.0),
                flow.get(entity).copied().unwrap_or(0.0),
                market,
                rng,
            );
            listing.previous_close = moved.from;
            listing.price = moved.to;
            listing.last_move = moved;
            listing.history.push(moved.to);
        }
    }

    /// Every listing, in a stable order.
    pub fn listings(&self) -> impl Iterator<Item = &Instrument> {
        self.listings.values()
    }

    /// One listing, if the world still backs it. A position whose entity is no
    /// longer here is a position in something that has ceased to exist.
    pub fn get(&self, entity: &EntityRef) -> Option<&Instrument> {
        self.listings.get(entity)
    }

    pub fn len(&self) -> usize {
        self.listings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.listings.is_empty()
    }

    /// A free ticker for a new listing, disambiguated if the letters are taken.
    fn mint_symbol(&mut self, prefix: &str, name: &str) -> String {
        let base = format!("{}{}", prefix.to_uppercase(), ticker_core(name));
        let mut symbol = base.clone();
        let mut suffix = 2;
        while self.symbols.contains(&symbol) {
            symbol = format!("{base}{suffix}");
            suffix += 1;
        }
        self.symbols.insert(symbol.clone());
        symbol
    }
}

/// Words that say what kind of thing something is rather than which one, so
/// they make poor tickers — every house would otherwise abbreviate to `HOU`.
const TICKER_STOPWORDS: [&str; 7] = ["THE", "OF", "A", "AN", "AND", "HOUSE", "ORDER"];

/// Three letters standing for a name: the initials of its significant words,
/// or its own first letters when there is only one.
fn ticker_core(name: &str) -> String {
    let words: Vec<String> = name
        .split_whitespace()
        .map(|word| {
            word.chars()
                .filter(char::is_ascii_alphanumeric)
                .collect::<String>()
                .to_uppercase()
        })
        .filter(|word| !word.is_empty() && !TICKER_STOPWORDS.contains(&word.as_str()))
        .collect();

    let Some(last) = words.last() else {
        return "XXX".to_owned();
    };
    let mut core: String = if words.len() == 1 {
        last.chars().take(3).collect()
    } else {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .collect()
    };
    core.truncate(3);
    // Two significant words give only two initials; fill from the last word so
    // "The Iron Road" reads TIRO rather than TIRX.
    for ch in last.chars().skip(1) {
        if core.len() >= 3 {
            break;
        }
        core.push(ch);
    }
    while core.len() < 3 {
        core.push('X');
    }
    core
}

/// What the fundamentals say a claim is worth (GDD 5.4's `fair_value` term).
///
/// A fundamental a rule names but the entity does not carry contributes
/// nothing rather than failing — the taxonomy is validated against the field
/// vocabulary at load, so the only way here is an entity that genuinely lacks
/// the number.
fn fair_value(rule: &ClassRule, entry: &Fundamentals) -> f32 {
    let mut value = rule.base_value;
    for term in &rule.terms {
        value += term.weight * read(entry, &term.field);
    }
    for term in &rule.scarcity {
        value += term.weight / read(entry, &term.field).max(1.0);
    }
    value = value.max(rule.floor_value);

    for modifier in &rule.modifiers {
        if meets(entry, modifier) {
            value *= modifier.factor;
        }
    }
    value.max(0.0)
}

fn income(rule: &ClassRule, entry: &Fundamentals) -> f32 {
    match &rule.income {
        Some(income) => (income.rate * read(entry, &income.field)).max(0.0),
        None => 0.0,
    }
}

fn read(entry: &Fundamentals, key: &str) -> f32 {
    Field::from_key(key)
        .and_then(|field| entry.scalar(field))
        .unwrap_or(0.0)
}

/// Whether an entity currently reads the way a modifier's condition names.
fn meets(entry: &Fundamentals, modifier: &Modifier) -> bool {
    let Some(field) = Field::from_key(&modifier.field) else {
        return false;
    };
    if let Some(flag) = entry.flag(field) {
        return modifier.when == flag.to_string();
    }
    entry.status(field) == Some(modifier.when.as_str())
}

#[cfg(test)]
mod tests;
