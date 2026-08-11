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
