use nt::*;
use crossbeam_channel::{Receiver, Sender};

const IP: &str = "172.22.11.2";

pub struct NetworkTableI {
    pub client_name: String,
    pub client: NetworkTables<Client>,
}

pub enum NTError{
    Disconnected,
    Connected
}

impl NetworkTableI {
    pub async fn new(name: String) -> NetworkTableI  {
        let (tx, rx) = crossbeam_channel::bounded(1);
        let client_name = name.clone();
        let client = NetworkTables::connect("10.31.89.2", &name).await;
        let _ = tx.send(client);

        let client = rx.recv().unwrap();

        let nt = NetworkTableI {
            client_name,
            client: client.unwrap(),
        };
        nt.client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
            println!("Client Disconnected!");
            // return Err(NTError::Disconnected);
        });

        nt
    }

    pub async fn write(&self, entry: String) {
        let id = self.client.create_entry(EntryData::new(
            entry,
            0,
            EntryValue::Double(5.0),
        )).await;
        id.unwrap();
        for (id, value) in self.client.entries() {
            println!("{} ==> {:?}", id, value);
        }
    }

}