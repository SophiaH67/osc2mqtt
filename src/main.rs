use async_osc::{OscMessage, OscPacket, OscSocket, OscType, Result};
use async_std::{stream::StreamExt, sync::Mutex};
use bimap::BiMap;
use paho_mqtt as mqtt;
mod homeassistant;

#[async_std::main]
async fn main() -> Result<()> {
    let mappings: BiMap<String, homeassistant::HassEntity> = BiMap::new();
    let mappings_mutex = Mutex::new(mappings);

    let client = mqtt::AsyncClient::new("tcp://192.168.67.85:1883").unwrap();
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();
    client.connect(conn_opts).await.unwrap();

    let osc_server_future = osc_server(&mappings_mutex, client.clone());
    let mqtt_server_future = mqtt_server(&mappings_mutex, client);

    futures::try_join!(osc_server_future, mqtt_server_future)?;

    Ok(())
}

async fn osc_server(
    mappings: &Mutex<bimap::BiMap<String, homeassistant::HassEntity>>,
    client: mqtt::AsyncClient,
) -> Result<()> {
    let mut socket = OscSocket::bind("127.0.0.1:9019").await?;
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

                homeassistant::update_entity_state(&message, &client, &hass_entity).await;
            }
        }
    }

    Ok(())
}

async fn mqtt_server(
    mappings: &Mutex<bimap::BiMap<String, homeassistant::HassEntity>>,
    mut client: mqtt::AsyncClient,
) -> Result<()> {
    let mut mqtt_stream = client.get_stream(100);
    let socket = OscSocket::bind("127.0.0.1:9000").await?;

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
        let payload = hass_arg_to_osc(payload.to_string().as_str());
        let packet = OscPacket::Message(OscMessage {
            addr: osc_address.to_string(),
            args: vec![payload],
        });
        drop(mappings);

        // Send the OSC message
        socket
            .send(packet)
            .await
            .expect("Failed to send OSC message");
    }

    Ok(())
}

fn hass_arg_to_osc(arg: &str) -> OscType {
    match arg {
        "ON" => OscType::Bool(true),
        "OFF" => OscType::Bool(false),
        _ => OscType::Float(arg.parse().unwrap()),
    }
}
