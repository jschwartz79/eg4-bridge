mod common;

use common::*;
use eg4_bridge::config::ConfigWrapper;
use eg4_bridge::home_assistant;
use serde_json::Value;
use std::collections::HashSet;

#[test]
fn discovery_disabled_returns_empty() {
    let mut cfg = Factory::example_config();
    cfg.homeassistant_enabled = false;
    let inverter = cfg.inverters[0].clone();
    let mqtt_cfg = cfg.mqtt.clone();
    let wrapper = ConfigWrapper::from_config(cfg);

    let ha = home_assistant::Config::new(&inverter, &mqtt_cfg, &wrapper);
    assert!(ha.all().unwrap().is_empty());
}

#[test]
fn discovery_payloads_cover_eg4_6000xp_features() {
    let mut cfg = Factory::example_config();
    cfg.homeassistant_enabled = true;
    let inverter = cfg.inverters[0].clone();
    let mqtt_cfg = cfg.mqtt.clone();
    let wrapper = ConfigWrapper::from_config(cfg);

    let ha = home_assistant::Config::new(&inverter, &mqtt_cfg, &wrapper);
    let msgs = ha.all().unwrap();
    assert!(!msgs.is_empty());

    let mut topics = HashSet::new();
    let mut unique_ids = HashSet::new();
    let mut kinds = HashSet::new();

    for msg in &msgs {
        assert!(msg.retain, "discovery message {} must be retained", msg.topic);
        assert!(
            topics.insert(msg.topic.clone()),
            "duplicate discovery topic: {}",
            msg.topic
        );

        let parts: Vec<&str> = msg.topic.split('/').collect();
        assert_eq!(parts.len(), 5, "unexpected discovery topic format: {}", msg.topic);
        kinds.insert(parts[1].to_string());

        let json: Value = serde_json::from_str(&msg.payload)
            .unwrap_or_else(|e| panic!("invalid JSON for {}: {}", msg.topic, e));
        let uid = json["unique_id"]
            .as_str()
            .unwrap_or_else(|| panic!("missing unique_id in {}", msg.topic));
        assert!(
            unique_ids.insert(uid.to_string()),
            "duplicate unique_id: {}",
            uid
        );
        assert_eq!(json["device"]["manufacturer"], "EG4 Electronics");
        assert_eq!(json["device"]["model"], "EG4 6000XP");
    }

    // All 5 Home Assistant entity domains must be present
    for domain in ["sensor", "switch", "select", "number", "text"] {
        assert!(kinds.contains(domain), "missing HA domain: {}", domain);
    }

    // Spot-check key EG4 6000XP features
    let find_topic = |suffix: &str| {
        msgs.iter()
            .find(|m| m.topic.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing discovery topic ending with {}", suffix))
    };

    // Scaled 0.1V number (Smart Load On Voltage, reg 213)
    let smart_load_v: Value =
        serde_json::from_str(&find_topic("/number/lxp_2222222222/SmartLoadOnVolt/config").payload)
            .unwrap();
    assert_eq!(smart_load_v["unit_of_measurement"], "V");
    assert_eq!(smart_load_v["step"], 0.1);
    assert!(smart_load_v["command_template"].as_str().unwrap().contains("* 10"));

    // Select for GEN port mode (reg 179 bit 13)
    let gen_port: Value =
        serde_json::from_str(&find_topic("/select/lxp_2222222222/gen_port_mode/config").payload)
            .unwrap();
    assert_eq!(gen_port["options"], serde_json::json!(["Generator", "Smart load"]));

    // Peak Shaving switch & timeslot
    assert!(msgs
        .iter()
        .any(|m| m.topic.ends_with("/switch/lxp_2222222222/grid_peak_shaving/config")));
    assert!(msgs
        .iter()
        .any(|m| m.topic.ends_with("/text/lxp_2222222222/peak_shaving_1/config")));

    // AC Coupling switch & Off-grid sensors
    assert!(msgs
        .iter()
        .any(|m| m.topic.ends_with("/switch/lxp_2222222222/ac_couple/config")));
    assert!(msgs
        .iter()
        .any(|m| m.topic.ends_with("/sensor/lxp_2222222222/soh/config")));
    assert!(msgs
        .iter()
        .any(|m| m.topic.ends_with("/sensor/lxp_2222222222/cycle_count/config")));
}
