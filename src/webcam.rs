use std::{sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
}};

use eframe::egui;
use egui::{ColorImage, TextureHandle};
use image::{ImageBuffer, Rgba, imageops};
use nokhwa::{
    pixel_format::RgbAFormat,
    threaded::CallbackCamera,
    utils::{CameraIndex, RequestedFormat, RequestedFormatType},
    Buffer,
};
use once_cell::sync::OnceCell;
use std::sync::Arc;

/// The type of image being targeted for processing. 
/// This ends up being a RGBA (32-bits/pixel) image per camera frame read
type ImgBuf = ImageBuffer<Rgba<u8>, Vec<u8>>;

/// The channel on which frames are sent to the GUI
static IMAGE_SENDER: OnceCell<Arc<Mutex<Sender<ImgBuf>>>> = OnceCell::new();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Uncomment to list available cameras on the system
    // use nokhwa::{query, utils::ApiBackend};
    // let cameras = query(ApiBackend::Auto).unwrap();
    // cameras.iter().for_each(|cam| println!("{:?}", cam));

    // Create sender/receiver
    let (tx, rx) = channel();
    IMAGE_SENDER.set(Arc::new(Mutex::new(tx))).unwrap();

    // Initialize camera, request the highest possible framerate
    let format = RequestedFormatType::AbsoluteHighestFrameRate;
    let format = RequestedFormat::new::<RgbAFormat>(format);
    let mut camera = CallbackCamera::new(CameraIndex::Index(0), format, callback).unwrap();

    // Open camera stream, start GUI then when GUI exits, close the stream
    camera.open_stream().unwrap();
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Webcam",
        options,
        Box::new(|_cc| Box::new(WebcamApp::new(rx))),
    );
    camera.stop_stream().unwrap();
    Ok(())
}

fn callback(image: Buffer) {
    // Get a lock to the image sender
    let tx = IMAGE_SENDER.get().unwrap().lock().unwrap();
    // Decode the image as RGBA from the webcam
    match image.decode_image::<RgbAFormat>() {
        Ok(frame) => {
            // Ship it off to the UI
            if let Err(e) = tx.send(frame) {
                println!("Failed to send frame: {e}");
            }
        }
        Err(e) => {
            println!("Failed to decode: {e}");
        }
    }
}

/// egui application to display the current webcam frame
pub struct WebcamApp {
    image: Option<ColorImage>,
    texture: Option<TextureHandle>,
    image_receiver: Receiver<ImgBuf>,
    count: usize,
    frames_recved: usize
}

impl WebcamApp {
    /// Creates a new instance of the webcam feed. This takes in the receiver the webcam frames will be received on.
    pub fn new(image_receiver: Receiver<ImgBuf>) -> WebcamApp {
        WebcamApp {
            image: None,
            texture: None,
            image_receiver,
            count: 0,
            frames_recved: 0
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
            // Convert to grayscale image
            let grayscale: ImageBuffer<Rgba<u8>, Vec<u8>> = imageops::grayscale_with_type(&frame);

            // Save an image every 60 frames
            // Increment Frames received
            self.frames_recved += 1;
            if self.frames_recved % 60 == 0 {
                let _path = format!("./image-{}.jpg", self.count);
                self.count += 1;
                #[cfg(feature = "save-pix")]
                if let Err(e) = grayscale.save(_path) {
                    println!("Failed to save image: {e}");
                }
                #[cfg(not(feature = "save-pix"))]
                println!("{} frames received", 60 * self.count)
            }

            // Get the pixel frame data and create a new ColorImage
            let size = [grayscale.width() as _, grayscale.height() as _];
            let image_buffer = grayscale.as_flat_samples();
            let image = ColorImage::from_rgba_unmultiplied(size, image_buffer.as_slice());
            self.image = Some(image)
        }
    }

}
