//! What the exchange reads off the world, entity by entity (GDD 5.2, 5.5).
//!
//! This module owns the **one vocabulary** the rest of the game speaks about the
//! world: which entities back a listing, which of their numbers the exchange
//! watches, and what those numbers currently are. Two consumers read it and it
//! matters that they read the *same* thing:
//!
//! - [`crate::world::delta`] diffs two snapshots into typed changes, which is
//!   what prices move on.
//! - [`crate::market::instruments`] values a snapshot into fair values, which is
//!   what prices move *toward*.
//!
//! Were those two to read the world separately they would drift apart, and the
//! decomposition on a listing screen would stop adding up — a player asking "why
//! did it move" would get one story from the ticker and another from the
//! fundamentals. One reader, one vocabulary, no drift.
//!
//! Everything here is pure: no macroquad, no I/O, no engine types.

use mytherra_core::data::ResourceStatus;
use mytherra_core::world::Hero;
use mytherra_protocol::WorldView;
use std::collections::BTreeMap;
use std::fmt::Debug;

/// A world entity the exchange watches (GDD 5.2).
///
/// Heroes are deliberately absent: no instrument is claimed on a single hero —
/// heroes move prices *through* the houses and orders they belong to, which is
/// what makes house scrip a bet on a bloodline rather than on one life.
///
/// Two variants are not one-to-one with a world struct, and both are deliberate:
/// [`EntityRef::Commodity`] is an aggregate over every resource node of one
/// kind (you trade grain, not a named field), and the individual
/// [`EntityRef::ResourceNode`] beneath it is watched but never listed — a node
/// is a fact the commodity is priced on, not a claim anyone can hold.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EntityRef {
    House(String),
    Order(String),
    Region(String),
    Settlement(String),
    TradeRoute(String),
    ResourceNode(String),
    /// One resource kind across the whole world, keyed by its display label.
    Commodity(String),
    Artifact(String),
}

impl EntityRef {
    /// The entity's world id — for a commodity, the resource kind's label.
    pub fn id(&self) -> &str {
        match self {
            EntityRef::House(id)
            | EntityRef::Order(id)
            | EntityRef::Region(id)
            | EntityRef::Settlement(id)
            | EntityRef::TradeRoute(id)
            | EntityRef::ResourceNode(id)
            | EntityRef::Commodity(id)
            | EntityRef::Artifact(id) => id,
        }
    }

    /// The key `instruments.json` files this entity's class under, or `None` for
    /// an entity that is watched but never listed.
    ///
    /// This is the only place the world's entities are mapped to classes; what a
    /// class is *called* on the boards is the data file's business, so the two
    /// cannot drift.
    pub fn listing_class(&self) -> Option<&'static str> {
        match self {
            EntityRef::House(_) => Some("house_scrip"),
            EntityRef::Order(_) => Some("order_charter"),
            EntityRef::Region(_) => Some("regional_bond"),
            EntityRef::Settlement(_) => Some("settlement_share"),
            EntityRef::TradeRoute(_) => Some("route_toll"),
            EntityRef::Commodity(_) => Some("commodity"),
            EntityRef::Artifact(_) => Some("relic_lot"),
            // A single node is supply, not a security. It is diffed so a
            // status change is a priceable shock (GDD 5.5), and its worth
            // reaches the boards through the commodity above it.
            EntityRef::ResourceNode(_) => None,
        }
    }

    /// Every class that can carry a listing (GDD 5.2). The taxonomy is closed —
    /// `instruments.json` must cover exactly these, which is the no-orphan
    /// guarantee of GDD 14 enforced at load rather than discovered in play.
    pub const LISTING_CLASSES: [&'static str; 7] = [
        "house_scrip",
        "order_charter",
        "regional_bond",
        "settlement_share",
        "route_toll",
        "commodity",
        "relic_lot",
    ];
}

/// A world field the exchange watches.
///
/// This is the vocabulary the data files key on (GDD 6), so the variants are
/// named for the world's fields rather than for any market interpretation of
/// them — the mapping from "harvest fell" to "grain moved" belongs in data, not
/// here.
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
    // Commodities: the aggregate over every node of one kind.
    NodeCount,
    /// Nodes of this kind not yet worked out. A depleted node yields nothing by
    /// the world's own rule, so this is the honest count of what is producing —
    /// how *well* the rest produce arrives as shocks off `NodeStatus`.
    NodesProducing,
    // Artifacts.
    Power,
    Instability,
}

impl Field {
    /// Every watched field. Data files are validated against this, so a field
    /// added above cannot be silently missing from the balance surface.
    pub const ALL: [Field; 21] = [
        Field::Prestige,
        Field::LivingMembers,
        Field::LivingRenown,
        Field::Prosperity,
        Field::Chaos,
        Field::Danger,
        Field::MagicAffinity,
        Field::Population,
        Field::CulturalInfluence,
        Field::DivineResonance,
        Field::Strife,
        Field::Harvest,
        Field::Lore,
        Field::Famine,
        Field::RegionStatus,
        Field::Volume,
        Field::NodeStatus,
        Field::NodeCount,
        Field::NodesProducing,
        Field::Power,
        Field::Instability,
    ];

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
            Field::NodeCount => "node_count",
            Field::NodesProducing => "nodes_producing",
            Field::Power => "power",
            Field::Instability => "instability",
        }
    }

    /// The field a data file's key names, or `None` if it names nothing —
    /// which is a configuration error the loader reports rather than ignores.
    pub fn from_key(key: &str) -> Option<Field> {
        Field::ALL.into_iter().find(|field| field.key() == key)
    }
}

/// The current reading of one watched field.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Scalar(f32),
    Flag(bool),
    /// A discrete status, as the world's own variant name.
    Status(String),
}

/// One entity's watched numbers, and where it sits in the world.
#[derive(Debug, Clone, PartialEq)]
pub struct Fundamentals {
    pub entity: EntityRef,
    /// The entity's name in the world's own words, for the boards.
    pub name: String,
    /// The entity this one belongs to: a town's province, a node's commodity, a
    /// house's seat. News travels along this link in both directions — a node
    /// depleting reaches the commodity above it, a province's famine reaches the
    /// towns below it — which is what lets `impacts.json` price a shock on
    /// something other than the entity it happened to.
    ///
    /// A trade route has two endpoints and so has no single parent; an Order
    /// spans the world and belongs to no province. Both are `None`, and their
    /// news is their own.
    pub parent: Option<EntityRef>,
    pub values: BTreeMap<Field, Value>,
}

impl Fundamentals {
    fn new(entity: EntityRef, name: impl Into<String>) -> Self {
        Self {
            entity,
            name: name.into(),
            parent: None,
            values: BTreeMap::new(),
        }
    }

    fn under(mut self, parent: EntityRef) -> Self {
        self.parent = Some(parent);
        self
    }

    fn scalar_at(mut self, field: Field, value: f32) -> Self {
        self.values.insert(field, Value::Scalar(value));
        self
    }

    fn flag_at(mut self, field: Field, value: bool) -> Self {
        self.values.insert(field, Value::Flag(value));
        self
    }

    /// Record a discrete status by the world's own variant name, so the exchange
    /// never has to enumerate Mytherra's status enums itself.
    fn status_at(mut self, field: Field, value: impl Debug) -> Self {
        self.values
            .insert(field, Value::Status(format!("{value:?}")));
        self
    }

    pub fn scalar(&self, field: Field) -> Option<f32> {
        match self.values.get(&field) {
            Some(Value::Scalar(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn flag(&self, field: Field) -> Option<bool> {
        match self.values.get(&field) {
            Some(Value::Flag(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn status(&self, field: Field) -> Option<&str> {
        match self.values.get(&field) {
            Some(Value::Status(value)) => Some(value),
            _ => None,
        }
    }
}

/// Every watched entity in one world view, with its numbers.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub year: u32,
    pub tick: u64,
    entities: BTreeMap<EntityRef, Fundamentals>,
}

impl Snapshot {
    /// Read a world view into the exchange's vocabulary.
    pub fn read(view: &WorldView) -> Self {
        let mut entities = BTreeMap::new();
        let living = living_heroes(&view.heroes);

        for house in &view.houses {
            let (members, renown) = house_vitals(&house.member_ids, &living);
            push(
                &mut entities,
                Fundamentals::new(EntityRef::House(house.id.clone()), house.name.clone())
                    .under(EntityRef::Region(house.seat_region_id.clone()))
                    .scalar_at(Field::Prestige, house.prestige)
                    .scalar_at(Field::LivingMembers, members)
                    .scalar_at(Field::LivingRenown, renown),
            );
        }

        for order in &view.orders {
            let (members, renown) = order_vitals(&view.heroes, order.role);
            push(
                &mut entities,
                Fundamentals::new(EntityRef::Order(order.id.clone()), order.name.clone())
                    .scalar_at(Field::Prestige, order.prestige)
                    .scalar_at(Field::LivingMembers, members)
                    .scalar_at(Field::LivingRenown, renown),
            );
        }

        for region in &view.regions {
            push(
                &mut entities,
                Fundamentals::new(EntityRef::Region(region.id.clone()), region.name.clone())
                    .scalar_at(Field::Prosperity, region.prosperity)
                    .scalar_at(Field::Chaos, region.chaos)
                    .scalar_at(Field::Danger, region.danger)
                    .scalar_at(Field::MagicAffinity, region.magic_affinity)
                    .scalar_at(Field::Population, region.population)
                    .scalar_at(Field::CulturalInfluence, region.cultural_influence)
                    .scalar_at(Field::DivineResonance, region.divine_resonance)
                    .scalar_at(Field::Strife, region.strife)
                    .scalar_at(Field::Harvest, region.harvest)
                    .scalar_at(Field::Lore, region.lore)
                    .flag_at(Field::Famine, region.famine)
                    .status_at(Field::RegionStatus, region.status),
            );
        }

        for settlement in &view.settlements {
            push(
                &mut entities,
                Fundamentals::new(
                    EntityRef::Settlement(settlement.id.clone()),
                    settlement.name.clone(),
                )
                .under(EntityRef::Region(settlement.region_id.clone()))
                .scalar_at(Field::Population, settlement.population)
                .scalar_at(Field::Prosperity, settlement.prosperity),
            );
        }

        for route in &view.trade_routes {
            push(
                &mut entities,
                Fundamentals::new(EntityRef::TradeRoute(route.id.clone()), route.name.clone())
                    .scalar_at(Field::Volume, route.volume),
            );
        }

        for node in &view.resource_nodes {
            push(
                &mut entities,
                Fundamentals::new(EntityRef::ResourceNode(node.id.clone()), node.name.clone())
                    // A node's only outlet to the boards: it is not itself a
                    // listing, so its news reaches the market through the
                    // commodity it supplies.
                    .under(EntityRef::Commodity(node.resource_type.label().to_owned()))
                    .status_at(Field::NodeStatus, node.status),
            );
        }

        for artifact in &view.artifacts {
            push(
                &mut entities,
                Fundamentals::new(
                    EntityRef::Artifact(artifact.id.clone()),
                    artifact.name.clone(),
                )
                .under(EntityRef::Region(artifact.region_id.clone()))
                .scalar_at(Field::Power, artifact.power as f32)
                .scalar_at(Field::Instability, artifact.instability),
            );
        }

        for (label, (count, producing)) in commodity_supply(view) {
            push(
                &mut entities,
                Fundamentals::new(EntityRef::Commodity(label.to_owned()), label)
                    .scalar_at(Field::NodeCount, count)
                    .scalar_at(Field::NodesProducing, producing),
            );
        }

        Self {
            year: view.year,
            tick: view.tick_count,
            entities,
        }
    }

    pub fn get(&self, entity: &EntityRef) -> Option<&Fundamentals> {
        self.entities.get(entity)
    }

    /// Every watched entity, in a stable order.
    pub fn iter(&self) -> impl Iterator<Item = &Fundamentals> {
        self.entities.values()
    }
}

fn push(entities: &mut BTreeMap<EntityRef, Fundamentals>, entry: Fundamentals) {
    entities.insert(entry.entity.clone(), entry);
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
fn house_vitals(member_ids: &[String], living: &BTreeMap<&str, &Hero>) -> (f32, f32) {
    let members: Vec<&Hero> = member_ids
        .iter()
        .filter_map(|id| living.get(id.as_str()).copied())
        .collect();
    let renown = members.iter().map(|hero| hero.renown).sum();
    (members.len() as f32, renown)
}

/// An Order's fellowship is every living hero of its calling, wherever they
/// dwell — it holds no member list of its own, so the roster is counted by role.
fn order_vitals(heroes: &[Hero], role: mytherra_core::data::HeroRole) -> (f32, f32) {
    let mut members = 0.0;
    let mut renown = 0.0;
    for hero in heroes.iter().filter(|h| h.is_alive && h.role == role) {
        members += 1.0;
        renown += hero.renown;
    }
    (members, renown)
}

/// Nodes per resource kind, and how many of them are still yielding.
///
/// A kind with no nodes anywhere produces no entry at all, so a commodity whose
/// last node is lost delists rather than lingering at zero — the world decides
/// what the boards carry.
fn commodity_supply(view: &WorldView) -> BTreeMap<&'static str, (f32, f32)> {
    let mut per_kind: BTreeMap<&'static str, (f32, f32)> = BTreeMap::new();
    for node in &view.resource_nodes {
        let entry = per_kind.entry(node.resource_type.label()).or_default();
        entry.0 += 1.0;
        if node.status != ResourceStatus::Depleted {
            entry.1 += 1.0;
        }
    }
    per_kind
}
