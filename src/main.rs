use async_osc::{OscPacket, OscSocket, Result};
use async_std::stream::StreamExt;
use paho_mqtt as mqtt;
mod homeassistant;

#[async_std::main]
async fn main() -> Result<()> {
    let client = mqtt::AsyncClient::new("tcp://192.168.67.85:1883").unwrap();
    let conn_opts = mqtt::ConnectOptionsBuilder::new()
        .keep_alive_interval(std::time::Duration::from_secs(20))
        .clean_session(true)
        .finalize();
    client.connect(conn_opts).await.unwrap();

    println!("Registered devices with Home Assistant");

    let mut socket = OscSocket::bind("127.0.0.1:9019").await?;
    println!("Listening on {}", socket.local_addr()?);

    while let Some(packet) = socket.next().await {
        let (packet, _peer_addr) = packet?;
        match packet {
            OscPacket::Bundle(_) => {}
            OscPacket::Message(message) => {
                // Print the address and the arguments
                homeassistant::handle_message(message, &client)
                    .await
                    .unwrap();
            }
        }
    }
    Ok(())
}
