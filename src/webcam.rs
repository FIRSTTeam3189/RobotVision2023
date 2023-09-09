use crossbeam_channel::{bounded, Sender, TrySendError};
use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use log::{debug, trace, warn};
use parking_lot::Mutex;

use nokhwa::{
    pixel_format::{RgbAFormat},
    threaded::CallbackCamera,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Buffer,
};
use once_cell::sync::OnceCell;
use std::{env, sync::Arc, time::Duration};
use tokio::runtime::Runtime;
use vision::{process::Processing, DynamicImage};

/// The channel on which frames are sent to the GUI
static IMAGE_SENDER: OnceCell<Arc<Mutex<Sender<DynamicImage>>>> = OnceCell::new();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let file_spec = FileSpec::default().basename("test").directory("./log/");
    let _log_file = file_spec.as_pathbuf(None);
    let _test = Logger::try_with_str("debug")? // Write all error, warn, and info messages
        .log_to_file(file_spec)
        .duplicate_to_stdout(Duplicate::Debug)
        .rotate(
            // If the program runs long enough,
            Criterion::Size(1000 * 1000 * 1000),   // - create a new file every day
            Naming::Numbers,          // - let the rotated files have a timestamp in their name
            Cleanup::KeepLogFiles(7), // - keep at most 7 log files
        )
        .start()?;

    trace!("------------------------------------ Application Starting -----------------------------------------------");

    // Might not work due to no GUI Window
    // Uncomment to list available cameras on the system
    // use nokhwa::{query, utils::ApiBackend};
    // let cameras = query(ApiBackend::Auto).unwrap();
    // cameras.iter().for_each(|cam| trace!("{:?}", cam));

    // let test = DetectorParameters::default();
    // std::fs::write("test.toml", toml::to_vec(&test)?)?;

    // Create sender/receiver
    let (tx, rx) = bounded(1);
    let (process_tx, _process_rx) = bounded(1);
    IMAGE_SENDER.set(Arc::new(Mutex::new(tx))).unwrap();

    // Initialize camera, request the highest possible framerate
    let format = RequestedFormatType::AbsoluteHighestFrameRate;
    let format = RequestedFormat::new::<RgbAFormat>(format);
    //Start processing thread
    let process = Processing::load(rx, process_tx, env::current_dir()?)?;
    let mut camera = CallbackCamera::new(CameraIndex::Index(process.camera_index()), format, callback).unwrap();
    debug!("Created Camera!!!!");
    debug!("Loaded PROCESSING");
    let rt = Runtime::new()?;
    let handle = rt.handle().clone();
    // Main Processing Thread for the image
    std::thread::spawn(|| 
        vision::process::process_thread(process, handle)
    );
    debug!("Started Processing thread!");
    // Open camera stream, start GUI then when GUI exits, close the stream
    camera.open_stream().unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        // Just for avoiding the warning this will never run
    }
    camera.stop_stream().unwrap();
    Ok(())
}

fn callback(image: Buffer) {
    // std::thread::sleep(Duration::from_millis(4));
    // Get a lock to the image sender
    let tx = IMAGE_SENDER.get().unwrap().lock();
    // Decode the image as RGBA from the webcam
    match image.decode_image::<RgbAFormat>() {
        Ok(frame) => {
            // Ship it off to the UI
            let dynamic_image = DynamicImage::from(frame);
            match tx.try_send(dynamic_image) {
                Ok(_) => {}
                Err(TrySendError::Full(_)) => {
                    debug!("Processing busy, dropping frame...");
                }
                Err(TrySendError::Disconnected(_)) => {
                    warn!("Failed to send frame -- disconnected.");
                }
            }
        }
        Err(e) => {
            warn!("Failed to decode: {e}");
        }
    }
}