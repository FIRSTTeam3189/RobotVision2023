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
    pub client: network_tables::v4::Client,
    pub topic: network_tables::v4::PublishedTopic
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

        let topic = client.publish_topic("Vision", v4::Type::FloatArray, None).await.unwrap();

        NetworkTableI {
            client,
            topic
        }
    }

    pub async fn write_topic(&self, entry: VisionMessage) {

        match entry {
            VisionMessage::NoTargets => {
                let data: Vec<network_tables::Value> = vec![
                    network_tables::Value::F64(0.0)
                ];
                self.client.publish_value(&self.topic, &Value::Array(data)).await.unwrap();
            }

            VisionMessage::AprilTag { distance, id } => {
                let data: Vec<network_tables::Value> = vec![
                    network_tables::Value::F64(1.0),
                    network_tables::Value::F64(distance),
                    network_tables::Value::F64(id)
                ];
                self.client.publish_value(&self.topic, &Value::Array(data)).await.unwrap();
            }

            VisionMessage::Contours { } => {
                let data: Vec<network_tables::Value> = vec![
                    network_tables::Value::F64(2.0)
                ];
                self.client.publish_value(&self.topic, &Value::Array(data)).await.unwrap();
            }
        }
    }

    pub async fn read_topic(&self) {
        // let sub = self.client.subscribe(&["Vision"]).await.unwrap();
    }
}
