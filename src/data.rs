//! Embedded game data and the exchange's configuration.

use crate::market::InstrumentRules;
use macroquad_toolkit::data_loader::load_embedded_json_labeled;
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const INSTRUMENTS_JSON: &str = include_str!("../assets/data/instruments.json");

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
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let instruments: InstrumentRules =
            load_embedded_json_labeled("instruments", INSTRUMENTS_JSON)?;
        // A taxonomy missing a class, or pricing on a field the world does not
        // carry, is a configuration error worth refusing to open the exchange
        // over — the alternative is a listing that silently never appears.
        instruments.validate()?;

        Ok(Self {
            config: load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?,
            instruments,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_config_loads() {
        let data = GameData::load().unwrap();

        assert_eq!(data.config.game_name, "tb_realms");
        assert!(data.config.view_poll_seconds > 0.0);
        assert!(data.config.request_timeout_seconds > 0.0);
        assert!(!data.config.server_url.is_empty());
    }

    #[test]
    fn a_blank_token_is_an_error_rather_than_an_empty_header() {
        let mut config = GameData::load().unwrap().config;
        config.spectator_token = "   ".to_owned();

        // Only meaningful when the developer's own override isn't set; when it
        // is, the override is the answer and that is the behaviour under test.
        match std::env::var("TB_REALMS_SPECTATOR_TOKEN") {
            Ok(value) if !value.trim().is_empty() => {
                assert_eq!(config.spectator_token().unwrap(), value);
            }
            _ => {
                let err = config.spectator_token().unwrap_err();
                assert!(err.contains("TB_REALMS_SPECTATOR_TOKEN"), "{err}");
            }
        }
    }

    #[test]
    fn a_published_build_prefers_the_gateway_and_a_dev_build_the_server() {
        let config = GameData::load().unwrap().config;
        assert!(!config.gateway_url.is_empty(), "the gateway is configured");

        // Skip when a developer override is in force — it outranks both, which
        // is exactly what the other branch of `world_url` promises.
        if std::env::var("TB_REALMS_SERVER_URL").is_ok() {
            return;
        }
        let expected = if cfg!(debug_assertions) {
            &config.server_url
        } else {
            &config.gateway_url
        };
        assert_eq!(&config.world_url(), expected);
    }
}
