use async_osc::{OscMessage, OscType};
use paho_mqtt::{AsyncClient, Message};
use serde_json::json;

pub enum HassDeviceClass {
    Motion,
}

impl HassDeviceClass {
    fn value(&self) -> &str {
        match *self {
            HassDeviceClass::Motion => "motion",
        }
    }
}

struct OscToHass {
    osc_address: &'static str,
    osc_type: OscType,
    hass_name: &'static str,
    hass_device_class: HassDeviceClass,
    state_topic: &'static str,
}

impl OscToHass {
    fn sensor_type(&self) -> &'static str {
        match self.osc_type {
            OscType::Bool(_) => "binary_sensor",
            OscType::Float(_) => "sensor",
            OscType::Int(_) => "sensor",
            _ => panic!("Unsupported OSC type"),
        }
    }
}

const OSC_TO_HASS_MAPPINGS: [OscToHass; 1] = [OscToHass {
    osc_address: "/avatar/parameters/isWristVisible",
    osc_type: OscType::Bool(true),
    hass_name: "garden",
    hass_device_class: HassDeviceClass::Motion,
    state_topic: "homeassistant/binary_sensor/garden/state",
}];

pub async fn register_devices(client: &AsyncClient) {
    for osc_to_hass_mapping in OSC_TO_HASS_MAPPINGS {
        register_device(osc_to_hass_mapping, &client).await;
    }
}

async fn register_device(osc_to_hass_mapping: OscToHass, client: &AsyncClient) {
    let hass_config_topic = format!(
        "homeassistant/{}/{}/config",
        osc_to_hass_mapping.sensor_type(),
        osc_to_hass_mapping.hass_name
    );
    let hass_config = json!({
        "name": osc_to_hass_mapping.hass_name,
        "device_class": osc_to_hass_mapping.hass_device_class.value(),
        "state_topic": osc_to_hass_mapping.state_topic,
    });
    let message = Message::new(hass_config_topic, hass_config.to_string(), 0);
    client.publish(message).await.unwrap();
}

pub(crate) async fn handle_message(message: OscMessage, client: &AsyncClient) -> Result<(), &str> {
    let osc_address = message.addr;
    let osc_args = message.args;

    // Find the mapping for this OSC address
    let osc_to_hass_mapping = OSC_TO_HASS_MAPPINGS
        .iter()
        .find(|osc_to_hass_mapping| osc_to_hass_mapping.osc_address == osc_address);

    // If we don't have a mapping for this OSC address, ignore it
    if osc_to_hass_mapping.is_none() {
        return Err("No mapping found for OSC address");
    }

    // If we have a mapping, but the parameter type doesn't match, ignore it
    // TODO

    // If we have a mapping and the OSC message has the correct number of arguments, publish the state
    let message = Message::new(
        osc_to_hass_mapping.unwrap().state_topic,
        osc_arg_to_string(&osc_args[0]),
        0,
    );
    client.publish(message).await.unwrap();

    println!(
        "Published state for {}: {}",
        osc_to_hass_mapping.unwrap().hass_name,
        osc_arg_to_string(&osc_args[0])
    );

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
