//! Typed differences between two consecutive world views (GDD 5.5).
//!
//! **The price oracle reads numbers, never prose.** Mytherra's `WorldEvent` is
//! `{ year, kind, message }` with four coarse kinds and free text for substance;
//! parsing that text for market signal would break silently the first time a
//! string was reworded. So the exchange diffs the *state* between consecutive
//! [`WorldView`]s and prices the result: a region's `harvest` fell thirty
//! points and `famine` flipped true, a trade route's `volume` halved, a resource
//! node's status changed, a house was founded or its line ran out.
//!
//! The chronicle rides alongside in [`WorldDelta::events`] as the newspaper —
//! rendered for the player, never read by the pricer. That split is what makes
//! GDD Pillar 4 mechanical: the prose is flavour and the deltas are truth.
//!
//! Everything here is pure. No macroquad, no I/O, no engine types — so a
//! century of deltas can be replayed headlessly by the backtest harness
//! (GDD 14).

use mytherra_core::world::{
    Hero, House, Order, Region, ResourceNode, Settlement, TradeRoute, WorldEvent,
};
use mytherra_protocol::WorldView;
use std::collections::BTreeMap;

/// Scalar moves smaller than this are treated as noise and dropped.
///
/// The world's stats are `f32` and several are recomputed from sums each tick,
/// so an untouched value can wobble in its last bits. A change below this
/// threshold is not something a market could price anyway, and admitting it
/// would fill every settlement's decomposition with meaningless rows.
const SCALAR_EPSILON: f32 = 0.001;

/// A world entity an instrument can be claimed on (GDD 5.2).
///
/// Each variant carries the entity's world id. Heroes are deliberately absent:
/// no instrument is claimed on a single hero — heroes move prices *through* the
/// houses and orders they belong to, which is what makes house scrip a bet on a
/// bloodline rather than on one life.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityRef {
    House(String),
    Order(String),
    Region(String),
    Settlement(String),
    TradeRoute(String),
    ResourceNode(String),
}

impl EntityRef {
    /// The entity's world id.
    pub fn id(&self) -> &str {
        match self {
            EntityRef::House(id)
            | EntityRef::Order(id)
            | EntityRef::Region(id)
            | EntityRef::Settlement(id)
            | EntityRef::TradeRoute(id)
            | EntityRef::ResourceNode(id) => id,
        }
    }

    /// The instrument class this entity backs, as it appears on the board.
    pub fn class(&self) -> &'static str {
        match self {
            EntityRef::House(_) => "house scrip",
            EntityRef::Order(_) => "order charter",
            EntityRef::Region(_) => "regional bond",
            EntityRef::Settlement(_) => "settlement share",
            EntityRef::TradeRoute(_) => "route toll",
            EntityRef::ResourceNode(_) => "commodity",
        }
    }
}

/// A world field the exchange watches.
///
/// This is the vocabulary `impacts.json` will key on (GDD 6), so the variants
/// are named for the world's fields rather than for any market interpretation
/// of them — the mapping from "harvest fell" to "grain moved" belongs in data,
/// not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Field {
    // Houses and Orders.
    Prestige,
    /// Members still drawing breath: a house's living heirs, an order's living
    /// fellowship. This is what prestige drifts toward, so it leads prestige and
    /// is the earlier signal of the two.
    LivingMembers,
    /// The summed renown of those living members — the quantity a house's
    /// prestige actually tracks. A line can keep its numbers and still collapse
    /// when the one legend among them dies.
    LivingRenown,
    // Regions (and, where shared, settlements).
    Prosperity,
    Chaos,
    Danger,
    MagicAffinity,
    Population,
    CulturalInfluence,
    DivineResonance,
    Strife,
    Harvest,
    Lore,
    Famine,
    RegionStatus,
    // Trade routes.
    Volume,
    // Resource nodes.
    NodeStatus,
}

impl Field {
    /// The field's name as data files and the decomposition view spell it.
    pub fn key(self) -> &'static str {
        match self {
            Field::Prestige => "prestige",
            Field::LivingMembers => "living_members",
            Field::LivingRenown => "living_renown",
            Field::Prosperity => "prosperity",
            Field::Chaos => "chaos",
            Field::Danger => "danger",
            Field::MagicAffinity => "magic_affinity",
            Field::Population => "population",
            Field::CulturalInfluence => "cultural_influence",
            Field::DivineResonance => "divine_resonance",
            Field::Strife => "strife",
            Field::Harvest => "harvest",
            Field::Lore => "lore",
            Field::Famine => "famine",
            Field::RegionStatus => "status",
            Field::Volume => "volume",
            Field::NodeStatus => "node_status",
        }
    }
}

/// How a watched field moved.
#[derive(Debug, Clone, PartialEq)]
pub enum Change {
    /// A continuous value moved. Both ends are kept so a pricing rule can use
    /// the difference, the ratio, or a threshold crossing without re-deriving
    /// any of them.
    Scalar { before: f32, after: f32 },
    /// A flag flipped — famine set or lifted.
    Flag { before: bool, after: bool },
    /// A discrete status changed, as its serialized label.
    Status { before: String, after: String },
}

impl Change {
    /// The signed size of a scalar move; zero for the discrete kinds, which
    /// carry direction in their labels rather than in a magnitude.
    pub fn magnitude(&self) -> f32 {
        match self {
            Change::Scalar { before, after } => after - before,
            Change::Flag { .. } | Change::Status { .. } => 0.0,
        }
    }
}

/// One field of one entity moving between consecutive views.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldMove {
    pub entity: EntityRef,
    pub field: Field,
    pub change: Change,
}

/// Everything that changed between two consecutive world views.
///
/// `listed` and `delisted` are the ones no authored market could produce: a
/// house is founded when a hero passes into legend, and delisted when its blood
/// runs out; a settlement can be razed outright. An instrument whose backing
/// entity has left the world stops being tradeable and becomes a record
/// (GDD 8, 12 open question 6).
#[derive(Debug, Clone, Default)]
pub struct WorldDelta {
    pub to_tick: u64,
    pub from_year: u32,
    pub to_year: u32,
    /// Entities present in the new view and absent from the old.
    pub listed: Vec<EntityRef>,
    /// Entities present in the old view and absent from the new.
    pub delisted: Vec<EntityRef>,
    /// Field moves on entities present in both.
    pub moves: Vec<FieldMove>,
    /// The chronicle lines that accompany this turning — the newspaper, not the
    /// signal. Nothing in pricing may read these.
    pub events: Vec<WorldEvent>,
}

impl WorldDelta {
    /// How many world years this delta spans. Usually one; more if the exchange
    /// was away from the wire while the world kept turning.
    pub fn years(&self) -> u32 {
        self.to_year.saturating_sub(self.from_year)
    }

    /// Whether anything at all moved — a turning can be genuinely quiet.
    pub fn is_empty(&self) -> bool {
        self.listed.is_empty() && self.delisted.is_empty() && self.moves.is_empty()
    }

    /// The largest scalar moves first, for a digest that leads with what
    /// actually mattered rather than with whatever the diff happened to visit
    /// first. Discrete changes — a famine set, a mine contested — sort last by
    /// magnitude but are never dropped, because a flag flip can matter more
    /// than any number beside it.
    pub fn most_moved(&self, limit: usize) -> Vec<&FieldMove> {
        let mut ranked: Vec<&FieldMove> = self.moves.iter().collect();
        ranked.sort_by(|a, b| {
            b.change
                .magnitude()
                .abs()
                .total_cmp(&a.change.magnitude().abs())
        });
        ranked.truncate(limit);
        ranked
    }
}

/// Diff two consecutive world views into typed changes.
///
/// `before` and `after` need not be one tick apart; if the exchange was off the
/// wire for a decade the delta simply spans it, and `years()` reports how far.
pub fn diff(before: &WorldView, after: &WorldView) -> WorldDelta {
    let mut delta = WorldDelta {
        to_tick: after.tick_count,
        from_year: before.year,
        to_year: after.year,
        ..Default::default()
    };

    // Houses and Orders are priced on their *living* membership, which is not a
    // field either one carries — it has to be reconstructed against the hero
    // roster of the view it came from. Both rosters are indexed once here and
    // borrowed by the two closures below.
    let old_heroes = living_heroes(&before.heroes);
    let new_heroes = living_heroes(&after.heroes);

    diff_entities(
        &before.houses,
        &after.houses,
        |h| h.id.as_str(),
        |h| EntityRef::House(h.id.clone()),
        |old, new, entity, moves| {
            house_moves(old, new, &old_heroes, &new_heroes, entity, moves);
        },
        &mut delta,
    );
    diff_entities(
        &before.orders,
        &after.orders,
        |o| o.id.as_str(),
        |o| EntityRef::Order(o.id.clone()),
        |old, new, entity, moves| {
            order_moves(old, new, &before.heroes, &after.heroes, entity, moves);
        },
        &mut delta,
    );
    diff_entities(
        &before.regions,
        &after.regions,
        |r| r.id.as_str(),
        |r| EntityRef::Region(r.id.clone()),
        region_moves,
        &mut delta,
    );
    diff_entities(
        &before.settlements,
        &after.settlements,
        |s| s.id.as_str(),
        |s| EntityRef::Settlement(s.id.clone()),
        settlement_moves,
        &mut delta,
    );
    diff_entities(
        &before.trade_routes,
        &after.trade_routes,
        |t| t.id.as_str(),
        |t| EntityRef::TradeRoute(t.id.clone()),
        route_moves,
        &mut delta,
    );
    diff_entities(
        &before.resource_nodes,
        &after.resource_nodes,
        |n| n.id.as_str(),
        |n| EntityRef::ResourceNode(n.id.clone()),
        node_moves,
        &mut delta,
    );

    delta
}

/// Diff one keyed collection: what appeared, what vanished, and — for the ids
/// in both — whatever `fields` reports as moved. Every entity class goes
/// through here so listing and delisting can never be handled inconsistently
/// between them.
fn diff_entities<T, Id, Anchor, Fields>(
    before: &[T],
    after: &[T],
    id: Id,
    anchor: Anchor,
    fields: Fields,
    delta: &mut WorldDelta,
) where
    Id: Fn(&T) -> &str,
    Anchor: Fn(&T) -> EntityRef,
    Fields: Fn(&T, &T, &EntityRef, &mut Vec<FieldMove>),
{
    let old: BTreeMap<&str, &T> = before.iter().map(|item| (id(item), item)).collect();
    let new: BTreeMap<&str, &T> = after.iter().map(|item| (id(item), item)).collect();

    for (key, item) in &new {
        match old.get(key) {
            Some(previous) => {
                let entity = anchor(item);
                fields(previous, item, &entity, &mut delta.moves);
            }
            None => delta.listed.push(anchor(item)),
        }
    }
    for (key, item) in &old {
        if !new.contains_key(key) {
            delta.delisted.push(anchor(item));
        }
    }
}

/// Record a scalar move, unless it is small enough to be float noise.
fn scalar(entity: &EntityRef, field: Field, before: f32, after: f32, moves: &mut Vec<FieldMove>) {
    if (after - before).abs() <= SCALAR_EPSILON {
        return;
    }
    moves.push(FieldMove {
        entity: entity.clone(),
        field,
        change: Change::Scalar { before, after },
    });
}

/// Record a flag flip.
fn flag(entity: &EntityRef, field: Field, before: bool, after: bool, moves: &mut Vec<FieldMove>) {
    if before == after {
        return;
    }
    moves.push(FieldMove {
        entity: entity.clone(),
        field,
        change: Change::Flag { before, after },
    });
}

/// Record a discrete status change, comparing by serialized label so the
/// exchange never has to enumerate the world's status enums itself.
fn status<T: PartialEq + std::fmt::Debug>(
    entity: &EntityRef,
    field: Field,
    before: T,
    after: T,
    moves: &mut Vec<FieldMove>,
) {
    if before == after {
        return;
    }
    moves.push(FieldMove {
        entity: entity.clone(),
        field,
        change: Change::Status {
            before: format!("{before:?}"),
            after: format!("{after:?}"),
        },
    });
}

/// Index a view's living heroes by id. The dead are dropped here rather than
/// filtered at each use, because every question the exchange asks of the roster
/// is about who is still drawing breath.
fn living_heroes(heroes: &[Hero]) -> BTreeMap<&str, &Hero> {
    heroes
        .iter()
        .filter(|hero| hero.is_alive)
        .map(|hero| (hero.id.as_str(), hero))
        .collect()
}

/// A house's living membership and the renown they carry between them.
///
/// `member_ids` is the whole lineage, living and dead, so its length only ever
/// grows and says nothing about a line's present strength. What a house's
/// prestige drifts toward is the summed renown of its *living* members, so that
/// is what the exchange watches — and watching both counts separately is what
/// distinguishes a house that lost a member from one that lost its legend.
fn house_vitals(house: &House, living: &BTreeMap<&str, &Hero>) -> (f32, f32) {
    let members: Vec<&Hero> = house
        .member_ids
        .iter()
        .filter_map(|id| living.get(id.as_str()).copied())
        .collect();
    let renown = members.iter().map(|hero| hero.renown).sum();
    (members.len() as f32, renown)
}

fn house_moves(
    before: &House,
    after: &House,
    old_living: &BTreeMap<&str, &Hero>,
    new_living: &BTreeMap<&str, &Hero>,
    entity: &EntityRef,
    moves: &mut Vec<FieldMove>,
) {
    scalar(
        entity,
        Field::Prestige,
        before.prestige,
        after.prestige,
        moves,
    );

    let (old_members, old_renown) = house_vitals(before, old_living);
    let (new_members, new_renown) = house_vitals(after, new_living);
    scalar(
        entity,
        Field::LivingMembers,
        old_members,
        new_members,
        moves,
    );
    scalar(entity, Field::LivingRenown, old_renown, new_renown, moves);
}

/// An Order's fellowship is every living hero of its calling, wherever they
/// dwell — it holds no member list of its own, so the roster is counted by role.
fn order_vitals(order: &Order, heroes: &[Hero]) -> (f32, f32) {
    let fellowship = heroes
        .iter()
        .filter(|hero| hero.is_alive && hero.role == order.role);
    let mut members = 0.0;
    let mut renown = 0.0;
    for hero in fellowship {
        members += 1.0;
        renown += hero.renown;
    }
    (members, renown)
}

fn order_moves(
    before: &Order,
    after: &Order,
    old_heroes: &[Hero],
    new_heroes: &[Hero],
    entity: &EntityRef,
    moves: &mut Vec<FieldMove>,
) {
    scalar(
        entity,
        Field::Prestige,
        before.prestige,
        after.prestige,
        moves,
    );

    let (old_members, old_renown) = order_vitals(before, old_heroes);
    let (new_members, new_renown) = order_vitals(after, new_heroes);
    scalar(
        entity,
        Field::LivingMembers,
        old_members,
        new_members,
        moves,
    );
    scalar(entity, Field::LivingRenown, old_renown, new_renown, moves);
}

fn region_moves(before: &Region, after: &Region, entity: &EntityRef, moves: &mut Vec<FieldMove>) {
    scalar(
        entity,
        Field::Prosperity,
        before.prosperity,
        after.prosperity,
        moves,
    );
    scalar(entity, Field::Chaos, before.chaos, after.chaos, moves);
    scalar(entity, Field::Danger, before.danger, after.danger, moves);
    scalar(
        entity,
        Field::MagicAffinity,
        before.magic_affinity,
        after.magic_affinity,
        moves,
    );
    scalar(
        entity,
        Field::Population,
        before.population,
        after.population,
        moves,
    );
    scalar(
        entity,
        Field::CulturalInfluence,
        before.cultural_influence,
        after.cultural_influence,
        moves,
    );
    scalar(
        entity,
        Field::DivineResonance,
        before.divine_resonance,
        after.divine_resonance,
        moves,
    );
    scalar(entity, Field::Strife, before.strife, after.strife, moves);
    scalar(entity, Field::Harvest, before.harvest, after.harvest, moves);
    scalar(entity, Field::Lore, before.lore, after.lore, moves);
    flag(entity, Field::Famine, before.famine, after.famine, moves);
    status(
        entity,
        Field::RegionStatus,
        before.status,
        after.status,
        moves,
    );
}

fn settlement_moves(
    before: &Settlement,
    after: &Settlement,
    entity: &EntityRef,
    moves: &mut Vec<FieldMove>,
) {
    scalar(
        entity,
        Field::Population,
        before.population,
        after.population,
        moves,
    );
    scalar(
        entity,
        Field::Prosperity,
        before.prosperity,
        after.prosperity,
        moves,
    );
}

fn route_moves(
    before: &TradeRoute,
    after: &TradeRoute,
    entity: &EntityRef,
    moves: &mut Vec<FieldMove>,
) {
    scalar(entity, Field::Volume, before.volume, after.volume, moves);
}

fn node_moves(
    before: &ResourceNode,
    after: &ResourceNode,
    entity: &EntityRef,
    moves: &mut Vec<FieldMove>,
) {
    status(
        entity,
        Field::NodeStatus,
        before.status,
        after.status,
        moves,
    );
}

#[cfg(test)]
mod tests;
