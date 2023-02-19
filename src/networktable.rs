use std::{
    net::{SocketAddr, Ipv4Addr, IpAddr},
    time::Duration, str::FromStr,
};

use log::{debug};
use network_tables::*;

pub enum VisionMessage {
    NoTargets,
    AprilTag {
        distance: f64,
        id: f64,
    },
    Contours {},
}

pub struct NetworkTableI {
    client: network_tables::v4::Client,
    detect_topic: network_tables::v4::PublishedTopic,
    ap_id_topic: network_tables::v4::PublishedTopic,
    ap_dist_topic: network_tables::v4::PublishedTopic,
}

pub enum NTError {
    Disconnected,
    Connected,
}


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

        let detect_topic = client.publish_topic("Vision/Detection", v4::Type::Int, None).await.unwrap();
        let ap_id_topic = client.publish_topic("Vision/AprilTag/ID", v4::Type::Float, None).await.unwrap();
        let ap_dist_topic = client.publish_topic("Vision/AprilTag/Distance", v4::Type::Float, None).await.unwrap();
        NetworkTableI {
            client,
            detect_topic,
            ap_id_topic,
            ap_dist_topic,
        }
    }

    pub async fn write_topic(&self, entry: VisionMessage) {

        match entry {
            VisionMessage::NoTargets => {
                self.client.publish_value(&self.detect_topic, &Value::Integer(0.into())).await.unwrap();
            }

            VisionMessage::AprilTag { distance, id } => {
                self.client.publish_value(&self.detect_topic, &Value::Integer(1.into())).await.unwrap();
                self.client.publish_value(&self.ap_id_topic, &Value::F64(id)).await.unwrap();
                self.client.publish_value(&self.ap_dist_topic, &Value::F64(distance)).await.unwrap();
            }

            VisionMessage::Contours { } => {
                self.client.publish_value(&self.detect_topic, &Value::Integer(2.into())).await.unwrap();
            }
        }
    }

    pub async fn read_topic(&self) {
        // let sub = self.client.subscribe(&["Vision"]).await.unwrap();
    }
}
