//! Embedded game data and asset manifests.

use macroquad_toolkit::assets::TextureConfig;
use macroquad_toolkit::data_loader::{
    load_embedded_json, load_embedded_json_labeled, DataRegistry,
};
use serde::{Deserialize, Serialize};

const GAME_CONFIG_JSON: &str = include_str!("../assets/data/game_config.json");
const ACTIONS_JSON: &str = include_str!("../assets/data/actions.json");
const TEXTURE_MANIFEST_JSON: &str = include_str!("../assets/data/texture_manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameConfig {
    pub game_name: String,
    pub display_name: String,
    pub save_slot: String,
    pub version: String,
    pub starting_points: i64,
    pub starting_energy: f32,
    pub max_energy: f32,
    pub energy_per_second: f32,
    pub world_width: usize,
    pub world_height: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: String,
    pub name: String,
    pub description: String,
    pub energy_cost: f32,
    pub points_reward: i64,
}

#[derive(Debug, Clone)]
pub struct GameData {
    pub config: GameConfig,
    pub actions: DataRegistry<ActionDef>,
    pub texture_manifest: Vec<TextureConfig>,
}

impl GameData {
    pub fn load() -> Result<Self, String> {
        let config = load_embedded_json_labeled("game_config", GAME_CONFIG_JSON)?;
        let actions = DataRegistry::from_embedded_json(ACTIONS_JSON, "id")?;
        let texture_manifest = load_embedded_json(TEXTURE_MANIFEST_JSON)?;

        Ok(Self {
            config,
            actions,
            texture_manifest,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_loads() {
        let data = GameData::load().unwrap();

        assert!(!data.config.game_name.is_empty());
        assert!(data.actions.contains("gather"));
        assert!(data.config.world_width > 0);
        assert!(data.config.world_height > 0);
    }
}
