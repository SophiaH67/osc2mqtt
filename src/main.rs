use async_osc::{OscPacket, OscSocket, Result};
use async_std::stream::StreamExt;

#[async_std::main]
async fn main() -> Result<()> {
    let mut socket = OscSocket::bind("localhost:9019").await?;
    println!("Listening on {}", socket.local_addr()?);

    while let Some(packet) = socket.next().await {
        let (packet, _peer_addr) = packet?;
        match packet {
            OscPacket::Bundle(_) => {}
            OscPacket::Message(message) => {
                // Print the address and the arguments
                println!("{}: {:?}", message.addr, message.args);
            }
        }
    }
    Ok(())
}
