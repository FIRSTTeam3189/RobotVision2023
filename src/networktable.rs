use std::{
    fmt::Display,
    net::{SocketAddr, ToSocketAddrs, Ipv4Addr, IpAddr},
    time::Duration, str::FromStr,
};

use crate::AprilTagFamily;
use crossbeam_channel::{bounded, Receiver, Sender};
use log::{debug, info, warn};
use network_tables::{*, v4::Client, };
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
    pub client: network_tables::v4::Client,
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
    pub async fn new(addr: &str, port: u16) -> NetworkTableI {
        debug!("connecting to network tables at {addr}");
        let addr = Ipv4Addr::from_str(&addr).unwrap();
        let client = match tokio::time::timeout(
            Duration::from_secs(5),
            network_tables::v4::Client::new(
            SocketAddr::new(IpAddr::V4(addr),
            port)),
        ).await {
            Ok(thing) => thing,
            Err(err) => panic!("connecting to network tables failed. [{err}]"),
        };

        NetworkTableI {
            client: client,
        }
    }

    pub async fn write_topic(&self, name: &str, entry: VisionMessage) /*-> Option<u16>*/ {
        match entry {
            VisionMessage::NoTargets => {
                info!("Publishing Topic....");
                self.client.publish_topic(name, network_tables::v4::Type::Float, None).await.unwrap();
                info!("Topic Published!");
            }

            VisionMessage::AprilTag { tagtype, distance, id } => {

            }

            VisionMessage::Contours { found, size } => {

            }
        }
    }

    pub async fn read_topic(&self) {
        // self.client.subscribe(topic_names)
        
    }
}
