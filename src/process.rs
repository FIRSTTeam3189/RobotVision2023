use crate::{CalibrationError, CameraCalibration, DetectorParameters, RgbaImage};
use apriltag::DetectorBuilder;
use crossbeam_channel::{Receiver, RecvError, SendError, Sender};
use image::{imageops::{self, grayscale}, DynamicImage, Frame, Rgba};
use imageproc::{self, rect::Rect, contrast::threshold_mut};
use std::{env, path::Path, thread, thread::JoinHandle, vec};

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
    Send(#[from] SendError<RgbaImage>),
}

pub type ProcessResult<T> = Result<T, ProcessError>;
pub struct Processing {
    image_rx: Receiver<DynamicImage>,
    calibration: CameraCalibration,
    parameters: DetectorParameters,
    sender: Sender<RgbaImage>,
}

impl Processing {
    const CAMERA_CAL_FILE_NAME: &str = "cam-cal.json";
    const DETECTOR_PERAMS_FILE_NAME: &str = "process.toml";

    pub fn new(image_rx: Receiver<DynamicImage>, sender: Sender<RgbaImage>) -> Self {
        Self {
            image_rx,
            sender,
            calibration: CameraCalibration::default(),
            parameters: DetectorParameters::default(),
        }
    }

    pub fn load<T: AsRef<Path>>(
        image_rx: Receiver<DynamicImage>,
        sender: Sender<RgbaImage>,
        path: T,
    ) -> ProcessResult<Self> {
        // Create Paths to config files
        let path = path.as_ref();
        let cal_path = path.join(Self::CAMERA_CAL_FILE_NAME);
        println!("loaded Calibration from: {}", cal_path.display());
        let detect_path = path.join(Self::DETECTOR_PERAMS_FILE_NAME);
        println!("loaded Detector Parameters from: {}", detect_path.display());

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

    let detector = DetectorBuilder::new();
    let detector = parameters
        .families
        .iter()
        .fold(detector, |d, f| d.add_family_bits(f.into(), 1));

    let mut detector = detector.build().unwrap();
    detector.set_thread_number(8);
    // detector.set_debug(true);
    detector.set_decimation(parameters.cli.decimation);
    detector.set_shapening(parameters.cli.sharpening);
    detector.set_refine_edges(false);
    detector.set_sigma(0.0);
    detector.set_thresholds(apriltag::detector::QuadThresholds { 
        min_cluster_pixels: (5),
        max_maxima_number: (10),
        min_angle: (apriltag::Angle::accept_all_candidates()),
        min_opposite_angle: (apriltag::Angle::from_degrees(360.0)),
        max_mse: (10.0), 
        min_white_black_diff: (5), 
        deglitch: (false) 
    });
    let tag_params = (&calibration).into();

    loop {
        // `image` is a dynamic image.
        // `grayscale` is the image sent to the AprilTag detector to find tags
        // `frame` is used as a display for the UI.
        let image = image_rx.recv()?;
        let grayscale = image.to_luma8();
        let mut frame = image.into_rgba8();

        // Do the actual proccessing here
        let detections = detector.detect(grayscale);
        // println!("thingy: {detections:?}");
        let rects: Vec<Rect> = detections
            .iter()
            .filter_map(|x| {
                // println!("\t-{x:?}");
                if let Some(pose) = x.estimate_tag_pose(&tag_params) {
                    let c = x.corners();
                    let center = x.center();

                    let mut lx = c[0][0];
                    let mut hx = c[0][0];

                    let mut ly = c[0][1];
                    let mut hy = c[0][1];

                    for corner in c {
                        if corner[0] < lx {
                            lx = corner[0];
                        }
                        if corner[0] > hx {
                            hx = corner[0];
                        }
                        if corner[1] < ly {
                            ly = corner[1];
                        }
                        if corner[1] > hy {
                            hy = corner[1];
                        }
                    }

                    if /*   hx <= lx || hy <= ly || */ x.decision_margin() < 1000.0 {
                        None
                    } else {
                        hx = (hx - center[0]) * 2.0;
                        hy = (hy - center[1]) * 2.0;
                        Some(Rect::at(lx as i32, ly as i32).of_size(hx as u32, hy as u32))
                    }
                } else {
                    None
                }
            })
            .collect();

        for rect in rects {
            // println!("{rect:?}");
            frame = imageproc::drawing::draw_filled_rect(&frame, rect, blue);
        }

        match sender.try_send(frame) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                println!("UI Thread is busy, skipping frame...")
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                println!("UI Thread disconnected! Breaking loop...");
                break;
            }
        }
    }

    Ok(())
}
