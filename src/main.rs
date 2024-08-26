use std::sync::{Arc, Mutex};

use image::ImageFormat;
use rust_streamer::streaming::Streaming;
use clap::{Args, Parser, Subcommand};
use eframe::egui::{self, Color32, Key};
use std::net::Ipv4Addr;

fn is_valid_ipv4(ip: &str) -> bool {
    ip.parse::<Ipv4Addr>().is_ok()
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Send,
    Recv(ServerArgs),
}


#[derive(Args, Debug)]
struct ServerArgs {
    ip: String,
}

#[derive(Clone, Copy, PartialEq)]  // Aggiunto PartialEq per l'enum Mode
enum Mode {
    Caster,
    Receiver,
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Caster
    }
}

#[derive(PartialEq)]
enum TransmissionStatus {
    Idle,
    Casting,
    Receiving,
}

impl Default for TransmissionStatus {
    fn default() -> Self {
        TransmissionStatus::Idle
    }
}

#[derive(PartialEq, Clone)]
struct ScreenArea {
    startx: u32,
    starty: u32,
    endx: u32,
    endy: u32,
}

struct MyApp {
    _streaming: Option<Streaming>,
    current_image: Arc<Mutex<Option<egui::ColorImage>>>,
    texture: Option<egui::TextureHandle>,
    mode: Mode,
    caster_address: String,
    selected_screen_area: Option<ScreenArea>,
    transmission_status: TransmissionStatus,
    pause: bool,
    wrong_ip: bool,
    blanking_screen: bool,
    slider_value1: f32,
    slider_value2: f32,
    slider_value3: f32,
    slider_value4: f32,
}

impl MyApp {
    fn new() -> Self {
        // TODO not use a fake streaming
        let current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
            [200, 200],
            Color32::BLACK,
        ))));
        
        

        Self {
            _streaming: None,
            current_image,
            texture: None,
            mode: Mode::default(),
            caster_address: String::default(),
            selected_screen_area: None,
            transmission_status: TransmissionStatus::default(),
            pause: false,
            wrong_ip: false,
            blanking_screen: false,
            slider_value1: 0.0,
            slider_value2: 0.0,
            slider_value3: 0.0,
            slider_value4: 0.0,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Screen-Caster");

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Mode:");
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Caster, "Caster").clicked() {
                        self.mode = Mode::Caster;
                    }
                });
                ui.add_enabled_ui(self.transmission_status == TransmissionStatus::Idle, |ui| {
                    if ui.radio(self.mode == Mode::Receiver, "Receiver").clicked() {
                        self.mode = Mode::Receiver;
                    }
                });
            });

            ui.separator();

            match self.mode {
                Mode::Caster => {
                    ui.label("Select screen area:");
                    ui.horizontal(|ui| {
                        if ui.selectable_value(&mut None, self.selected_screen_area.clone(), "Total screen").clicked(){
                            self.selected_screen_area = None;
                            if let Some(s) = &self._streaming {
                                if let Streaming::Server(ss) = &s{
                                    ss.capture_fullscreen();
                                }
                            }
                        }
                        if ui.selectable_value(&mut true, self.selected_screen_area.is_some(), "Personalized area").clicked(){
                            self.selected_screen_area = Some(ScreenArea {
                                startx: 0,
                                starty: 0,
                                endx: 0,
                                endy: 0,
                            });
                        }
                        if self.selected_screen_area.is_some(){
                            ui.label("Left:");
                            ui.add(egui::Slider::new(&mut self.slider_value1, 0.0..=1920.0));
                            ui.label("Top:");
                            ui.add(egui::Slider::new(&mut self.slider_value2, 0.0..=1080.0));
                            ui.label("Right:");
                            ui.add(egui::Slider::new(&mut self.slider_value3, 0.0..=1920.0));
                            ui.label("Bottom:");
                            ui.add(egui::Slider::new(&mut self.slider_value4, 0.0..=1080.0));                         
                            
                            if let Some(Streaming::Server(ss)) = &self._streaming {
                                let startx = self.slider_value1.round() as u32;
                                let starty = self.slider_value2.round() as u32;
                                let endx = 1920 - self.slider_value3.round() as u32;
                                let endy = 1080 - self.slider_value4.round() as u32;

                                #[cfg(target_os = "linux")]
                                ss.capture_resize(startx, starty, endx, endy);
                                #[cfg(target_os = "windows")]
                                ss.capture_resize(startx, starty, endx, endy);
                                #[cfg(target_os = "macos")]
                                ss.capture_resize(startx, starty, endx, endy);
                            }
                        }
                    });
                }
                Mode::Receiver => {
                    if self.wrong_ip{
                        ui.colored_label(egui::Color32::RED, "Please insert a valid IP address!");
                    }
                    ui.label("Enter caster's address:");

                    ui.add_enabled(self.transmission_status == TransmissionStatus::Idle, |ui: &mut egui::Ui|{
                        ui.text_edit_singleline(&mut self.caster_address)
                    });
                }
            }

            ui.separator();

            match &self.transmission_status {
                TransmissionStatus::Idle => {
                    match self.mode {
                        Mode::Caster => {
                            if ui.button("Start trasmission").clicked() {
                                if let Some(s) = &self._streaming{
                                    match s {
                                        Streaming::Client(_) => {
                                            let image_clone = self.current_image.clone();
                                            let streaming = Streaming::new_server(move |bytes| {
                                                let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                    .unwrap()
                                                    .to_rgba8();
                        
                                                let size = [image.width() as usize, image.height() as usize];
                                                let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                        
                                                // println!("Received image with size {:?}", size);
                        
                                                *image_clone.lock().unwrap() = Some(image);
                                            })
                                            .unwrap();
                                            self._streaming = Some(streaming);
                                        }
                                        Streaming::Server(_) => { /* Nothing to do because it is already a streaming server */ }
                                    }
            
                                }
                                else{
                                    let image_clone = self.current_image.clone();
                                    let streaming = Streaming::new_server(move |bytes| {
                                        let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                            .unwrap()
                                            .to_rgba8();
            
                                        let size = [image.width() as usize, image.height() as usize];
                                        let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
            
                                        // println!("Received image with size {:?}", size);
            
                                        *image_clone.lock().unwrap() = Some(image);
                                    })
                                    .unwrap();
                                    self._streaming = Some(streaming);
                                }
                                if let Some(s) = &self._streaming{
                                    self.pause = false;
                                    self.blanking_screen = false;
                                    s.start().unwrap();
                                }
                                self.transmission_status = TransmissionStatus::Casting;
                            }
                        }
                        Mode::Receiver => {
                            if ui.button("Start reception").clicked() {
                                if is_valid_ipv4(&self.caster_address){
                                    self.wrong_ip = false;
                                    self.transmission_status = TransmissionStatus::Receiving;
                                    if let Some(s) = &self._streaming{
                                        match s {
                                            Streaming::Client(_) => {}
                                            Streaming::Server(_) => {
                                                let image_clone = self.current_image.clone();
                                                let streaming = Streaming::new_client(self.caster_address.clone(), move |bytes| {
                                                    let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                        .unwrap()
                                                        .to_rgba8();
                            
                                                    let size = [image.width() as usize, image.height() as usize];
                                                    let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                            
                                                    // println!("Received image with size {:?}", size);
                            
                                                    *image_clone.lock().unwrap() = Some(image);
                                                })
                                                .unwrap();
                                                self._streaming = Some(streaming);
                                            }
                                        }
                
                                    }
                                    else{
                                        let image_clone = self.current_image.clone();
                                        let streaming = Streaming::new_client(self.caster_address.clone(), move |bytes| {
                                            let image = image::load_from_memory_with_format(bytes, ImageFormat::Jpeg)
                                                .unwrap()
                                                .to_rgba8();
                    
                                            let size = [image.width() as usize, image.height() as usize];
                                            let image = egui::ColorImage::from_rgba_premultiplied(size, &image);
                    
                                            // println!("Received image with size {:?}", size);
                    
                                            *image_clone.lock().unwrap() = Some(image);
                                        })
                                        .unwrap();
                                        self._streaming = Some(streaming);
                                    }
                                    if let Some(s) = &self._streaming{
                                        self.wrong_ip = false;
                                        s.start().unwrap();
                                    }
                                }
                                else{
                                    self.wrong_ip = true;
                                }
                            }
                        }
                    }
                }
                TransmissionStatus::Casting => {
                    let input = ctx.input(|i| i.clone());
                    if !self.pause{
                        ui.label("Casting...");
                    }
                    else{
                        ui.colored_label(egui::Color32::LIGHT_RED, "Pause...");
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Stop transmission").on_hover_text("Ctrl + T").clicked() || input.key_pressed(Key::T) && input.modifiers.ctrl{
                            self._streaming.take();
                            self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                                [200, 200],
                                Color32::BLACK))));
                            self.transmission_status = TransmissionStatus::Idle;
                        }

                        if ui.add_enabled(!self.pause, egui::Button::new("Pause")).on_hover_text("Ctrl + P").clicked() || input.key_pressed(Key::P) && input.modifiers.ctrl{
                            self.pause = true;
                            if let Some(Streaming::Server(s)) = &self._streaming{
                                s.pause().unwrap();
                            }
                        }
                        if ui.add_enabled(self.pause, egui::Button::new("Resume")).on_hover_text("Ctrl + R").clicked() || input.key_pressed(Key::R) && input.modifiers.ctrl{
                            self.pause = false;
                            if let Some(Streaming::Server(s)) = &self._streaming{
                                s.start().unwrap();
                            }
                        }
                        if ui.selectable_value(&mut self.blanking_screen.clone(), true, "Blanking screen").on_hover_text("Ctrl + B").clicked() || input.key_pressed(Key::B) && input.modifiers.ctrl {
                            self.blanking_screen = !self.blanking_screen;
                            if let Some(Streaming::Server(s)) = &self._streaming {
                                if self.blanking_screen {
                                    s.blank_screen();
                                } else {
                                    s.restore_screen();
                                }
                            }
                        }
                    });

                }
                TransmissionStatus::Receiving => {
                    ui.label(format!("Receiving from {}...", self.caster_address));
                    if ui.button("Stop reception").clicked() {
                        self._streaming.take();
                        self.caster_address = String::default();
                        self.current_image = Arc::new(Mutex::new(Some(egui::ColorImage::new(
                            [200, 200],
                            Color32::BLACK))));
                        self.transmission_status = TransmissionStatus::Idle;
                    }
                }
            }

            let mut data = self.current_image.lock().unwrap();
            if let Some(image) = data.take() {
                self.texture = Some(ui.ctx().load_texture("image", image, Default::default()));
            }
            drop(data);

            if let Some(texture) = &self.texture {
                ui.add(egui::Image::from_texture(texture).shrink_to_fit());
            }
        });
    }
}

fn main() {
    let options = Default::default();
    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(MyApp::new()))
        }),
    )
    .unwrap();
    println!("Finished");
}
