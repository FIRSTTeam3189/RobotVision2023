use crossbeam_channel::{bounded, Receiver, Sender, TrySendError};
use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use log::{debug, trace, warn};
use parking_lot::Mutex;

use eframe::egui;
use egui::{ColorImage, TextureHandle};
use image::{imageops::{self, filter3x3}, ImageBuffer, Rgba};
use nokhwa::{
    pixel_format::{LumaFormat, RgbAFormat},
    threaded::CallbackCamera,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Buffer,
};
use once_cell::sync::OnceCell;
use std::{env, sync::Arc, time::Duration};
use tokio::runtime::Runtime;
use vision::{process::Processing, DetectorParameters, DynamicImage, RgbaImage};

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
    // Uncomment to list available cameras on the system
    // use nokhwa::{query, utils::ApiBackend};
    // let cameras = query(ApiBackend::Auto).unwrap();
    // cameras.iter().for_each(|cam| trace!("{:?}", cam));

    // let test = DetectorParameters::default();
    // std::fs::write("test.toml", toml::to_vec(&test)?)?;
    // Create sender/receiver
    let (tx, rx) = bounded(1);
    let (process_tx, process_rx) = bounded(1);
    IMAGE_SENDER.set(Arc::new(Mutex::new(tx))).unwrap();

    // Initialize camera, request the highest possible framerate
    let format = RequestedFormatType::AbsoluteHighestFrameRate;
    let format = RequestedFormat::new::<RgbAFormat>(format);
    let mut camera = CallbackCamera::new(CameraIndex::Index(0), format, callback).unwrap();
    debug!("Created Camera!!!!");

    //Start processing thread
    let process = Processing::load(rx, process_tx, env::current_dir()?)?;
    debug!("Loaded PROCESSING");
    let rt = Runtime::new()?;
    let handle = rt.handle().clone();
    let _handle = std::thread::spawn(|| vision::process::process_thread(process, handle));
    debug!("Started Processing thread!");
    // Open camera stream, start GUI then when GUI exits, close the stream
    camera.open_stream().unwrap();
    // let options = eframe::NativeOptions::default();
    // eframe::run_native(
    //     "Webcam",
    //     options,
    //     Box::new(|_cc| Box::new(WebcamApp::new(process_rx))),
    // );
    loop {
        std::thread::sleep(Duration::from_secs(1));
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

/// egui application to display the current webcam frame
pub struct WebcamApp {
    image: Option<ColorImage>,
    texture: Option<TextureHandle>,
    image_receiver: Receiver<RgbaImage>,
    count: usize,
    frames_recved: usize,
}

impl WebcamApp {
    /// Creates a new instance of the webcam feed. This takes in the receiver the webcam frames will be received on.
    pub fn new(image_receiver: Receiver<RgbaImage>) -> WebcamApp {
        WebcamApp {
            image: None,
            texture: None,
            image_receiver,
            count: 0,
            frames_recved: 0,
        }
    }
}

impl eframe::App for WebcamApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // See if there is a new image to load
            if let Some(frame) = self.image.take() {
                self.texture = Some(ui.ctx().load_texture("frame", frame, Default::default()))
            }

            // If there is some texture to load in, show it as the image on the egui window. Otherwise, just show the spinner
            if let Some(texture) = self.texture.as_ref() {
                ui.image(texture, ui.available_size());
            } else {
                ui.spinner();
            }

            ctx.request_repaint();
        });
    }

    fn post_rendering(&mut self, _window_size_px: [u32; 2], _frame: &eframe::Frame) {
        // Try and see if there is an image coming in from the camera thread
        if let Ok(frame) = self.image_receiver.try_recv() {
            // Save an image every 60 frames
            // Increment Frames received
            self.frames_recved += 1;
            if self.frames_recved % 60 == 0 {
                self.count += 1;

                #[cfg(feature = "save-pix")]
                {
                    debug!("Saving Images");
                    let path = format!("images/image-{}.jpg", self.count);
                    let grayscale_frame = DynamicImage::from(frame.clone()).into_luma8();
                    if let Err(e) = grayscale_frame.save(path) {
                        warn!("Failed to save image: {e}");
                    }
                }
                #[cfg(not(feature = "save-pix"))]
                trace!("{} frames received", 60 * self.count)
            }

            // Get the pixel frame data and create a new ColorImage
            let size = [frame.width() as _, frame.height() as _];
            let image_buffer = frame.as_flat_samples();
            let image = ColorImage::from_rgba_unmultiplied(size, image_buffer.as_slice());
            self.image = Some(image)
        }
    }
}
