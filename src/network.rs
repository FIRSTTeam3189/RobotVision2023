use std::{net::TcpStream, io::{Write, Read, self, BufRead}};

use log::*;

use crate::AprilTagFamily;

pub struct Network {
    pub reader: io::BufReader<TcpStream>,
    pub writer: io::BufWriter<TcpStream>
}

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

impl Network {
    pub fn new(addr: &str) -> Network {
        let client = TcpStream::connect(addr).unwrap();
        let writer = io::BufWriter::new(client.try_clone().unwrap());
        let reader = io::BufReader::new(client);
        Network {
            reader,
            writer
        }
    }

    pub fn write(&mut self, message: VisionMessage) {
        debug!("Writing...");
        match message {
            VisionMessage::NoTargets => {
                let mess = self.writer.write(&[0]).unwrap();
                debug!("bytes written: {}", mess);
            },
            VisionMessage::AprilTag { tagtype, distance, id } => {

            },
            VisionMessage::Contours { found, size } => {

            }
        }

        self.writer.flush();

    }

    pub fn read(&mut self) {
        let mut line = String::new();
        let mess = self.reader.read_line(&mut line).unwrap();
        debug!("read: {0} {1}", line, mess);
    }
}
