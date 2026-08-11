//! Embedded game data and the exchange's configuration.

use crate::market::orders::OrderConfig;
use crate::market::{ImpactTable, InstrumentRules, MarketConfig};
use crate::state::LicenceTable;
use macroquad_toolkit::achievements::Achievement;
use macroquad_toolkit::data_loader::{load_embedded_json, load_embedded_json_labeled};
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/game_config.json");
const INSTRUMENTS_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/instruments.json");
const IMPACTS_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/impacts.json");
const LICENCES_JSON: &str = macroquad_toolkit::include_json_str!("../assets/data/licences.json");
const ACHIEVEMENTS_JSON: &str =
    macroquad_toolkit::include_json_str!("../assets/data/achievements.json");

/// Where the exchange reads its world, and how often (GDD 5.1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
    /// The authority server a local/dev build reads. Used directly by a debug
    /// native build.
    pub server_url: String,
    /// Public gateway base URL a *published* build reads instead, so a
    /// home-run server is reachable through one https origin without exposing
    /// its address. Mirrors mytherra's own `gateway_url`, and for the same
    /// reason: a browser on an https page cannot call a plain-http server, so
    /// the WebGL build has no other route. Empty falls back to `server_url`.
    #[serde(default)]
    pub gateway_url: String,
    /// The shared secret for the server's read-only spectator endpoints.
    ///
    /// **Committed blank on purpose.** On native it is overridden by
    /// `TB_REALMS_SPECTATOR_TOKEN`, so no secret lives in git. A WebGL build
    /// has nowhere secret to keep one and needs this filled in before
    /// publishing — at which point the token is readable from the `.wasm` by
    /// anyone, which is why it may only ever guard a read-only projection.
    #[serde(default)]
    pub spectator_token: String,
    /// Real seconds between reads of the world. The shared world turns on the
    /// server's schedule; the exchange re-reads at this cadence.
    pub view_poll_seconds: f32,
    /// Seconds before a read in flight is treated as failed.
    pub request_timeout_seconds: f32,
    /// How prices form (GDD 5.4).
    pub market: MarketConfig,
    /// What an order costs (GDD 5.5, open question 3).
    pub orders: OrderConfig,
}

impl GameConfig {
    /// The base URL this build should read.
    ///
    /// A debug native build talks straight to `server_url`; anything published
    /// — the WebGL build, or a release binary — goes through the gateway when
    /// one is configured. Native builds can override either with
    /// `TB_REALMS_SERVER_URL`.
    pub fn world_url(&self) -> String {
        if let Some(override_url) = env_override("TB_REALMS_SERVER_URL") {
            return override_url;
        }
        let published = cfg!(not(debug_assertions)) || cfg!(target_arch = "wasm32");
        if published && !self.gateway_url.is_empty() {
            self.gateway_url.clone()
        } else {
            self.server_url.clone()
        }
    }

    /// The spectator secret this build presents, or an error naming how to
    /// supply it. Fails loudly rather than sending an empty token and letting
    /// the server's 401 be mistaken for the world being down.
    pub fn spectator_token(&self) -> Result<String, String> {
        if let Some(token) = env_override("TB_REALMS_SPECTATOR_TOKEN") {
            return Ok(token);
        }
        if self.spectator_token.trim().is_empty() {
            return Err(
                "no spectator token: set TB_REALMS_SPECTATOR_TOKEN to the value of \
                 SPECTATOR_TOKEN in mytherra-server/.env (or fill in spectator_token \
                 in assets/data/game_config.json for a published build)"
                    .to_owned(),
            );
        }
        Ok(self.spectator_token.clone())
    }
}

/// A non-empty environment override. Always `None` on wasm, which has no
/// environment to read — the published build carries its config instead.
fn env_override(key: &str) -> Option<String> {
    if cfg!(target_arch = "wasm32") {
        return None;
    }
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    /// The instrument taxonomy and its fair-value rules (GDD 5.2, 5.4).
    pub instruments: InstrumentRules,
    /// The state-delta → price-shock table (GDD 5.5).
    pub impacts: ImpactTable,
    /// The information ladder (GDD 5.3).
    pub licences: LicenceTable,
    /// What a broker can be noticed doing. Definitions only — what has actually
    /// been won lives in the save.
    pub achievements: Vec<Achievement>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let instruments: InstrumentRules =
            load_embedded_json_labeled("instruments", INSTRUMENTS_JSON)?;
        // A taxonomy missing a class, or pricing on a field the world does not
        // carry, is a configuration error worth refusing to open the exchange
        // over — the alternative is a listing that silently never appears.
        instruments.validate()?;

        let impacts: ImpactTable = load_embedded_json_labeled("impacts", IMPACTS_JSON)?;
        // Likewise a shock rule that can never fire: a mistyped field key looks
        // exactly like a balance value someone deliberately set to zero.
        impacts.validate()?;

        let licences: LicenceTable = load_embedded_json_labeled("licences", LICENCES_JSON)?;
        // A class no licence unlocks would be paper nobody could ever reach.
        licences.validate()?;

        let achievements: Vec<Achievement> = load_embedded_json(ACHIEVEMENTS_JSON)?;
        if achievements.is_empty() {
            return Err("achievements.json defines none".to_owned());
        }

        Ok(Self {
            config: load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?,
            instruments,
            impacts,
            licences,
            achievements,
        })
    }
}

#[cfg(test)]
mod tests;
