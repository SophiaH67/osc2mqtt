use async_osc::{OscMessage, OscType};
use paho_mqtt::{AsyncClient, Message};
use serde_json::json;

#[derive(Hash, Clone)]
pub struct HassEntity {
    hass_name: String,
    sensor_type: String,
    unique_id: String,
    hass_device_class: Option<String>,
    state_topic: String,
    pub command_topic: String,
}

impl PartialEq for HassEntity {
    fn eq(&self, other: &Self) -> bool {
        self.hass_name == other.hass_name
    }
}

impl Eq for HassEntity {}

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

        HassEntity {
            unique_id: "osc.".to_owned() + &osc_address_segments.join("_"),
            hass_name: hass_name.clone(),
            sensor_type: hass_sensor_type.clone(),
            hass_device_class: hass_device_class,
            state_topic: format!("homeassistant/{}/{}/state", hass_sensor_type, hass_name),
            command_topic: format!("homeassistant/{}/{}/set", hass_sensor_type, hass_name),
        }
    }
}

async fn register_device(osc_to_hass_mapping: &HassEntity, client: &AsyncClient) {
    let hass_config_topic = format!(
        "homeassistant/{}/{}/config",
        osc_to_hass_mapping.sensor_type, osc_to_hass_mapping.hass_name
    );

    let hass_config = json!({
        "name": osc_to_hass_mapping.hass_name,
        "device_class": osc_to_hass_mapping.hass_device_class,
        "state_topic": osc_to_hass_mapping.state_topic,
        "command_topic": osc_to_hass_mapping.command_topic,
        "object_id": osc_to_hass_mapping.unique_id,
        "suggested_area": "Osc",
    });

    let message = Message::new(hass_config_topic, hass_config.to_string(), 0);
    client.publish(message).await.unwrap();
}

fn osc_arg_to_string(osc_arg: &OscType) -> String {
    match osc_arg {
        OscType::Bool(value) => {
            if *value {
                "ON".to_string()
            } else {
                "OFF".to_string()
            }
        }
        OscType::Float(value) => value.to_string(),
        OscType::Int(value) => value.to_string(),
        _ => panic!("Unsupported OSC type"),
    }
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
    message: &OscMessage,
    client: &AsyncClient,
    hass_entity: &HassEntity,
) {
    let osc_args = message.args.to_owned();

    let message = Message::new(&hass_entity.state_topic, osc_arg_to_string(&osc_args[0]), 0);
    client.publish(message).await.unwrap();
}
