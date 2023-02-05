use std::{fmt::Display, time::Duration, net::{SocketAddr, ToSocketAddrs}};

use crate::AprilTagFamily;
use crossbeam_channel::{bounded, Receiver, Sender};
use nt::{NetworkTables, Client, EntryData, EntryValue, ConnectionCallbackType};

const IP: &str = "ws://roboRIO-3189-FRC.local:1735";

pub enum VisionMessage {
    NoTargets,
    AprilTag {
        tagtype: AprilTagFamily,
        distance: f64,
        id: f64,
    },
    Contours {
        found: bool,
        size: f64,
    },
}

// pub struct NetworkThread {
//     netthread: tokio::task::JoinHandle<NetworkTableI>,
// }

pub struct NetworkTableI {
    pub client: NetworkTables<Client>,
}

pub enum NTError {
    Disconnected,
    Connected,
}

// impl NetworkThread {
//     pub async fn log(msg: VisionMessage) -> NetworkThread {
//         let (st, rt) = bounded(1);
//         let netthread = tokio::spawn(async move {
//             let net = NetworkTableI::new("Vision-Net".to_string()).await;
//             for msg in rt {
//                 match msg {
//                     VisionMessage::NoTargets => {
//                         net.write_value("found", EntryValue::Boolean(false)).await;
//                     }
//                     VisionMessage::AprilTag {
//                         tagtype,
//                         distance,
//                         id,
//                     } => {
//                         net.write_value("distance", EntryValue::Double(distance))
//                             .await;
//                         net.write_value("id", EntryValue::Double(id)).await;
//                     }
//                     VisionMessage::Contours { found, size } => {}
//                 }
//             }
//             net
//         });

//         let nthread = NetworkThread { netthread };

//         nthread
//     }
// }

impl NetworkTableI {
    pub async fn new(addr: &str, client_name: &str) -> NetworkTableI {
        let client =
            match tokio::time::timeout(Duration::from_secs(5), NetworkTables::connect(addr, &client_name))
                .await
            {
                Ok(thing) => thing,
                Err(err) => panic!("connecting to network tables failed. [{err}]"),
            };


        let nt = NetworkTableI {
            client: client.unwrap(),
        };

        nt.client.add_connection_callback(ConnectionCallbackType::ClientConnected, |l| println!("Connected! {}", l));
        nt.client.add_connection_callback(ConnectionCallbackType::ClientDisconnected, |l| println!("Disconnected! {}", l));
        
        nt
    }

    pub async fn init_value(&self, name: &str, entry: EntryValue) -> Option<u16> {
        match self
            .client.create_entry(EntryData::new(name.to_string(), 0, entry))
            .await {
                Ok(id) => {
                    println!("Wrote topic [{id}]");
                    Some(id)
                }
                Err(err) => {
                    println!("Failed writing topic [{err}]");
                    None
                }
            }
    }

    pub fn update_value(&self, key: u16, entry: EntryValue ) {
        self.client.update_entry(key, entry);
    }
}
