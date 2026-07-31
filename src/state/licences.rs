//! Broker licences: what this exchange lets you see (GDD 5.3).
//!
//! **Information is the progression axis, not capital** (Pillar 3). A rich
//! broker with poor information should lose to a poorer one who can see
//! further, so what a licence buys is sight: which classes of paper appear on
//! the board at all, which entities the research browser will show, and how far
//! back the chronicle may be read.
//!
//! # An honest note about the gate
//!
//! GDD 5.3 maps these tiers onto Mytherra's `VisibilityScope`, and they are
//! named against it here. But the spectator projection this game reads already
//! carries *every* scope — that is what makes the endpoint safe to expose — so
//! a licence cannot make the exchange unsee what the server already sent. This
//! gate is the exchange's own: it decides what a broker of this rank is shown
//! and permitted to trade.
//!
//! That is a real distinction and worth stating plainly rather than implying a
//! server-side filter that is not there. A Solo world source, ticking a world
//! in-process, *could* enforce the scopes for real; the tiers are shaped so that
//! it would slot in without redesign.

use serde::{Deserialize, Serialize};

/// One rung of the ladder, as `licences.json` states it.
#[derive(Debug, Clone, Deserialize)]
pub struct Tier {
    pub id: String,
    pub name: String,
    /// Crowns to buy this rung. The first is free — a broker starts on the floor.
    pub cost: f32,
    /// The Mytherra visibility scopes this rung corresponds to, by their
    /// serialized names. Validated at load so a typo cannot invent a scope.
    pub scopes: Vec<String>,
    /// Instrument classes this rung unlocks.
    pub classes: Vec<String>,
    /// What it lets you see, in a phrase.
    pub reveals: String,
    pub blurb: String,
}

/// The ladder.
#[derive(Debug, Clone, Deserialize)]
pub struct LicenceTable {
    tiers: Vec<Tier>,
}

/// Every scope name Mytherra defines, as serde spells them. Kept as strings
/// rather than the enum because this is a *validation* list: the point is to
/// catch a typo in data, and the enum is not `FromStr`.
const SCOPES: [&str; 9] = [
    "heroes",
    "observatory",
    "regions",
    "settlements",
    "resources",
    "divine_tools",
    "pantheon",
    "eras",
    "full_chronicle",
];

impl LicenceTable {
    /// Check the ladder covers every class exactly once and names only real
    /// scopes.
    ///
    /// A class under no tier would be paper no licence could ever reach, and a
    /// class under two would make the ladder's order meaningless. Both are
    /// configuration errors worth refusing to open the exchange over.
    pub fn validate(&self) -> Result<(), String> {
        if self.tiers.is_empty() {
            return Err("licences.json defines no tiers".to_owned());
        }
        if self.tiers[0].cost != 0.0 {
            return Err("the first licence must be free — a broker starts on the floor".to_owned());
        }
        for class in crate::world::fundamentals::EntityRef::LISTING_CLASSES {
            let count = self
                .tiers
                .iter()
                .filter(|tier| tier.classes.iter().any(|held| held == class))
                .count();
            match count {
                1 => {}
                0 => return Err(format!("no licence ever unlocks '{class}'")),
                n => return Err(format!("'{class}' is unlocked by {n} licences")),
            }
        }
        for tier in &self.tiers {
            for class in &tier.classes {
                if !crate::world::fundamentals::EntityRef::LISTING_CLASSES.contains(&class.as_str())
                {
                    return Err(format!(
                        "licence '{}' unlocks '{class}', which is not an instrument class",
                        tier.id
                    ));
                }
            }
            for scope in &tier.scopes {
                if !SCOPES.contains(&scope.as_str()) {
                    return Err(format!(
                        "licence '{}' names '{scope}', which is not a visibility scope",
                        tier.id
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn tiers(&self) -> &[Tier] {
        &self.tiers
    }

    /// The rung a broker holding `held` rungs is standing on.
    pub fn current(&self, held: usize) -> &Tier {
        let index = held.min(self.tiers.len() - 1);
        &self.tiers[index]
    }

    /// The next rung up, and what it costs, or `None` at the top.
    pub fn next(&self, held: usize) -> Option<&Tier> {
        self.tiers.get(held + 1)
    }

    /// Whether a broker holding `held` rungs may trade this class.
    pub fn permits(&self, held: usize, class: &str) -> bool {
        self.tiers
            .iter()
            .take(held + 1)
            .any(|tier| tier.classes.iter().any(|open| open == class))
    }

    /// Whether the top rung — the one that opens the archive — is held.
    ///
    /// The chronicle's depth and the era history hang off this rather than off a
    /// class list, because what an Archivist buys is *history*, and history is
    /// not an instrument.
    pub fn is_archivist(&self, held: usize) -> bool {
        held + 1 >= self.tiers.len()
    }

    /// The highest rung a broker could reach, for the records screen.
    pub fn top(&self) -> usize {
        self.tiers.len() - 1
    }
}

/// Why a licence could not be bought.
#[derive(Debug, Clone, PartialEq)]
pub enum LicenceError {
    /// Already at the top of the ladder.
    Complete,
    /// Not enough crowns.
    Insufficient { need: f32, have: f32 },
}

impl LicenceError {
    pub fn label(&self) -> String {
        match self {
            LicenceError::Complete => "there is nothing further to be shown".to_owned(),
            LicenceError::Insufficient { need, have } => {
                format!("that licence costs {need:.0}; the ledger holds {have:.0}")
            }
        }
    }
}

/// How many rungs the broker has climbed. Saved: a licence is bought once and
/// is the one piece of progression the world cannot rebuild.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Licences {
    /// Index of the highest tier held. Zero is the free rung everyone starts on.
    pub held: usize,
}

#[cfg(test)]
mod tests;
