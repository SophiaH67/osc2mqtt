use async_osc::{OscMessage, OscType};
use paho_mqtt::{AsyncClient, Message};
use serde_json::json;

struct OscToHass {
    osc_type: OscType,
    hass_name: String,
    sensor_type: String,
    hass_device_class: String,
    state_topic: String,
    command_topic: String,
}

impl OscToHass {
    fn new(osc_type: OscType, hass_name: &str) -> Self {
        let hass_sensor_type = match osc_type {
            OscType::Bool(_) => "switch".to_string(),
            OscType::Float(_) => "number".to_string(),
            OscType::Int(_) => "number".to_string(),
            _ => panic!("Unsupported OSC type"),
        };
        let hass_device_class = match osc_type {
            OscType::Bool(_) => "switch".to_string(),
            OscType::Float(_) => "None".to_string(),
            OscType::Int(_) => "None".to_string(),
            _ => panic!("Unsupported OSC type"),
        };

        OscToHass {
            osc_type: osc_type.clone(),
            hass_name: hass_name.to_string(),
            sensor_type: hass_sensor_type.clone(),
            hass_device_class,
            state_topic: format!("homeassistant/{}/{}/state", hass_sensor_type, hass_name),
            command_topic: format!("homeassistant/{}/{}/set", hass_sensor_type, hass_name),
        }
    }
}

async fn register_device(osc_to_hass_mapping: &OscToHass, client: &AsyncClient) {
    let hass_config_topic = format!(
        "homeassistant/{}/{}/config",
        osc_to_hass_mapping.sensor_type, osc_to_hass_mapping.hass_name
    );

    let hass_config = json!({
        "name": osc_to_hass_mapping.hass_name,
        "device_class": osc_to_hass_mapping.hass_device_class,
        "state_topic": osc_to_hass_mapping.state_topic,
        "command_topic": osc_to_hass_mapping.command_topic,
    });

    let message = Message::new(hass_config_topic, hass_config.to_string(), 0);
    client.publish(message).await.unwrap();
}

pub(crate) async fn handle_message(message: OscMessage, client: &AsyncClient) -> Result<(), &str> {
    let osc_address = message.addr.as_str().to_owned();
    let osc_address_copy = osc_address.to_owned();
    let osc_args = message.args.to_owned();

    // Get the last non-empty segment of the OSC address
    let osc_address_segments: Vec<&str> = osc_address_copy.split("/").collect();
    let last_segment = osc_address_segments
        .iter()
        .rev()
        .find(|s| !s.is_empty())
        .unwrap();

    // Create a mapping from OSC address to Home Assistant device
    let osc_to_hass_mapping = OscToHass::new(osc_args[0].to_owned(), last_segment);

    // Register the device with Home Assistant
    register_device(&osc_to_hass_mapping, client).await;

    // If we have a mapping and the OSC message has the correct number of arguments, publish the state
    let message = Message::new(
        &osc_to_hass_mapping.state_topic,
        osc_arg_to_string(&osc_args[0]),
        0,
    );
    client.publish(message).await.unwrap();

    Ok(())
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
