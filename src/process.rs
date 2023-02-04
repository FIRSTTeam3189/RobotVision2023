use crate::{CalibrationError, CameraCalibration, DetectorParameters, RgbaImage, networktable::NetworkTableI};
use apriltag::{DetectorBuilder, Detector};
use crossbeam_channel::{Receiver, RecvError, SendError, Sender};
use image::{DynamicImage, Rgba, Pixel, Luma, ImageBuffer};
use imageproc::{self, rect::Rect, definitions::{HasWhite, HasBlack}, morphology, distance_transform::Norm, contours, geometry};

use std::{path::Path};

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

#[derive(Clone)]
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
}

pub async fn process_thread(params: Processing) -> ProcessResult<()> {
    const ARC_LENGTH_MIN: f64 = 20.0;

    let image_rx = params.image_rx;
    let calibration = params.calibration;
    let parameters = params.parameters;
    let sender = params.sender;

    let val = parameters.cli.clone();

    let aspect_ratio_max: f64 = val.aspect_max;
    let aspect_ratio_min: f64 = val.aspect_min;

    let blue = Rgba([0u8, 0u8, 255u8, 255u8]);
    // rectangle: Rect::at(130, 10).of_size(200, 200);


    let net = NetworkTableI::new("name".to_string()).await;

    let mut detector = detector_creator(&parameters);
    // NetworkTableI::write(net.client, "Test-Entry".to_string());

    let tag_params = (&calibration).into();

    loop {
        // `image` is a dynamic image.
        // `grayscale` is the image sent to the AprilTag detector to find tags
        // `frame` is used as a display for the UI.
        let image = image_rx.recv()?;
        let mut frame = image.to_rgba8();
        // Color boundaries
        let rb = vec![val.rmin as u8, val.rmax as u8];
        let gb = vec![val.gmin as u8, val.gmax as u8];
        let bb = vec![val.bmin as u8, val.bmax as u8];

        let mut mask_p = mask_maker(&frame, rb, gb, bb);
        morphology::open_mut(&mut mask_p, Norm::L1, 2);
        let found_contours = contours::find_contours::<i32>(&mask_p);
        let mut accepted_contours: Vec<contours::Contour<i32>> = Vec::new();
        for contour in found_contours {
            if geometry::arc_length(contour.points.as_slice(), true) > ARC_LENGTH_MIN {
                let min_area = geometry::min_area_rect(contour.points.as_slice());
                // min_area set as: [top left, top right, bottom right, bottom left]
                let aspect_ratio = ((min_area[0].x - min_area[1].x) as f64)/((min_area[0].y - min_area[3].y) as f64);
                if aspect_ratio_min < aspect_ratio && aspect_ratio < aspect_ratio_max {
                    accepted_contours.push(contour);
                }
            }
        }

        // Do the actual proccessing here
        let grayscale = image.into_luma8();
        let detections =  detector.detect(grayscale);
        let rects: Vec<Rect> = detections
            .iter()
            .filter_map(|x| {
                if let Some(_pose) = x.estimate_tag_pose(&tag_params) {
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

                    if hx <= lx || hy <= ly || x.decision_margin() < 1100.0 {
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
            frame = imageproc::drawing::draw_filled_rect(&frame, rect, blue);
        }

        match sender.try_send(DynamicImage::from(mask_p).to_rgba8()) {
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

fn detector_creator(parameters: &DetectorParameters) -> Detector {
    let detector = DetectorBuilder::new();
    let detector = parameters
        .families
        .iter()
        .fold(detector, |d, f| d.add_family_bits(f.into(), 1)
    );

    let mut detector = detector.build().unwrap();
    detector.set_thread_number(8);
    // detector.set_debug(true);
    detector.set_decimation(parameters.cli.decimation);
    detector.set_shapening(parameters.cli.shapening);
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

    detector
}

fn mask_maker(frame: &ImageBuffer<Rgba<u8>, Vec<u8>>, rb: Vec<u8>, gb: Vec<u8>, bb: Vec<u8>) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let mut mask_p = ImageBuffer::from_pixel(frame.width(), frame.height(), Luma::<u8>::black());
        frame.enumerate_pixels().for_each(|(x, y, p)| {
            let p = p.to_rgba();
            if p[1] > gb[0] && p[1] < gb[1] && p[0] > rb[0] && p[0] < rb[1] && p[2] > bb[0] && p[2] < bb[1]{
                mask_p.put_pixel(x, y, Luma::<u8>::white()); 
            }
       });
    mask_p
}