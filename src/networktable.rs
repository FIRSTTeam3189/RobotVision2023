use nt::*;
use std::result::Result;

const IP: &str = "172.22.11.2";

pub struct NetworkTableI {
    client_name: String,
    client: NetworkTables<Client>,
}

pub enum NTError{
    Disconnected,
    Connected
}

impl NetworkTableI {
    pub async fn new(name: String) -> Result<Box<dyn std::error::Error>> {
        let client = NetworkTables::connect(IP, &name).await;
        let nt = NetworkTableI {
            client_name: name,
            client: client.unwrap(),
        };
        nt.client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
            println!("Client Disconnected!");
            return Err(NTError::Disconnected);
        });
        Ok(nt)
    }

}