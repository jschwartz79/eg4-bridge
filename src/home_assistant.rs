use crate::eg4::packet::Register;
use crate::prelude::*;

use serde::{Serialize, Serializer};

// ValueTemplate {{{
#[derive(Clone, Debug, PartialEq)]
pub enum ValueTemplate {
    None,
    Default, // "{{ value_json.$key }}"
    String(String),
}
impl ValueTemplate {
    pub fn from_default(key: &str) -> Self {
        Self::String(format!("{{{{ value_json.{} }}}}", key))
    }
    pub fn is_none(&self) -> bool {
        *self == Self::None
    }
    pub fn is_default(&self) -> bool {
        *self == Self::Default
    }
}
impl Serialize for ValueTemplate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ValueTemplate::String(str) => serializer.serialize_str(str),
            // This is unreachable because ValueTemplate::None and ValueTemplate::Default
            // are skipped during serialization via #[serde(skip_serializing_if = "ValueTemplate::is_none")]
            _ => unreachable!(),
        }
    }
} // }}}

#[derive(Clone, Debug, Serialize)]
pub struct Availability {
    topic: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Device {
    manufacturer: String,
    model: String,
    name: String,
    identifiers: [String; 1],
}

pub struct Config {
    inverter: config::Inverter,
    mqtt_config: config::Mqtt,
    global_config: config::ConfigWrapper,
}

// https://www.home-assistant.io/integrations/sensor.mqtt/
#[derive(Clone, Debug, Serialize)]
pub struct Entity {
    // Transient key used to derive unique_id, topic, and default value_template
    #[serde(skip)]
    key: String,

    unique_id: String,
    name: String,
    state_topic: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    entity_category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<String>,
    #[serde(skip_serializing_if = "ValueTemplate::is_none")]
    value_template: ValueTemplate,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit_of_measurement: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon: Option<String>,

    device: Device,
    availability: Availability,
}

// https://www.home-assistant.io/integrations/switch.mqtt/
#[derive(Debug, Serialize)]
pub struct Switch {
    name: String,
    state_topic: String,
    command_topic: String,
    value_template: String,
    unique_id: String,
    device: Device,
    availability: Availability,
}

// https://www.home-assistant.io/integrations/number.mqtt/
#[derive(Debug, Serialize)]
pub struct Number {
    name: String,
    state_topic: String,
    command_topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_template: Option<String>,
    value_template: String,
    unique_id: String,
    device: Device,
    availability: Availability,
    min: f64,
    max: f64,
    step: f64,
    unit_of_measurement: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<String>,
}

// https://www.home-assistant.io/integrations/select.mqtt/
#[derive(Debug, Serialize)]
pub struct Select {
    name: String,
    state_topic: String,
    command_topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_template: Option<String>,
    value_template: String,
    options: Vec<String>,
    unique_id: String,
    device: Device,
    availability: Availability,
}

// https://www.home-assistant.io/integrations/text.mqtt/
#[derive(Debug, Serialize)]
pub struct Text {
    name: String,
    state_topic: String,
    command_topic: String,
    command_template: String,
    value_template: String,
    unique_id: String,
    device: Device,
    availability: Availability,
    pattern: String,
}

impl Config {
    pub fn new(
        inverter: &config::Inverter,
        mqtt_config: &config::Mqtt,
        global_config: &config::ConfigWrapper,
    ) -> Self {
        Self {
            inverter: inverter.clone(),
            mqtt_config: mqtt_config.clone(),
            global_config: global_config.clone(),
        }
    }

    fn datalog_str(&self) -> String {
        self.inverter
            .datalog()
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    pub fn sensors(&self) -> Vec<mqtt::Message> {
        let ns = self.mqtt_config.namespace();
        let dl = self.datalog_str();

        let base = Entity {
            key: String::new(),
            unique_id: String::new(),
            name: String::new(),
            entity_category: None,
            device_class: None,
            state_class: None,
            unit_of_measurement: None,
            icon: None,
            value_template: ValueTemplate::Default,
            state_topic: format!("{}/{}/inputs/all", ns, dl),
            device: self.device(),
            availability: self.availability(),
        };

        let voltage = Entity {
            device_class: Some("voltage".to_string()),
            state_class: Some("measurement".to_string()),
            unit_of_measurement: Some("V".to_string()),
            ..base.clone()
        };

        let frequency = Entity {
            device_class: Some("frequency".to_string()),
            state_class: Some("measurement".to_string()),
            unit_of_measurement: Some("Hz".to_string()),
            ..base.clone()
        };

        let power = Entity {
            device_class: Some("power".to_string()),
            state_class: Some("measurement".to_string()),
            unit_of_measurement: Some("W".to_string()),
            ..base.clone()
        };

        let current = Entity {
            device_class: Some("current".to_string()),
            state_class: Some("measurement".to_string()),
            unit_of_measurement: Some("A".to_string()),
            ..base.clone()
        };

        let energy = Entity {
            device_class: Some("energy".to_string()),
            state_class: Some("total_increasing".to_string()),
            unit_of_measurement: Some("kWh".to_string()),
            ..base.clone()
        };

        let temperature = Entity {
            device_class: Some("temperature".to_string()),
            state_class: Some("measurement".to_string()),
            unit_of_measurement: Some("°C".to_string()),
            ..base.clone()
        };

        let power_factor = Entity {
            device_class: Some("power_factor".to_string()),
            state_class: Some("measurement".to_string()),
            ..base.clone()
        };

        let diagnostic = Entity {
            entity_category: Some("diagnostic".to_string()),
            ..base.clone()
        };

        let sensors = vec![
            // Status & diagnostics
            Entity {
                key: "status".to_string(),
                name: "Status".to_string(),
                state_topic: format!("{}/{}/input/0/parsed", ns, dl),
                value_template: ValueTemplate::None,
                ..base.clone()
            },
            Entity {
                key: "soc".to_string(),
                name: "State of Charge".to_string(),
                device_class: Some("battery".to_string()),
                state_class: Some("measurement".to_string()),
                unit_of_measurement: Some("%".to_string()),
                ..base.clone()
            },
            Entity {
                key: "soh".to_string(),
                name: "State of Health".to_string(),
                entity_category: Some("diagnostic".to_string()),
                state_class: Some("measurement".to_string()),
                unit_of_measurement: Some("%".to_string()),
                icon: Some("mdi:battery-heart-variant".to_string()),
                ..base.clone()
            },
            Entity {
                key: "fault_code".to_string(),
                name: "Fault Code".to_string(),
                state_topic: format!("{}/{}/input/fault_code/parsed", ns, dl),
                value_template: ValueTemplate::None,
                icon: Some("mdi:alert".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "warning_code".to_string(),
                name: "Warning Code".to_string(),
                state_topic: format!("{}/{}/input/warning_code/parsed", ns, dl),
                value_template: ValueTemplate::None,
                icon: Some("mdi:alert-outline".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "internal_fault".to_string(),
                name: "Internal Fault Code".to_string(),
                icon: Some("mdi:alert-octagon".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bat_status_9".to_string(),
                name: "Battery Status".to_string(),
                state_topic: format!("{}/{}/inputs/3/bat_status_9_decoded", ns, dl),
                value_template: ValueTemplate::None,
                icon: Some("mdi:battery-status-variant".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bat_status_inv".to_string(),
                name: "Battery Inverter Status".to_string(),
                state_topic: format!("{}/{}/inputs/3/bat_status_inv_decoded", ns, dl),
                value_template: ValueTemplate::None,
                icon: Some("mdi:battery-sync".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bat_brand".to_string(),
                name: "Battery Brand".to_string(),
                state_topic: format!("{}/{}/inputs/2/bat_brand_decoded", ns, dl),
                value_template: ValueTemplate::None,
                icon: Some("mdi:battery".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bat_com_type".to_string(),
                name: "Battery Comm Type".to_string(),
                state_topic: format!("{}/{}/inputs/2/bat_com_type_decoded", ns, dl),
                value_template: ValueTemplate::None,
                icon: Some("mdi:serial-port".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bms_event_1".to_string(),
                name: "BMS Fault Code".to_string(),
                icon: Some("mdi:battery-alert".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bms_event_2".to_string(),
                name: "BMS Warning Code".to_string(),
                icon: Some("mdi:battery-alert-variant-outline".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bat_count".to_string(),
                name: "Battery Parallel Count".to_string(),
                state_class: Some("measurement".to_string()),
                icon: Some("mdi:battery-plus".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "bat_capacity".to_string(),
                name: "Battery Capacity".to_string(),
                state_class: Some("measurement".to_string()),
                unit_of_measurement: Some("Ah".to_string()),
                icon: Some("mdi:battery-high".to_string()),
                ..diagnostic.clone()
            },
            Entity {
                key: "cycle_count".to_string(),
                name: "Battery Cycle Count".to_string(),
                state_class: Some("total_increasing".to_string()),
                icon: Some("mdi:counter".to_string()),
                ..diagnostic.clone()
            },
            // Voltages
            Entity {
                key: "v_bat".to_string(),
                name: "Battery Voltage".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "vbat_inv".to_string(),
                name: "Battery Voltage (Inverter Sampled)".to_string(),
                entity_category: Some("diagnostic".to_string()),
                ..voltage.clone()
            },
            Entity {
                key: "charge_volt_ref".to_string(),
                name: "BMS Charge Voltage Reference".to_string(),
                entity_category: Some("diagnostic".to_string()),
                ..voltage.clone()
            },
            Entity {
                key: "dischg_cut_volt".to_string(),
                name: "BMS Discharge Cutoff Voltage".to_string(),
                entity_category: Some("diagnostic".to_string()),
                ..voltage.clone()
            },
            Entity {
                key: "v_ac_r".to_string(),
                name: "Grid Voltage".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_ac_s".to_string(),
                name: "Grid Voltage S".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_ac_t".to_string(),
                name: "Grid Voltage T".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_pv_1".to_string(),
                name: "PV Voltage (String 1)".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_pv_2".to_string(),
                name: "PV Voltage (String 2)".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_pv_3".to_string(),
                name: "PV Voltage (String 3)".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_eps_r".to_string(),
                name: "EPS Voltage".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_eps_s".to_string(),
                name: "EPS Voltage S".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_eps_t".to_string(),
                name: "EPS Voltage T".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_gen".to_string(),
                name: "Generator Voltage".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_eps_l1".to_string(),
                name: "EPS Voltage L1".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_eps_l2".to_string(),
                name: "EPS Voltage L2".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "v_bus_1".to_string(),
                name: "Bus 1 Voltage".to_string(),
                entity_category: Some("diagnostic".to_string()),
                ..voltage.clone()
            },
            Entity {
                key: "v_bus_2".to_string(),
                name: "Bus 2 Voltage".to_string(),
                entity_category: Some("diagnostic".to_string()),
                ..voltage.clone()
            },
            // Frequencies
            Entity {
                key: "f_ac".to_string(),
                name: "Grid Frequency".to_string(),
                ..frequency.clone()
            },
            Entity {
                key: "f_eps".to_string(),
                name: "EPS Frequency".to_string(),
                ..frequency.clone()
            },
            Entity {
                key: "f_eps_l1".to_string(),
                name: "EPS Frequency L1".to_string(),
                ..frequency.clone()
            },
            Entity {
                key: "f_eps_l2".to_string(),
                name: "EPS Frequency L2".to_string(),
                ..frequency.clone()
            },
            Entity {
                key: "f_gen".to_string(),
                name: "Generator Frequency".to_string(),
                ..frequency.clone()
            },
            // Power factor
            Entity {
                key: "pf".to_string(),
                name: "Power Factor".to_string(),
                ..power_factor.clone()
            },
            Entity {
                key: "pf_eps_l1".to_string(),
                name: "EPS Power Factor L1".to_string(),
                ..power_factor.clone()
            },
            Entity {
                key: "pf_eps_l2".to_string(),
                name: "EPS Power Factor L2".to_string(),
                ..power_factor.clone()
            },
            // Apparent power
            Entity {
                key: "s_eps".to_string(),
                name: "Apparent EPS Power".to_string(),
                device_class: Some("apparent_power".to_string()),
                unit_of_measurement: Some("VA".to_string()),
                ..power.clone()
            },
            Entity {
                key: "s_eps_l1".to_string(),
                name: "Apparent EPS Power L1".to_string(),
                device_class: Some("apparent_power".to_string()),
                unit_of_measurement: Some("VA".to_string()),
                ..power.clone()
            },
            Entity {
                key: "s_eps_l2".to_string(),
                name: "Apparent EPS Power L2".to_string(),
                device_class: Some("apparent_power".to_string()),
                unit_of_measurement: Some("VA".to_string()),
                ..power.clone()
            },
            // Active power
            Entity {
                key: "p_pv".to_string(),
                name: "PV Power (Array)".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_pv_1".to_string(),
                name: "PV Power (String 1)".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_pv_2".to_string(),
                name: "PV Power (String 2)".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_pv_3".to_string(),
                name: "PV Power (String 3)".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_battery".to_string(),
                name: "Battery Power (discharge is negative)".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_charge".to_string(),
                name: "Battery Charge".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_discharge".to_string(),
                name: "Battery Discharge".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_grid".to_string(),
                name: "Grid Power (export is negative)".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_to_user".to_string(),
                name: "Power from Grid".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_to_grid".to_string(),
                name: "Power to Grid".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_eps".to_string(),
                name: "Active EPS Power".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_inv".to_string(),
                name: "Inverter Power".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_rec".to_string(),
                name: "AC Charge Power".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_gen".to_string(),
                name: "Generator Power".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_eps_l1".to_string(),
                name: "EPS Power L1".to_string(),
                ..power.clone()
            },
            Entity {
                key: "p_eps_l2".to_string(),
                name: "EPS Power L2".to_string(),
                ..power.clone()
            },
            // Energy
            Entity {
                key: "e_pv_all".to_string(),
                name: "PV Generation (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_all_1".to_string(),
                name: "PV Generation (All time) (String 1)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_all_2".to_string(),
                name: "PV Generation (All time) (String 2)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_all_3".to_string(),
                name: "PV Generation (All time) (String 3)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_day".to_string(),
                name: "PV Generation (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_day_1".to_string(),
                name: "PV Generation (Today) (String 1)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_day_2".to_string(),
                name: "PV Generation (Today) (String 2)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_pv_day_3".to_string(),
                name: "PV Generation (Today) (String 3)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_chg_all".to_string(),
                name: "Battery Charge (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_chg_day".to_string(),
                name: "Battery Charge (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_dischg_all".to_string(),
                name: "Battery Discharge (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_dischg_day".to_string(),
                name: "Battery Discharge (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_to_user_all".to_string(),
                name: "Energy from Grid (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_to_user_day".to_string(),
                name: "Energy from Grid (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_to_grid_all".to_string(),
                name: "Energy to Grid (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_to_grid_day".to_string(),
                name: "Energy to Grid (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_eps_all".to_string(),
                name: "Energy from EPS (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_eps_day".to_string(),
                name: "Energy from EPS (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_rec_all".to_string(),
                name: "Energy of AC Charging (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_rec_day".to_string(),
                name: "Energy of AC Charging (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_inv_all".to_string(),
                name: "Energy of Inverter (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_inv_day".to_string(),
                name: "Energy of Inverter (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_gen_all".to_string(),
                name: "Energy of Generator (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_gen_day".to_string(),
                name: "Energy of Generator (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_eps_l1_all".to_string(),
                name: "Energy of EPS L1 (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_eps_l1_day".to_string(),
                name: "Energy of EPS L1 (Today)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_eps_l2_all".to_string(),
                name: "Energy of EPS L2 (All time)".to_string(),
                ..energy.clone()
            },
            Entity {
                key: "e_eps_l2_day".to_string(),
                name: "Energy of EPS L2 (Today)".to_string(),
                ..energy.clone()
            },
            // Temperatures
            Entity {
                key: "t_inner".to_string(),
                name: "Inverter Temperature".to_string(),
                ..temperature.clone()
            },
            Entity {
                key: "t_rad_1".to_string(),
                name: "Radiator 1 Temperature".to_string(),
                ..temperature.clone()
            },
            Entity {
                key: "t_rad_2".to_string(),
                name: "Radiator 2 Temperature".to_string(),
                ..temperature.clone()
            },
            Entity {
                key: "t_bat".to_string(),
                name: "Battery Temperature".to_string(),
                ..temperature.clone()
            },
            // Currents
            Entity {
                key: "bat_current".to_string(),
                name: "Battery Current (BMS)".to_string(),
                ..current.clone()
            },
            Entity {
                key: "max_chg_curr".to_string(),
                name: "Max Charge Current".to_string(),
                ..current.clone()
            },
            Entity {
                key: "max_dischg_curr".to_string(),
                name: "Max Discharge Current".to_string(),
                ..current.clone()
            },
            Entity {
                key: "i_eps_l1".to_string(),
                name: "EPS Current L1".to_string(),
                ..current.clone()
            },
            Entity {
                key: "i_eps_l2".to_string(),
                name: "EPS Current L2".to_string(),
                ..current.clone()
            },
            // BMS cell stats
            Entity {
                key: "min_cell_voltage".to_string(),
                name: "Min Cell Voltage (BMS)".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "max_cell_voltage".to_string(),
                name: "Max Cell Voltage (BMS)".to_string(),
                ..voltage.clone()
            },
            Entity {
                key: "min_cell_temp".to_string(),
                name: "Min Cell Temperature (BMS)".to_string(),
                ..temperature.clone()
            },
            Entity {
                key: "max_cell_temp".to_string(),
                name: "Max Cell Temperature (BMS)".to_string(),
                ..temperature.clone()
            },
            Entity {
                key: "runtime".to_string(),
                name: "Total Runtime".to_string(),
                entity_category: Some("diagnostic".to_string()),
                device_class: Some("duration".to_string()),
                state_class: Some("total_increasing".to_string()),
                unit_of_measurement: Some("s".to_string()),
                ..base.clone()
            },
        ];

        sensors
            .into_iter()
            .map(|mut sensor| {
                sensor.unique_id = self.unique_id(&sensor.key);
                if sensor.value_template.is_default() {
                    sensor.value_template = ValueTemplate::from_default(&sensor.key);
                }

                mqtt::Message {
                    topic: self.ha_discovery_topic("sensor", &sensor.key),
                    retain: true,
                    payload: serde_json::to_string(&sensor).unwrap(),
                }
            })
            .collect()
    }

    pub fn all(&self) -> Result<Vec<mqtt::Message>> {
        if !self.global_config.homeassistant_enabled() {
            return Ok(Vec::new());
        }

        let mut r = vec![
            // === Switches: Register 21 ===
            self.switch_reg(21, "ac_charge", "ac_charge_en", "AC Charge")?,
            self.switch_reg(21, "charge_priority", "charge_priority_en", "Charge Priority")?,
            self.switch_reg(21, "forced_discharge", "forced_discharge_en", "Forced Discharge")?,
            self.switch_reg(21, "eps", "eps_en", "Off-Grid Output (EPS)")?,
            self.switch_reg(
                21,
                "seamless_eps_switch",
                "sw_seamless_en",
                "Seamless EPS Switching",
            )?,
            self.switch_reg(21, "set_to_standby", "set_to_standby", "Inverter Power")?,
            self.switch_reg(21, "feed_in_grid", "feed_in_grid_en", "Export to Grid")?,
            // === Switches: Register 110 ===
            self.switch_reg(
                110,
                "fast_zero_export",
                "ub_run_without_grid",
                "Fast Zero Export",
            )?,
            self.switch_reg(110, "micro_grid", "ub_micro_grid_en", "Micro-Grid Mode")?,
            self.switch_reg(110, "bat_shared", "ub_bat_shared", "Shared Battery")?,
            self.switch_reg(110, "charge_last", "ub_chg_last_en", "Charge Last")?,
            self.switch_reg(110, "buzzer", "buzzer_en", "Off-Grid Buzzer")?,
            self.switch_reg(110, "green_mode", "green_mode_en", "Green Mode")?,
            self.switch_reg(110, "eco_mode", "eco_mode_en", "Eco Mode")?,
            // === Switches: Register 179 ===
            self.switch_reg(179, "bat_wakeup", "bat_wakeup_en", "Battery Wakeup")?,
            self.switch_reg(
                179,
                "grid_peak_shaving",
                "grid_peak_shaving_en",
                "Grid Peak Shaving",
            )?,
            self.switch_reg(
                179,
                "gen_peak_shaving",
                "gen_peak_shaving_en",
                "Generator Peak Shaving",
            )?,
            self.switch_reg(179, "ac_couple", "ac_couple_en", "AC Coupling (GEN Port)")?,
            // === Switches: Register 233 ===
            self.switch_reg(
                233,
                "battery_backup_ctrl",
                "battery_backup_ctrl_en",
                "Battery Backup Mode",
            )?,
            self.switch_reg(233, "sporadic_charge", "sporadic_charge_en", "Sporadic Charge")?,
            // === Selects: Whole-Register Enums ===
            self.select_mapped_register(
                Register::OutputPrioConfig,
                "output_priority",
                "Output Priority",
                &[(0, "Battery first"), (1, "PV first"), (2, "AC first")],
            )?,
            self.select_mapped_register(
                Register::LineMode,
                "line_mode",
                "AC Input Range",
                &[
                    (0, "APL (90-280V, 20ms)"),
                    (1, "UPS (170-280V, 10ms)"),
                    (2, "GEN (90-280V, 20ms)"),
                ],
            )?,
            self.select_mapped_register(
                Register::PvInputMode,
                "pv_input_mode",
                "PV Input Mode",
                &[
                    (0, "No PV"),
                    (1, "PV1"),
                    (2, "PV2"),
                    (3, "Two Parallel PV"),
                    (4, "Two Separate PV"),
                ],
            )?,
            self.select_direct_register(
                Register::EpsVoltageSet,
                "eps_voltage",
                "Backup Output Voltage (V)",
                &["208", "220", "230", "240", "277"],
            )?,
            self.select_direct_register(
                Register::EpsFreqSet,
                "eps_frequency",
                "Backup Output Frequency (Hz)",
                &["50", "60"],
            )?,
            // === Selects: Bitfields (Registers 110, 120, 179) ===
            self.select_bits(
                110,
                "ongrid_working_mode",
                "ongrid_working_mode",
                "On-Grid Working Mode",
                &["Self consumption", "Charge first"],
            )?,
            self.select_bits(
                120,
                "ac_charge_type",
                "ac_charge_type",
                "AC Charge Control",
                &[
                    "Disabled",
                    "By time",
                    "By voltage",
                    "By SOC",
                    "Voltage + time",
                    "SOC + time",
                ],
            )?,
            self.select_bits(
                120,
                "discharge_ctrl_type",
                "discharge_ctrl_type",
                "Discharge Control",
                &["By voltage", "By SOC", "Both"],
            )?,
            self.select_bits(
                120,
                "ongrid_eod_type",
                "ongrid_eod_type",
                "On-Grid EOD Control",
                &["By voltage", "By SOC"],
            )?,
            self.select_bits(
                120,
                "gen_charge_type",
                "gen_charge_type",
                "Generator Charge Control",
                &["By voltage", "By SOC"],
            )?,
            self.select_bits(
                179,
                "gen_port_mode",
                "gen_port_mode",
                "GEN Port Mode",
                &["Generator", "Smart load"],
            )?,
            self.select_bits(
                179,
                "ac_couple_ctrl_type",
                "ac_couple_ctrl_type",
                "AC Couple Control",
                &["By SOC", "By voltage"],
            )?,
            // === Numbers: Charging & Discharging ===
            self.number_percent(Register::ChargePowerPercentCmd, "System Charge Rate (%)")?,
            self.number_percent(Register::DischgPowerPercentCmd, "System Discharge Rate (%)")?,
            self.number_int(
                Register::ChargeCurr,
                "Charge Current Limit",
                0.0,
                140.0,
                "A",
                Some("current"),
            )?,
            self.number_int(
                Register::DischgCurr,
                "Discharge Current Limit",
                0.0,
                140.0,
                "A",
                Some("current"),
            )?,
            self.number_scaled_10(
                Register::ChargeVoltRef,
                "Charge Voltage (Lead-Acid)",
                50.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::CutVoltForDischg,
                "Discharge Cut-Off Voltage (Lead-Acid)",
                40.0,
                52.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::FloatChargeVolt,
                "Float Charge Voltage",
                50.0,
                56.0,
                "V",
                Some("voltage"),
            )?,
            self.number_percent(Register::MaxBackFlow, "Export Power Limit (%)")?,
            self.number_range(Register::DischgCutOffSocEod, "Discharge Cutoff %", 10.0, 90.0, "%")?,
            self.number_range(
                Register::EpsDischgCutoffSocEod,
                "Discharge Cutoff for EPS %",
                0.0,
                90.0,
                "%",
            )?,
            self.number_scaled_10(
                Register::OnGridEodVoltage,
                "On-Grid EOD Voltage",
                40.0,
                56.0,
                "V",
                Some("voltage"),
            )?,
            // === Numbers: AC Charging ===
            self.number_percent(Register::AcChargePowerCmd, "AC Charge Rate (%)")?,
            self.number_percent(Register::AcChargeSocLimit, "AC Charge Limit %")?,
            self.number_int(
                Register::AcChargeBatCurrent,
                "AC Charge Current Limit",
                0.0,
                140.0,
                "A",
                Some("current"),
            )?,
            self.number_scaled_10(
                Register::AcChargeStartVolt,
                "AC Charge Start Voltage",
                38.5,
                52.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::AcChargeEndVolt,
                "AC Charge End Voltage",
                48.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_range(
                Register::AcChargeStartSocLimit,
                "Charge From AC Lower Limit %",
                0.0,
                90.0,
                "%",
            )?,
            self.number_range(
                Register::AcChargeEndSocLimit,
                "Charge From AC Upper Limit %",
                20.0,
                100.0,
                "%",
            )?,
            // === Numbers: Charge Priority & Forced Discharge ===
            self.number_percent(Register::ChargePriorityPowerCmd, "Charge Priority Rate (%)")?,
            self.number_percent(Register::ChargePrioritySocLimit, "Charge Priority Limit %")?,
            self.number_scaled_10(
                Register::ChargePriorityEndVolt,
                "Charge Priority End Voltage",
                48.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_percent(Register::ForcedDischgPowerCmd, "Forced Discharge Rate (%)")?,
            self.number_percent(Register::ForcedDischgSocLimit, "Forced Discharge Limit %")?,
            self.number_scaled_10(
                Register::ForcedDischgEndVolt,
                "Forced Discharge End Voltage",
                40.0,
                56.0,
                "V",
                Some("voltage"),
            )?,
            // === Numbers: Battery Protection & Lead-Acid ===
            self.number_scaled_10(
                Register::BatLowVoltage,
                "Battery Low Alarm Voltage",
                40.0,
                50.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::BatLowBackVoltage,
                "Battery Low Recovery Voltage",
                42.0,
                52.0,
                "V",
                Some("voltage"),
            )?,
            self.number_range(Register::BatLowSoc, "Battery Low Alarm SOC", 0.0, 90.0, "%")?,
            self.number_range(
                Register::BatLowBackSoc,
                "Battery Low Recovery SOC",
                20.0,
                100.0,
                "%",
            )?,
            self.number_scaled_10(
                Register::BatLowToUtilityVoltage,
                "Switch to Grid Voltage",
                44.4,
                51.4,
                "V",
                Some("voltage"),
            )?,
            self.number_percent(Register::BatLowToUtilitySoc, "Switch to Grid SOC")?,
            self.number_int(
                Register::BatteryCapacity,
                "Battery Capacity (Unmatched)",
                0.0,
                10000.0,
                "Ah",
                None,
            )?,
            self.number_scaled_10(
                Register::BatteryNominalVolt,
                "Battery Nominal Voltage (Unmatched)",
                40.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::EqualizationVolt,
                "Equalization Voltage",
                50.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_int(
                Register::EqualizationInterval,
                "Equalization Interval",
                0.0,
                365.0,
                "d",
                None,
            )?,
            self.number_int(
                Register::EqualizationTime,
                "Equalization Duration",
                0.0,
                24.0,
                "h",
                None,
            )?,
            // === Numbers: Generator ===
            self.number_scaled_10(
                Register::GenChgStartVolt,
                "Generator Charge Start Voltage",
                38.4,
                52.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::GenChgEndVolt,
                "Generator Charge End Voltage",
                48.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_range(
                Register::GenChgStartSoc,
                "Generator Charge Start SOC",
                0.0,
                90.0,
                "%",
            )?,
            self.number_range(
                Register::GenChgEndSoc,
                "Generator Charge End SOC",
                20.0,
                100.0,
                "%",
            )?,
            self.number_int(
                Register::MaxGenChgBatCurr,
                "Generator Charge Current Limit",
                0.0,
                60.0,
                "A",
                Some("current"),
            )?,
            // === Numbers: Grid Peak Shaving ===
            self.number_scaled_10(
                Register::GridPeakShavingPower,
                "Peak Shaving Power",
                0.0,
                25.5,
                "kW",
                Some("power"),
            )?,
            self.number_percent(Register::GridPeakShavingSoc, "Peak Shaving SOC")?,
            self.number_scaled_10(
                Register::GridPeakShavingVolt,
                "Peak Shaving Voltage",
                48.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_percent(Register::GridPeakShavingSoc2, "Peak Shaving SOC 2")?,
            self.number_scaled_10(
                Register::GridPeakShavingVolt2,
                "Peak Shaving Voltage 2",
                48.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            // === Numbers: Smart Load (GEN Port) ===
            self.number_scaled_10(
                Register::SmartLoadOnVolt,
                "Smart Load On Voltage",
                48.0,
                59.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::SmartLoadOffVolt,
                "Smart Load Off Voltage",
                40.0,
                52.0,
                "V",
                Some("voltage"),
            )?,
            self.number_percent(Register::SmartLoadOnSoc, "Smart Load On SOC")?,
            self.number_percent(Register::SmartLoadOffSoc, "Smart Load Off SOC")?,
            self.number_scaled_10(
                Register::SmartLoadStartPvPower,
                "Smart Load Min PV Power",
                0.0,
                12.0,
                "kW",
                Some("power"),
            )?,
            // === Numbers: AC Coupling (GEN Port) ===
            self.number_range(
                Register::AcCoupleStartSoc,
                "AC Couple Start SOC",
                0.0,
                80.0,
                "%",
            )?,
            self.number_percent(Register::AcCoupleEndSoc, "AC Couple End SOC")?,
            self.number_scaled_10(
                Register::AcCoupleStartVolt,
                "AC Couple Start Voltage",
                40.0,
                52.0,
                "V",
                Some("voltage"),
            )?,
            self.number_scaled_10(
                Register::AcCoupleEndVolt,
                "AC Couple End Voltage",
                40.0,
                56.0,
                "V",
                Some("voltage"),
            )?,
            // === Time Ranges ===
            self.time_range("ac_charge/1", "AC Charge Timeslot 1")?,
            self.time_range("ac_charge/2", "AC Charge Timeslot 2")?,
            self.time_range("ac_charge/3", "AC Charge Timeslot 3")?,
            self.time_range("ac_first/1", "AC First Timeslot 1")?,
            self.time_range("ac_first/2", "AC First Timeslot 2")?,
            self.time_range("ac_first/3", "AC First Timeslot 3")?,
            self.time_range("charge_priority/1", "Charge Priority Timeslot 1")?,
            self.time_range("charge_priority/2", "Charge Priority Timeslot 2")?,
            self.time_range("charge_priority/3", "Charge Priority Timeslot 3")?,
            self.time_range("forced_discharge/1", "Forced Discharge Timeslot 1")?,
            self.time_range("forced_discharge/2", "Forced Discharge Timeslot 2")?,
            self.time_range("forced_discharge/3", "Forced Discharge Timeslot 3")?,
            self.time_range("peak_shaving/1", "Peak Shaving Timeslot 1")?,
            self.time_range("peak_shaving/2", "Peak Shaving Timeslot 2")?,
        ];

        r.append(&mut self.sensors());

        Ok(r)
    }

    fn ha_discovery_topic(&self, kind: &str, name: &str) -> String {
        format!(
            "{}/{}/lxp_{}/{}/config",
            self.mqtt_config.homeassistant().prefix(),
            kind,
            self.datalog_str(),
            // The forward slash is used in some names (e.g. ac_charge/1) but
            // has semantic meaning in MQTT, so must be changed
            name.replace('/', "_"),
        )
    }

    fn switch_reg(
        &self,
        register: u16,
        name: &str,
        json_field: &str,
        label: &str,
    ) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let config = Switch {
            value_template: format!("{{{{ value_json.{} }}}}", json_field),
            state_topic: format!(
                "{}/{}/hold/{}/bits",
                self.mqtt_config.namespace(),
                dl,
                register
            ),
            command_topic: format!(
                "{}/cmd/{}/set/{}",
                self.mqtt_config.namespace(),
                dl,
                name
            ),
            unique_id: format!("lxp_{}_{}", dl, name),
            name: label.to_string(),
            device: self.device(),
            availability: self.availability(),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("switch", name),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    fn number_percent(&self, register: Register, label: &str) -> Result<mqtt::Message> {
        self.number_range(register, label, 0.0, 100.0, "%")
    }

    fn number_range(
        &self,
        register: Register,
        label: &str,
        min: f64,
        max: f64,
        unit: &str,
    ) -> Result<mqtt::Message> {
        self.number_int(register, label, min, max, unit, None)
    }

    fn number_int(
        &self,
        register: Register,
        label: &str,
        min: f64,
        max: f64,
        unit: &str,
        device_class: Option<&str>,
    ) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let config = Number {
            name: label.to_string(),
            state_topic: format!(
                "{}/{}/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_topic: format!(
                "{}/cmd/{}/set/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_template: None,
            value_template: "{{ float(value) }}".to_string(),
            unique_id: format!("lxp_{}_number_{:?}", dl, register),
            device: self.device(),
            availability: self.availability(),
            min,
            max,
            step: 1.0,
            unit_of_measurement: unit.to_string(),
            device_class: device_class.map(|s| s.to_string()),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("number", &format!("{:?}", register)),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    /// Number entity for registers stored in tenths (0.1V or 0.1kW)
    fn number_scaled_10(
        &self,
        register: Register,
        label: &str,
        min: f64,
        max: f64,
        unit: &str,
        device_class: Option<&str>,
    ) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let config = Number {
            name: label.to_string(),
            state_topic: format!(
                "{}/{}/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_topic: format!(
                "{}/cmd/{}/set/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_template: Some("{{ (value | float * 10) | round(0) | int }}".to_string()),
            value_template: "{{ (value | float / 10) | round(1) }}".to_string(),
            unique_id: format!("lxp_{}_number_{:?}", dl, register),
            device: self.device(),
            availability: self.availability(),
            min,
            max,
            step: 0.1,
            unit_of_measurement: unit.to_string(),
            device_class: device_class.map(|s| s.to_string()),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("number", &format!("{:?}", register)),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    /// Select entity backed by a decoded JSON field in `hold/{register}/bits`
    fn select_bits(
        &self,
        register: u16,
        name: &str,
        json_field: &str,
        label: &str,
        options: &[&str],
    ) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let config = Select {
            name: label.to_string(),
            state_topic: format!(
                "{}/{}/hold/{}/bits",
                self.mqtt_config.namespace(),
                dl,
                register,
            ),
            command_topic: format!(
                "{}/cmd/{}/set/{}",
                self.mqtt_config.namespace(),
                dl,
                name,
            ),
            command_template: None,
            value_template: format!("{{{{ value_json.{} }}}}", json_field),
            options: options.iter().map(|s| s.to_string()).collect(),
            unique_id: format!("lxp_{}_select_{}", dl, name),
            device: self.device(),
            availability: self.availability(),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("select", name),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    /// Select entity backed by a whole holding register whose raw integer values match the option labels
    fn select_direct_register(
        &self,
        register: Register,
        name: &str,
        label: &str,
        options: &[&str],
    ) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let config = Select {
            name: label.to_string(),
            state_topic: format!(
                "{}/{}/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_topic: format!(
                "{}/cmd/{}/set/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_template: None,
            value_template: "{{ value }}".to_string(),
            options: options.iter().map(|s| s.to_string()).collect(),
            unique_id: format!("lxp_{}_select_{}", dl, name),
            device: self.device(),
            availability: self.availability(),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("select", name),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    /// Select entity backed by a whole holding register with integer <-> label mapping via Jinja2
    fn select_mapped_register(
        &self,
        register: Register,
        name: &str,
        label: &str,
        entries: &[(u16, &str)],
    ) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let val_map = entries
            .iter()
            .map(|(k, v)| format!("'{}': '{}'", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        let cmd_map = entries
            .iter()
            .map(|(k, v)| format!("'{}': {}", v, k))
            .collect::<Vec<_>>()
            .join(", ");

        let config = Select {
            name: label.to_string(),
            state_topic: format!(
                "{}/{}/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_topic: format!(
                "{}/cmd/{}/set/hold/{}",
                self.mqtt_config.namespace(),
                dl,
                register as u16,
            ),
            command_template: Some(format!("{{{{ {{{}}}[value] }}}}", cmd_map)),
            value_template: format!("{{{{ {{{}}}.get(value | string, value) }}}}", val_map),
            options: entries.iter().map(|(_, v)| v.to_string()).collect(),
            unique_id: format!("lxp_{}_select_{}", dl, name),
            device: self.device(),
            availability: self.availability(),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("select", name),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    // Models a time range as an MQTT Text field taking values like: 00:00-23:59
    fn time_range(&self, name: &str, label: &str) -> Result<mqtt::Message> {
        let dl = self.datalog_str();
        let config = Text {
            name: label.to_string(),
            state_topic: format!("{}/{}/{}", self.mqtt_config.namespace(), dl, name),
            command_topic: format!(
                "{}/cmd/{}/set/{}",
                self.mqtt_config.namespace(),
                dl,
                name,
            ),
            command_template: r#"{% set parts = value.split("-") %}{"start":"{{ parts[0] }}", "end":"{{ parts[1] }}"}"#.to_string(),
            value_template: r#"{{ value_json["start"] }}-{{ value_json["end"] }}"#.to_string(),
            unique_id: format!("lxp_{}_text_{}", dl, name),
            device: self.device(),
            availability: self.availability(),
            pattern: r"([01]?[0-9]|2[0-3]):[0-5][0-9]-([01]?[0-9]|2[0-3]):[0-5][0-9]".to_string(),
        };

        Ok(mqtt::Message {
            topic: self.ha_discovery_topic("text", name),
            retain: true,
            payload: serde_json::to_string(&config)?,
        })
    }

    fn unique_id(&self, name: &str) -> String {
        format!("lxp_{}_{}", self.datalog_str(), name)
    }

    fn device(&self) -> Device {
        let dl = self.datalog_str();
        Device {
            identifiers: [format!("lxp_{}", dl)],
            manufacturer: "EG4 Electronics".to_owned(),
            model: "EG4 6000XP".to_owned(),
            name: format!("EG4 6000XP ({})", dl),
        }
    }

    fn availability(&self) -> Availability {
        Availability {
            topic: format!("{}/LWT", self.mqtt_config.namespace()),
        }
    }
}
