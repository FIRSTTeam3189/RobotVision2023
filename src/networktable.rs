// use nt::*;


// static IP: String = String::from("172.22.11.2");

// pub enum ClientCallBackType {
//     ClientConnected(ConnectionCallbackType::ClientConnected),
//     ClientDisconnected,
// }

// pub enum NTCallbackTypes {
//     Add,
//     Delete,
//     Update,
// }

// pub struct NetworkTableI {
//     client_name: String,
//     client: nt::NetworkTables<>,
//     connection_callback: ClientCallBackType,
//     callback_types: NTCallbackTypes
// }

// impl NetworkTableI {
//     pub async fn new(name: String) -> Result<NetworkTables<Client>> {
//         let nt = NetworkTableI {
//             client_name: name,
//             client: NetworkTables::connect(&IP, &name).await,
//             connection_callback,
//             callback_types
//         };
//         nt.client.add_connection_callback(nt.connection_callback.ClientConnected, |_| {
//             println!("Network Tables Connected!");
//         });
//         nt
//     }

// }