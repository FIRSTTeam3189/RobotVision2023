use crate::{ CalibrationError, CameraCalibration, DetectorParameters, RgbaImage, networktable::{NetworkTableI, VisionMessage} };
use apriltag::{Detector, DetectorBuilder};
use crossbeam_channel::{Receiver, RecvError, SendError, Sender, TrySendError};
use image::{DynamicImage, ImageBuffer, Luma, Pixel, Rgba};
use imageproc::{ self,/* contours, */ definitions::{HasBlack, HasWhite}/*, distance_transform::Norm, geometry, morphology, rect::Rect */};
use log::*;
use tokio::{runtime::Handle};
use std::{ path::Path};

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

pub struct CustomPose {
    distance: f64,
    id: usize,
    translation_matrix: [f64; 3],
    rotation_matrix: f64
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
        trace!("loaded Calibration from: {}", cal_path.display());
        let detect_path = path.join(Self::DETECTOR_PERAMS_FILE_NAME);
        trace!("loaded Detector Parameters from: {}", detect_path.display());

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

pub fn process_thread(params: Processing, handle: Handle) -> ProcessResult<()> {
    const _ARC_LENGTH_MIN: f64 = 20.0;

    let image_rx = params.image_rx;
    let calibration = params.calibration;
    let parameters = params.parameters;
    let sender = params.sender;

    let val = parameters.cli.clone();

    // Color boundaries
    let _rb = vec![val.rmin as u8, val.rmax as u8];
    let _gb = vec![val.gmin as u8, val.gmax as u8];
    let _bb = vec![val.bmin as u8, val.bmax as u8];

    let _aspect_ratio_max: f64 = val.aspect_max;
    let _aspect_ratio_min: f64 = val.aspect_min;

    // rectangle: Rect::at(130, 10).of_size(200, 200);

    let mut detector = detector_creator(&parameters);
    let tag_params = (&calibration).into();

    let net = handle.block_on(NetworkTableI::new(&parameters.network_table_addr, parameters.network_table_port));

    let (net_tx, net_rx) = crossbeam_channel::bounded(5);
    // let (tagproc_tx, tagproc_rx) = crossbeam_channel::bounded(5);
    

    handle.spawn(async move {
        loop {
            let message = net_rx.recv().unwrap();
            net.write_topic(message).await;
            //net.read_topic().await;
        }
    });

    debug!("Process & thread Init Complete!!!!!!!!!!!!!!!!!");
    loop {
        // `image` is a dynamic image.
        // `grayscale` is the image sent to the AprilTag detector to find tags
        // `frame` is used as a display for the UI.
        let image = image_rx.recv()?;
        let frame = image.to_rgba8();

        // let mut mask_p = mask_maker(&frame, rb, gb, bb);
        // morphology::open_mut(&mut mask_p, Norm::L1, 2);
        // let found_contours = contours::find_contours::<i32>(&mask_p);
        // let mut accepted_contours: Vec<contours::Contour<i32>> = Vec::new();
        // for contour in found_contours {
        //     if geometry::arc_length(contour.points.as_slice(), true) > ARC_LENGTH_MIN {
        //         let min_area = geometry::min_area_rect(contour.points.as_slice());
        //         // min_area set as: [top left, top right, bottom right, bottom left]
        //         let aspect_ratio = ((min_area[0].x - min_area[1].x) as f64)
        //             / ((min_area[0].y - min_area[3].y) as f64);
        //         if aspect_ratio_min < aspect_ratio && aspect_ratio < aspect_ratio_max {
        //             accepted_contours.push(contour);
        //         }
        //     }
        // }

        // Do the actual proccessing here
        let grayscale = image.into_luma8();
        let detections = detector.detect(&grayscale);
        let custom_poses: Vec<CustomPose> = detections
            .iter()
            .filter_map(|x| {
                if let Some(_pose) = x.estimate_tag_pose(&tag_params) {
                    let translation_matrix = _pose.translation().data().clone();
                    let translation_matrix = [translation_matrix[2],translation_matrix[0],translation_matrix[1]];
                    let rotation_matrix = _pose.rotation().data().clone()[8];
                    // The Z coordinates of the rotation matrix is sent to the network tables

                    let c = x.corners();

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

                    if hx <= lx || hy <= ly || x.decision_margin() < 1150.0 {
                        None
                    } else {
                        // Find distance from camera to AprilTag
                        // If distance is less than shortest distance, the pose becomes the new
                        // closest distance later
                        let distance: f64 = f64::sqrt((translation_matrix[0] * translation_matrix[0]) + (translation_matrix[1] * translation_matrix[1]));
                        // hx = (hx - center[0]) * 2.0;
                        // hy = (hy - center[1]) * 2.0;

                        // debug!("translation: {:?}", _pose.translation());
                        // debug!("rotations: {:?}", _pose.rotation());
                        Some(CustomPose{distance, id: x.id(), translation_matrix, rotation_matrix})
                    }
                } else {
                    None
                }
            })
            .collect();

        if custom_poses.len() > 0 {
            let mut closest_distance: f64 = custom_poses[0].distance;
            let mut closest_pose: CustomPose = CustomPose{
                distance: custom_poses[0].distance,
                id: custom_poses[0].id,
                translation_matrix: custom_poses[0].translation_matrix,
                rotation_matrix: custom_poses[0].rotation_matrix
            };

            for pose in custom_poses {
                if pose.distance < closest_distance {
                    closest_distance = pose.distance;
                    closest_pose = CustomPose{
                        distance: pose.distance,
                        id: pose.id,
                        translation_matrix: pose.translation_matrix,
                        rotation_matrix: pose.rotation_matrix
                    };
                }
            }

            match net_tx.try_send(VisionMessage::AprilTag { 
                id: closest_pose.id as i32,
                translation_matrix: closest_pose.translation_matrix,
                rotation_matrix: closest_pose.rotation_matrix,
            }) {
                Ok(_) => {}
                Err(TrySendError::Full(_)) => {
                    // debug!("Dropping Data");
                }
                Err(TrySendError::Disconnected(_)) => {
                    // warn!("Disconnected to Channel");
                }
            }
        } else {
            match net_tx.try_send(VisionMessage::NoTargets) {
                Ok(_) => {}
                Err(TrySendError::Full(_)) => {
                    // debug!("Dropping Data");
                }
                Err(TrySendError::Disconnected(_)) => {
                    // warn!("Disconnected to Channel");
                }
            }
        }
            
            
        // if rects.is_empty() {}
        // for rect in rects {
        //     frame = imageproc::drawing::draw_filled_rect(&frame, rect, blue);    
        // }
        match sender.try_send(DynamicImage::from(frame).to_rgba8()) {
            Ok(_) => {}
            Err(crossbeam_channel::TrySendError::Full(_)) => {
                debug!("UI Thread is busy, skipping frame...")
            }
            Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                debug!("UI Thread disconnected! Breaking loop...");
                break;
            }
        }
    }
    // std::mem::drop(detector);
    Ok(())
}

fn detector_creator(parameters: &DetectorParameters) -> Detector {
    let detector = DetectorBuilder::new();
    let detector = parameters
        .families
        .iter()
        .fold(detector, |d, f| d.add_family_bits(f.into(), 1));

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
        deglitch: (false),
    });

    detector
}

fn _mask_maker(
    frame: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    rb: Vec<u8>,
    gb: Vec<u8>,
    bb: Vec<u8>,
) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    let mut mask_p = ImageBuffer::from_pixel(frame.width(), frame.height(), Luma::<u8>::black());
    frame.enumerate_pixels().for_each(|(x, y, p)| {
        let p = p.to_rgba();
        if p[1] > gb[0]
            && p[1] < gb[1]
            && p[0] > rb[0]
            && p[0] < rb[1]
            && p[2] > bb[0]
            && p[2] < bb[1]
        {
            mask_p.put_pixel(x, y, Luma::<u8>::white());
        }
    });
    mask_p
}
