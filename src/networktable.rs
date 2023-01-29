<<<<<<< HEAD
use nt::*;
=======
// use nt::*;
// use std::result::Result;
>>>>>>> 33b74d256952fd0f2d85d7b520c36a35bb29d7ea

// const IP: &str = "172.22.11.2";

// pub struct NetworkTableI {
//     client_name: String,
//     client: NetworkTables<Client>,
// }

// pub enum NTError{
//     Disconnected,
//     Connected
// }

<<<<<<< HEAD
impl NetworkTableI {
    pub async fn new(name: String) -> NetworkTableI {
        let client = NetworkTables::connect(IP, &name).await.unwrap();
        let nt = NetworkTableI {
            client_name: name,
            client: client,
        };
        nt.client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
            println!("Client Disconnected!");
            // return Err(NTError::Disconnected);
        });
        nt
    }

    pub async fn write(client: NetworkTables<Client>, entry: String) {
        let id = client
        .create_entry(EntryData::new(
            entry,
            0,
            EntryValue::Double(5.0),
        )).await;
        id.unwrap();
        for (id, value) in client.entries() {
            println!("{} ==> {:?}", id, value);
        }
    }
=======
// impl NetworkTableI {
//     pub async fn new(name: String) -> Result<Box<dyn std::error::Error>> {
//         let client = NetworkTables::connect(IP, &name).await;
//         let nt = NetworkTableI {
//             client_name: name,
//             client: client.unwrap(),
//         };
//         nt.client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |_| {
//             println!("Client Disconnected!");
//             return Err(NTError::Disconnected);
//         });
//         Ok(nt)
//     }
>>>>>>> 33b74d256952fd0f2d85d7b520c36a35bb29d7ea

// }