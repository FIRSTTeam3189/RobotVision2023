use std::{fmt::Display, time::Duration, net::{SocketAddr, ToSocketAddrs}};

use crate::AprilTagFamily;
use crossbeam_channel::{bounded, Receiver, Sender};
use network_tables::{v4::{Client, Type, Topic, PublishedTopic}, Value};

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
    pub client: Client,
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
    pub async fn new<P : Into<SocketAddr>>(addr: P) -> NetworkTableI {
        let client =
            match tokio::time::timeout(Duration::from_secs(5), Client::try_new(addr))
                .await
            {
                Ok(thing) => thing,
                Err(err) => panic!("connecting to network tables failed. [{err}]"),
            };

        let nt = NetworkTableI {
            client: client.unwrap(),
        };
        nt
    }

    pub async fn write(&self, entry: &str) -> Option<PublishedTopic> {
        match self
            .client.publish_topic(entry, Type::Boolean, None)
            .await {
                Ok(thing) => {
                    println!("Wrote topic [{thing:?}]");
                    Some(thing)
                }
                Err(err) => {
                    println!("Failed writing topic [{err}]");
                    None
                }
            }
    }

    pub async fn write_value(&self, key: &PublishedTopic, entry: &Value ) {
        match self
            .client.publish_value(key, entry)
            .await {
                Ok(thing) => println!("Wrote topic [{thing:?}]"),
                Err(err) => println!("Failed writing topic [{err}]"),
            }
    }
}
