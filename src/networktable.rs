use std::{
    net::{SocketAddr, Ipv4Addr, IpAddr},
    time::Duration, str::FromStr,
};

use log::{debug};
use network_tables::*;

pub enum VisionMessage {
    NoTargets,
    AprilTag {
        id: i32,
        translation_matrix: [f64;3]
    }
}

pub struct NetworkTableI {
    client: network_tables::v4::Client,
    detect_topic: network_tables::v4::PublishedTopic,
    ap_id_topic: network_tables::v4::PublishedTopic,
    ap_tmatrix_topic: network_tables::v4::PublishedTopic
}


impl NetworkTableI {
    pub async fn new(addr: &str, port: u16) -> NetworkTableI {
        let addr = Ipv4Addr::from_str(&addr).unwrap();
        let socket_addr = SocketAddr::new(IpAddr::V4(addr),
            port);
        debug!("connecting to network tables at {socket_addr}");
        let client = match tokio::time::timeout(
            Duration::from_secs(5),
            network_tables::v4::Client::new(
                socket_addr),
        ).await {
            Ok(thing) => thing,
            Err(err) => panic!("connecting to network tables failed. [{err}]"),
        };

        let detect_topic = client.publish_topic("Vision/Detection", v4::Type::Int, None).await.unwrap();
        let ap_id_topic = client.publish_topic("Vision/AprilTag/ID", v4::Type::Int, None).await.unwrap();
        let ap_tmatrix_topic = client.publish_topic("Vision/AprilTag/TMatrix", v4::Type::FloatArray, None).await.unwrap();

        NetworkTableI { 
            client, 
            detect_topic, 
            ap_id_topic, 
            ap_tmatrix_topic 
        }
    }

    pub async fn write_topic(&self, entry: VisionMessage) {
        match entry {
            VisionMessage::NoTargets => {
                let _output = self.client.publish_value(&self.detect_topic, &Value::Integer(0.into())).await;
            }

            VisionMessage::AprilTag { id, translation_matrix } => {
                let _detect_output = self.client.publish_value(&self.detect_topic, &Value::Integer(1.into())).await;
                let _id_output = self.client.publish_value(&self.ap_id_topic, &Value::Integer(id.into())).await;
                let _t_matoutput = self.client.publish_value(&self.ap_tmatrix_topic, &Value::Array(vec![
                    Value::F64(translation_matrix[0]),
                    Value::F64(translation_matrix[1]),
                    Value::F64(translation_matrix[2])
                ])).await;
            }
        }
    }

    pub async fn read_topic(&self) {
        let mut enable_topic_sub = self.client.subscribe(&["Vision/Enable"]).await.unwrap();
        if let Some(message) = enable_topic_sub.next().await {
            let data = message.data;
            debug!("Enable: {:?}", data.as_bool());
        }
    }

}
