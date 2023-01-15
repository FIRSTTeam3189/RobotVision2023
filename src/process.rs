use crate::{CalibrationError, CameraCalibration, DetectorParameters, ImgBuf};
use image::{imageops, Rgba};
use imageproc::{self, rect::Rect};
use std::{path::Path, sync::mpsc::{Receiver, RecvError, SendError, Sender}, thread, thread::JoinHandle};

use thiserror::Error;
#[derive(Error, Debug)]
pub enum ProcessError {
    #[error("Fail to load calibration {0}")]
    Calibration(#[from] CalibrationError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Toml error: {0}")]
    TomlDeserialization(#[from] toml::de::Error),
    #[error("Receive error: {0}")]
    Receive(#[from] RecvError),
    #[error("Send error: {0}")]
    Send(#[from] SendError<ImgBuf>),
}

pub type ProcessResult<T> = Result<T, ProcessError>;
pub struct Processing {
    image_rx: Receiver<ImgBuf>,
    calibration: CameraCalibration,
    parameters: DetectorParameters,
    sender: Sender<ImgBuf>,
}

impl Processing {
    const CAMERA_CAL_FILE_NAME: &str = "camera.cal.config";
    const DETECTOR_PERAMS_FILE_NAME: &str = "process.config";

    pub fn new(image_rx: Receiver<ImgBuf>, sender: Sender<ImgBuf>) -> Self {
        Self {
            image_rx,
            sender,
            calibration: CameraCalibration::default(),
            parameters: DetectorParameters::default(),
        }
    }

    pub fn load<T: AsRef<Path>>(
        image_rx: Receiver<ImgBuf>,
        sender: Sender<ImgBuf>,
        path: T,
    ) -> ProcessResult<Self> {
        // Create Paths to config files
        let path = path.as_ref();
        let cal_path = path.join(Self::CAMERA_CAL_FILE_NAME);
        let detect_path = path.join(Self::DETECTOR_PERAMS_FILE_NAME);

        // Contents of the files
        let cal_contents = std::fs::read_to_string(cal_path)?;
        let detect_contents = std::fs::read_to_string(detect_path)?;

        // Note: The python program gives a json file, hence why we use serde json
        // The detector parameters are written in toml
        let calibration = serde_json::from_str(&cal_contents)?;
        let parameters = toml::from_str(&detect_contents)?;

        Ok(Processing {
            image_rx,
            calibration,
            parameters,
            sender,
        })
    }

    pub fn start(self) -> JoinHandle<ProcessResult<()>> {
        thread::spawn(move || process_thread(self))
    }
}

fn process_thread(params: Processing) -> ProcessResult<()> {
    let mut image_rx = params.image_rx;
    let calibration = params.calibration;
    let parameters = params.parameters;
    let sender = params.sender;

    let blue = Rgba([0u8, 0u8, 255u8, 255u8]);
    let rectangle = Rect::at(130, 10).of_size(200, 200);

    loop {
        let image = image_rx.recv()?;
        // Convert to grayscale image
        let grayscale: ImgBuf = imageops::grayscale_with_type(&image);

        // Do the actuall proccessing here
        
        sender.send(grayscale)?;
        
    }

    Ok(())
}
