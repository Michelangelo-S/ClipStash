use glium::glutin::event_loop::ControlFlow;
use glium::glutin::platform::windows::WindowBuilderExtWindows;
use glium::glutin::window::Icon;
use glium::{self, Display};
use image::io::Reader as ImageReader;
use image::GenericImageView;
use imgui::{Context, FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::sync::mpsc;
use std::{env, thread};
use systray::Application;

use std::cell::RefCell;
use std::path::PathBuf;

pub struct Skin {
    pub title_bg: [f32; 4],
    pub title_bg_active: [f32; 4],
    pub title_bg_collapsed: [f32; 4],
    pub button: [f32; 4],
    pub button_hovered: [f32; 4],
    pub button_active: [f32; 4],
    pub header: [f32; 4],
    pub header_hovered: [f32; 4],
    pub header_active: [f32; 4],
    pub frame_rounding: f32,
    //TODO: Add more?
}

// Define an enum for the different types of messages
enum TrayMessage {
    Show,
    Quit,
}

pub struct ImguiWindow {
    imgui: RefCell<imgui::Context>,
    platform: RefCell<WinitPlatform>,
    renderer: RefCell<Renderer>,
    display: RefCell<Display>,
    event_loop: glium::glutin::event_loop::EventLoop<()>,
    last_frame: std::time::Instant,
    last_window_drag: std::time::Instant,
    is_window_dragging: bool,
    is_mouse_in_window: bool,
    is_window_minimized: bool,
    redraw_next_frame: bool,
    rx: mpsc::Receiver<TrayMessage>,
}

impl ImguiWindow {
    pub fn new() -> Self {
        let imgui = RefCell::new(Context::create());
        imgui.borrow_mut().set_ini_filename(None);

        const FONT_SIZE: f32 = 16.0;

        let mut values = Vec::new();
        for range in &[
            0x0020..0x00FF, // Latin
            0x0370..0x03FF, // Greek
            0x0E00..0x0E7F, // Thai
            0x0400..0x04FF, // Russian
        ] {
            values.push(range.start);
            values.push(range.end);
        }
        values.push(0); // Add zero to the end of the array
        let values: &'static [u32] = Box::leak(values.into_boxed_slice());

        let glyph_ranges = imgui::FontGlyphRanges::from_slice(values);

        imgui.borrow_mut().fonts().add_font(&[FontSource::TtfData {
            data: include_bytes!("../resources/Roboto-Regular.ttf"),
            size_pixels: FONT_SIZE,
            config: Some(FontConfig {
                oversample_h: 2,
                pixel_snap_h: true,
                glyph_ranges: glyph_ranges,
                ..FontConfig::default()
            }),
        }]);

        let mut img_path = if cfg!(debug_assertions) {
            // In debug mode, use the project directory
            let manifest_dir =
                env::var("CARGO_MANIFEST_DIR").expect("Failed to get project directory");
            PathBuf::from(manifest_dir)
        } else {
            // In release mode, use the directory of the current executable
            let exe_path = env::current_exe().expect("Failed to get current exe path");
            exe_path
                .parent()
                .expect("Failed to get directory of current exe")
                .to_path_buf()
        };
        img_path = img_path.join("resources").join("window.ico");
        let img_path_clone = img_path.clone(); // Clone img_path before it's moved

        let image = ImageReader::open(img_path_clone)
            .unwrap()
            .decode()
            .expect("Failed to decode image");

        let (width, height) = image.dimensions();
        let rgba = image.into_rgba8();
        let icon = Icon::from_rgba(rgba.into_raw(), width, height).expect("Failed to create icon");

        let event_loop = glium::glutin::event_loop::EventLoop::new();
        let window_builder = glium::glutin::window::WindowBuilder::new()
            .with_inner_size(glium::glutin::dpi::LogicalSize::new(800.0, 600.0))
            .with_window_icon(Some(icon.clone()))
            .with_taskbar_icon(Some(icon))
            .with_title("Clipboard Manager");

        let context_builder = glium::glutin::ContextBuilder::new()
            .with_vsync(true)
            .with_double_buffer(Some(true))
            .with_hardware_acceleration(Some(true));

        let display = RefCell::new(
            glium::Display::new(window_builder, context_builder, &event_loop).unwrap(),
        );

        let platform = RefCell::new(WinitPlatform::init(&mut *imgui.borrow_mut()));

        {
            let display_ref = display.borrow_mut();
            let gl_window = display_ref.gl_window();
            let window = gl_window.window();

            platform.borrow_mut().attach_window(
                &mut imgui.borrow_mut().io_mut(),
                window,
                HiDpiMode::Default,
            );
        }

        let renderer =
            RefCell::new(Renderer::init(&mut *imgui.borrow_mut(), &*display.borrow()).unwrap());
        let last_frame = std::time::Instant::now();

        // Create a channel for sending TrayMessage from the system tray thread to the main thread
        let (tx, rx) = mpsc::channel::<TrayMessage>();
        let mut app = Application::new().unwrap();
        app.set_icon_from_file(img_path.to_str().unwrap()).unwrap(); // Use the original img_path

        let tx_clone = tx.clone(); // Clone the transmitter to use in the new thread
        let _ = app.add_menu_item("Show", move |_| {
            let _ = tx_clone.send(TrayMessage::Show);
            Ok::<_, systray::Error>(())
        });

        let _ = app.add_menu_item("Quit", move |_| {
            // Send a Quit message to the main thread
            let _ = tx.send(TrayMessage::Quit);
            Ok::<_, systray::Error>(())
        });

        // Spawn a new thread to handle the system tray
        thread::spawn(move || {
            app.wait_for_message()
                .expect_err("Failed to wait for system tray message");
        });

        Self {
            imgui,
            platform,
            renderer,
            display,
            event_loop,
            last_frame,
            last_window_drag: std::time::Instant::now(),
            is_window_dragging: false,
            is_mouse_in_window: false,
            is_window_minimized: false,
            redraw_next_frame: false,
            rx,
        }
    }

    pub fn set_skin(&mut self, skin: Skin) {
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::TitleBg as usize] =
            skin.title_bg;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::TitleBgActive as usize] =
            skin.title_bg_active;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::TitleBgCollapsed as usize] =
            skin.title_bg_collapsed;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::Button as usize] =
            skin.button;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::ButtonHovered as usize] =
            skin.button_hovered;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::ButtonActive as usize] =
            skin.button_active;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::Header as usize] =
            skin.header;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::HeaderHovered as usize] =
            skin.header_hovered;
        self.imgui.borrow_mut().style_mut().colors[imgui::StyleColor::HeaderActive as usize] =
            skin.header_active;
        self.imgui.borrow_mut().style_mut().frame_rounding = skin.frame_rounding;
    }

    pub fn render<F>(mut self, on_draw: F)
    where
        F: Fn(
                &mut imgui::Context,
                &mut Display,
                &mut WinitPlatform,
                &mut Renderer,
                &mut ControlFlow,
            ) + 'static,
    {
        let desired_frame_time_inactive = std::time::Duration::from_secs_f32(1.0 / 10.0); // 120 FPS
        let desired_frame_time_active = std::time::Duration::from_secs_f32(1.0 / 60.0); // 60 FPS
        let desired_frame_time_moving = std::time::Duration::from_secs_f32(1.0 / 120.0); // 120 FPS

        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            // Check if there's a message from the system tray thread
            match self.rx.try_recv() {
                Ok(TrayMessage::Show) => {
                    // Show the window
                    self.is_window_minimized = false;
                    let display_ref = self.display.borrow();
                    let gl_window = display_ref.gl_window();
                    gl_window.window().set_visible(true);
                }
                Ok(TrayMessage::Quit) => {
                    // It seems we need to wake up the window to trigger the event
                    self.is_window_minimized = false;
                    *control_flow = ControlFlow::Exit;
                }
                Err(_) => {}
            }

            if self.is_window_minimized {
                // If the window is minimized, wait for a message from the system tray thread
                std::thread::sleep(std::time::Duration::from_millis(200));
            }

            match event {
                glium::glutin::event::Event::NewEvents(_) => {
                    self.imgui
                        .borrow_mut()
                        .io_mut()
                        .update_delta_time(self.last_frame.elapsed());
                    self.last_frame = std::time::Instant::now();
                }

                glium::glutin::event::Event::MainEventsCleared => {
                    let display_ref = self.display.borrow();
                    let gl_window = display_ref.gl_window();
                    self.platform
                        .borrow()
                        .prepare_frame(self.imgui.borrow_mut().io_mut(), gl_window.window())
                        .expect("Failed to prepare frame");
                    gl_window.window().request_redraw();
                }

                glium::glutin::event::Event::RedrawEventsCleared => {
                    // This is a hack to address the behavior difference between
                    // Windows and Linux in winit rleated to the ControlFlow::Wait
                    // Issue: https://github.com/rust-windowing/winit/issues/1619
                    // TODO: Remove this once the issue is resolved
                    if self.redraw_next_frame {
                        self.redraw_next_frame = false;
                        *control_flow = ControlFlow::Poll;
                    } else {
                        *control_flow = ControlFlow::Wait;
                    }
                }

                glium::glutin::event::Event::RedrawRequested(_) => {
                    let frame_start = std::time::Instant::now();

                    // Borrow the fields as mutable
                    let mut imgui = self.imgui.borrow_mut();
                    let mut display = self.display.borrow_mut();
                    let mut platform = self.platform.borrow_mut();
                    let mut renderer = self.renderer.borrow_mut();

                    on_draw(
                        &mut *imgui,
                        &mut *display,
                        &mut *platform,
                        &mut *renderer,
                        control_flow,
                    );

                    // Probably cleaner to deduce a UI refresh using a
                    // custom event, but for now it works good enough.
                    if control_flow == &ControlFlow::Poll {
                        // TODO: Remove this once the winit issue is closed
                        *control_flow = ControlFlow::Wait;
                        self.redraw_next_frame = true;
                    }

                    let frame_time = frame_start.elapsed();
                    let desired_frame_time = if self.is_window_dragging {
                        desired_frame_time_moving
                    } else if self.is_window_minimized {
                        desired_frame_time_inactive
                    } else {
                        desired_frame_time_active
                    };

                    if frame_time < desired_frame_time {
                        std::thread::sleep(desired_frame_time - frame_time);
                    }

                    // Check if more than 3 seconds have passed since the last cursor move event
                    if self.is_window_dragging
                        && self.last_window_drag.elapsed() > std::time::Duration::from_secs(3)
                    {
                        self.is_window_dragging = false;
                    }
                }

                event => {
                    let display_ref = self.display.borrow();
                    let gl_window = display_ref.gl_window();
                    let mut platform_ref = self.platform.borrow_mut();
                    platform_ref.handle_event(
                        self.imgui.borrow_mut().io_mut(),
                        gl_window.window(),
                        &event,
                    );

                    if let glium::glutin::event::Event::WindowEvent { event, .. } = event {
                        match event {
                            glium::glutin::event::WindowEvent::Moved(_) => {
                                // Update the last cursor move time
                                self.last_window_drag = std::time::Instant::now();
                                self.is_window_dragging = true;
                            }
                            glium::glutin::event::WindowEvent::CursorEntered { .. } => {
                                self.is_mouse_in_window = true;
                            }
                            glium::glutin::event::WindowEvent::CursorLeft { .. } => {
                                self.is_mouse_in_window = false;
                            }
                            glium::glutin::event::WindowEvent::CloseRequested => {
                                *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                            }
                            glium::glutin::event::WindowEvent::Resized(new_size) => {
                                // If the size is 0, the window is minimized
                                if new_size.width == 0 && new_size.height == 0 {
                                    gl_window.window().set_visible(false);
                                    self.is_window_minimized = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        });
    }
}
