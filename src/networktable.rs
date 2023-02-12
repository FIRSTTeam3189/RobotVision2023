use std::{
    fmt::Display,
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};

use crate::AprilTagFamily;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info, warn};
use nt::{Client, ConnectionCallbackType, EntryData, EntryValue, NetworkTables};

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
    pub async fn new<'a>(addr: &'a str, client_name: &'a str) -> NetworkTableI {
        debug!("connecting to network tables at {addr}");
        let client = match tokio::time::timeout(
            Duration::from_secs(5),
            NetworkTables::connect(addr, &client_name),
        )
        .await
        {
            Ok(thing) => thing,
            Err(err) => panic!("connecting to network tables failed. [{err}]"),
        };

        let nt = NetworkTableI {
            client: client.unwrap(),
        };

        nt.client
            .add_connection_callback(ConnectionCallbackType::ClientConnected, |l| {
                info!("Connected! {}", l)
            });
        nt.client
            .add_connection_callback(ConnectionCallbackType::ClientDisconnected, |l| {
                info!("Disconnected! {}", l)
            });

        nt
    }

    pub async fn init_value(&mut self, name: &str, entry: EntryValue) -> Option<u16> {
        // self.client.reconnect().await;
        match self
            .client
            .create_entry(EntryData::new(name.to_string(), 0, entry))
            .await
        {
            Ok(id) => {
                debug!("Wrote topic [{id}]");
                Some(id)
            }
            Err(err) => {
                warn!("Failed writing topic [{err}]");
                None
            }
        }
    }

    pub fn update_value(&self, key: u16, entry: EntryValue) {
        self.client.update_entry(key, entry);
    }
}
