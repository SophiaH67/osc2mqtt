use async_osc::{OscMessage, OscPacket, OscSocket, Result};
use async_std::{stream::StreamExt, sync::Mutex};
use bimap::BiMap;
use convertions::hass_arg_to_osc;
use dotenv::dotenv;
use paho_mqtt as mqtt;
use std::env;
mod convertions;
mod homeassistant;

#[async_std::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let mappings: BiMap<String, homeassistant::HassEntity> = BiMap::new();
    let mappings_mutex = Mutex::new(mappings);

    let osc_listen_address = env::var("OSC_LISTEN_ADDRESS").expect("OSC_LISTEN_ADDRESS not set");
    let osc_send_address = env::var("OSC_SEND_ADDRESS").expect("OSC_SEND_ADDRESS not set");
    let mqtt_address = env::var("MQTT_ADDRESS").expect("MQTT_ADDRESS not set");

    let mqtt_client = mqtt::AsyncClient::new(mqtt_address).unwrap();
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();
    mqtt_client.connect(conn_opts).await.unwrap();

    let osc_server_future = osc_server(&mappings_mutex, mqtt_client.clone(), osc_listen_address);
    let mqtt_server_future = mqtt_server(&mappings_mutex, mqtt_client, osc_send_address);

    futures::try_join!(osc_server_future, mqtt_server_future)?;

    Ok(())
}

async fn osc_server(
    mappings: &Mutex<bimap::BiMap<String, homeassistant::HassEntity>>,
    client: mqtt::AsyncClient,
    osc_listen_address: String,
) -> Result<()> {
    let mut socket = OscSocket::bind(osc_listen_address).await?;
    println!("Listening on {}", socket.local_addr()?);

    while let Some(packet) = socket.next().await {
        let (packet, _peer_addr) = packet?;
        match packet {
            OscPacket::Bundle(_) => {}
            OscPacket::Message(message) => {
                let mut mappings = mappings.lock().await;

                let hass_entity =
                    homeassistant::get_or_register_mapping(&message, &client, &mappings).await;

                if !mappings.contains_left(&message.addr) {
                    mappings.insert(message.addr.clone(), hass_entity.clone());
                }
                drop(mappings);

                homeassistant::update_entity_state(&message.args[0], &client, &hass_entity).await;
            }
        }
    }

    Ok(())
}

async fn mqtt_server(
    mappings: &Mutex<bimap::BiMap<String, homeassistant::HassEntity>>,
    mut mqtt_client: mqtt::AsyncClient,
    osc_send_address: String,
) -> Result<()> {
    let mut mqtt_stream = mqtt_client.get_stream(100);
    let socket = OscSocket::bind("0.0.0.0:0").await?;
    socket.connect(osc_send_address).await?;

    mqtt_client.subscribe("homeassistant/#", 1).await.unwrap();

    while let Some(message) = mqtt_stream.next().await {
        let message = message.unwrap();
        let topic = message.topic();
        let payload = message.payload_str();

        let mappings = mappings.lock().await;
        // Loop through all mappings and check if the topic matches the state topic of a mapping
        let mapping = mappings
            .iter()
            .find(|(_, hass_entity)| hass_entity.command_topic == topic);

        // If no mapping is found, continue
        if mapping.is_none() {
            continue;
        }

        let (osc_address, _hass_entity) = mapping.unwrap();

        // If the payload is ON or OFF, convert it to a boolean
        let payload = hass_arg_to_osc(payload.to_string());
        let packet = OscPacket::Message(OscMessage {
            addr: osc_address.to_string(),
            args: vec![payload.clone()],
        });

        // Send the OSC message
        socket
            .send(packet)
            .await
            .expect("Failed to send OSC message");

        // Update the home assistant entity state
        homeassistant::update_entity_state(&payload, &mqtt_client, &mapping.unwrap().1).await;
        drop(mappings);
    }

    Ok(())
}
