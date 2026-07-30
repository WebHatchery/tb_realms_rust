//! Typed state deltas into price shocks (GDD 5.5), driven by `impacts.json`.
//!
//! **The price oracle reads numbers, never prose.** Mytherra's chronicle is
//! `{ year, kind, message }` with four coarse kinds and free text for substance.
//! Parsing that for market signal would break silently the first time a string
//! was reworded, so every rule here keys on a typed [`FieldMove`] between
//! consecutive world views instead. The chronicle is rendered beside the numbers
//! as the newspaper and nothing in this file may read it.
//!
//! **Shocks travel.** A resource node is not a listing, so its status change
//! would price nothing at all if a shock could only land on the entity it
//! happened to; a famine is a fact about a province that ought to reach the
//! towns inside it. So a rule declares which way it travels along the world's
//! own parent link ([`Fundamentals::parent`]): to the moved entity itself, up to
//! its parent, or down to its children.
//!
//! Levels are not this file's business. A node depleting lowers the commodity's
//! `nodes_producing` in `instruments.json`, which reaches price as drift; the
//! rule here is the *jolt*. Both on purpose: the jolt is immediate and the
//! revaluation is permanent, and a player who only sees one of them is being
//! told half the story.

use crate::world::delta::{Change, WorldDelta};
use crate::world::fundamentals::{EntityRef, Field, Snapshot};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Whose move a rule is about, relative to the listing it shocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Via {
    /// The listing's own entity moved.
    #[default]
    SelfEntity,
    /// The listing is the moved entity's parent — a node's status reaching the
    /// commodity above it.
    Parent,
    /// The listing is a child of the moved entity — a province's famine
    /// reaching the towns inside it.
    Child,
}

/// One line of the balance surface.
#[derive(Debug, Clone, Deserialize)]
pub struct ImpactRule {
    pub field: String,
    pub class: String,
    #[serde(default)]
    pub via: Via,
    /// Scalar moves: shock per unit of change. Positive means the price moves
    /// with the field.
    #[serde(default)]
    pub per_unit: f32,
    /// Scalar moves: the largest shock this one rule may produce, however far
    /// the field moved. Without it a single catastrophic year could swamp every
    /// other rule in the table.
    #[serde(default = "default_cap")]
    pub cap: f32,
    /// Flag and status moves: the reading that fires the rule — `"true"` /
    /// `"false"`, or the world's own variant name.
    #[serde(default)]
    pub when: Option<String>,
    /// Flag and status moves: the flat shock when it fires.
    #[serde(default)]
    pub shock: f32,
}

fn default_cap() -> f32 {
    0.25
}

/// The whole table.
#[derive(Debug, Clone, Deserialize)]
pub struct ImpactTable {
    /// The most all rules together may shock one listing in one settlement.
    total_cap: f32,
    rules: Vec<ImpactRule>,
}

impl ImpactTable {
    /// Check the table against the field vocabulary and the instrument
    /// taxonomy, so a rule that could never fire is a load error rather than a
    /// silence nobody notices. A mistyped field key is the failure this catches:
    /// it looks exactly like a balance value set to zero.
    pub fn validate(&self) -> Result<(), String> {
        if self.total_cap <= 0.0 {
            return Err("impacts.json has a total_cap of zero: no shock could land".to_owned());
        }
        for rule in &self.rules {
            if Field::from_key(&rule.field).is_none() {
                return Err(format!(
                    "impacts.json shocks on '{}', which is not a world field",
                    rule.field
                ));
            }
            if !EntityRef::LISTING_CLASSES.contains(&rule.class.as_str()) {
                return Err(format!(
                    "impacts.json shocks '{}', which is not an instrument class",
                    rule.class
                ));
            }
            if rule.when.is_none() && rule.per_unit == 0.0 {
                return Err(format!(
                    "impacts.json rule on '{}' for '{}' can never fire: no 'when' and no 'per_unit'",
                    rule.field, rule.class
                ));
            }
            if rule.cap < 0.0 {
                return Err(format!(
                    "impacts.json rule on '{}' has a negative cap",
                    rule.field
                ));
            }
        }
        Ok(())
    }

    /// What one turning of the world does to every listing it touches, as a
    /// fraction of each price.
    ///
    /// Listings not mentioned are not in the result at all — a quiet year should
    /// produce a short map, not a long one full of zeroes.
    pub fn shocks(&self, delta: &WorldDelta, snapshot: &Snapshot) -> BTreeMap<EntityRef, f32> {
        let children = child_index(snapshot);
        let mut shocks: BTreeMap<EntityRef, f32> = BTreeMap::new();

        for moved in &delta.moves {
            let parent = snapshot
                .get(&moved.entity)
                .and_then(|entry| entry.parent.clone());
            let no_children = Vec::new();
            let below = children.get(&moved.entity).unwrap_or(&no_children);

            let targets = std::iter::once((Via::SelfEntity, &moved.entity))
                .chain(parent.iter().map(|up| (Via::Parent, up)))
                .chain(below.iter().map(|down| (Via::Child, down)));

            for (via, target) in targets {
                let Some(class) = target.listing_class() else {
                    continue;
                };
                let shock = self.shock_for(&moved.field, &moved.change, class, via);
                if shock != 0.0 {
                    *shocks.entry(target.clone()).or_default() += shock;
                }
            }
        }

        for shock in shocks.values_mut() {
            *shock = shock.clamp(-self.total_cap, self.total_cap);
        }
        shocks
    }

    /// Every rule matching one move, summed.
    fn shock_for(&self, field: &Field, change: &Change, class: &str, via: Via) -> f32 {
        self.rules
            .iter()
            .filter(|rule| rule.class == class && rule.via == via && rule.field == field.key())
            .map(|rule| fires(rule, change))
            .sum()
    }
}

/// What one rule yields for one change, or zero if it does not apply.
fn fires(rule: &ImpactRule, change: &Change) -> f32 {
    match change {
        // A scalar rule is the one kind with no `when`: it fires on any move and
        // scales with how far the field went.
        Change::Scalar { .. } if rule.when.is_none() => {
            (rule.per_unit * change.magnitude()).clamp(-rule.cap, rule.cap)
        }
        Change::Flag { after, .. } => match rule.when.as_deref() {
            Some(when) if when == after.to_string() => rule.shock,
            _ => 0.0,
        },
        Change::Status { after, .. } => match rule.when.as_deref() {
            Some(when) if when == after => rule.shock,
            _ => 0.0,
        },
        Change::Scalar { .. } => 0.0,
    }
}

/// Entities grouped by the entity they belong to, so a shock can travel
/// downward. Built per settlement rather than kept, because the roster it
/// describes changes every time the world turns.
fn child_index(snapshot: &Snapshot) -> BTreeMap<EntityRef, Vec<EntityRef>> {
    let mut index: BTreeMap<EntityRef, Vec<EntityRef>> = BTreeMap::new();
    for entry in snapshot.iter() {
        if let Some(parent) = &entry.parent {
            index
                .entry(parent.clone())
                .or_default()
                .push(entry.entity.clone());
        }
    }
    index
}

#[cfg(test)]
mod tests;
