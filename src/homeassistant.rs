use async_osc::{OscMessage, OscType};
use paho_mqtt::{AsyncClient, Message};
use serde_json::json;
use serde::Serialize;

use crate::convertions::osc_arg_to_hass;

#[derive(Hash, Clone)]
pub struct HassEntity {
    hass_name: String,
    sensor_type: String,
    unique_id: String,
    hass_device_class: Option<String>,
    state_topic: String,
    pub command_topic: String,
    hass_value_template: Option<&'static str>,
    hass_command_template: Option<&'static str>,
}

impl PartialEq for HassEntity {
    fn eq(&self, other: &Self) -> bool {
        self.hass_name == other.hass_name
    }
}

impl Eq for HassEntity {}

#[derive(Serialize)]
struct HassIntroduction {
    name: String,
    unique_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    device_class: Option<String>,
    state_topic: String,
    command_topic: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value_template: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_template: Option<&'static str>,
}

impl HassEntity {
    fn new(osc_message: &OscMessage) -> Self {
        let osc_type = &osc_message.args[0];
        // Get the last non-empty segment of the OSC address
        let osc_address = osc_message.addr.as_str().to_owned();
        let osc_address_segments: Vec<&str> = osc_address.split("/").collect();
        let last_segment = osc_address_segments
            .iter()
            .rev()
            .find(|s| !s.is_empty())
            .unwrap();
        let hass_name = "Osc".to_owned() + &last_segment.to_string();

        let hass_sensor_type = match osc_type {
            OscType::Bool(_) => "switch".to_string(),
            OscType::Float(_) => "number".to_string(),
            OscType::Int(_) => "number".to_string(),
            _ => panic!("Unsupported OSC type"),
        };
        let hass_device_class = match osc_type {
            OscType::Bool(_) => Some("switch".to_string()),
            OscType::Float(_) => None,
            OscType::Int(_) => None,
            _ => panic!("Unsupported OSC type"),
        };

        let hass_value_template = match osc_type {
            // If it's a float, add 1, then multiply by 50
            OscType::Float(_) => Some("{{ (value_json | float + 1) * 50 }}"),
            // If it's an int, divide by 2.55
            OscType::Int(_) => Some("{{ value_json | float / 2.55 }}"),
            _ => None,
        };
        let hass_command_template = match osc_type {
            // If it's a float, divide by 50, then subtract 1
            OscType::Float(_) => Some("{{ (value | float / 50) - 1 }}"),
            // If it's an int, multiply by 2.55
            OscType::Int(_) => Some("{{ value | float * 2.55 }}"),
            _ => None,
        };

        HassEntity {
            unique_id: "osc.".to_owned() + &osc_address_segments.join("_"),
            hass_name: hass_name.clone(),
            sensor_type: hass_sensor_type.clone(),
            hass_device_class: hass_device_class,
            state_topic: format!("homeassistant/{}/{}/state", hass_sensor_type, hass_name),
            command_topic: format!("homeassistant/{}/{}/set", hass_sensor_type, hass_name),
            hass_value_template: hass_value_template,
            hass_command_template: hass_command_template,
        }
    }
}

async fn register_device(osc_to_hass_mapping: &HassEntity, client: &AsyncClient) {
    let hass_config_topic = format!(
        "homeassistant/{}/{}/config",
        osc_to_hass_mapping.sensor_type, osc_to_hass_mapping.hass_name
    );

    let hass_config = HassIntroduction {
        name: osc_to_hass_mapping.hass_name.clone(),
        unique_id: osc_to_hass_mapping.unique_id.clone(),
        device_class: osc_to_hass_mapping.hass_device_class.clone(),
        state_topic: osc_to_hass_mapping.state_topic.clone(),
        command_topic: osc_to_hass_mapping.command_topic.clone(),
        value_template: osc_to_hass_mapping.hass_value_template,
        command_template: osc_to_hass_mapping.hass_command_template,
    };

    let message = Message::new(hass_config_topic, json!(hass_config).to_string(), 1);
    client.publish(message).await.unwrap();
}

pub(crate) async fn get_or_register_mapping(
    message: &OscMessage,
    client: &AsyncClient,
    mappings: &bimap::BiMap<String, HassEntity>,
) -> HassEntity {
    let osc_address = message.addr.as_str().to_owned();

    // Check if the OSC address is already mapped to a Home Assistant device
    if mappings.contains_left(&osc_address) {
        return mappings.get_by_left(&osc_address).unwrap().to_owned();
    }

    // Create a mapping from OSC address to Home Assistant device
    let osc_to_hass_mapping = HassEntity::new(&message);

    // Register the device with Home Assistant
    register_device(&osc_to_hass_mapping, client).await;

    osc_to_hass_mapping
}

pub(crate) async fn update_entity_state(
    osc_arg: &OscType,
    mqtt_client: &AsyncClient,
    hass_entity: &HassEntity,
) {
    // If it's a float, make sure it's between -1 and 1
    // If it's an int, make sure it's between 0 and 255
    let osc_arg = match osc_arg {
        OscType::Float(f) => OscType::Float(f.max(-1.0).min(1.0)),
        OscType::Int(i) => OscType::Int(*i.max(&0).min(&255)),
        _ => osc_arg.to_owned(),
    };

    let message = Message::new(&hass_entity.state_topic, osc_arg_to_hass(&osc_arg), 0);
    mqtt_client.publish(message).await.unwrap();
}
