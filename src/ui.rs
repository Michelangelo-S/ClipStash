use arboard::Clipboard;
use glium::glutin::event_loop::ControlFlow;
use glium::Surface;
use imgui::{Condition, Ui};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::WinitPlatform;

use crate::history::ClipboardHistory;
use crate::preferences::Config;

pub struct UI {
    show_about_dialog: bool,
    auto_launch: auto_launch::AutoLaunch,
}

impl UI {
    pub fn new(autostart: auto_launch::AutoLaunch) -> Self {
        Self {
            show_about_dialog: false,
            auto_launch: autostart,
        }
    }

    pub fn on_draw(
        &mut self,
        imgui: &mut imgui::Context,
        display: &mut glium::Display,
        platform: &mut WinitPlatform,
        renderer: &mut Renderer,
        control_flow: &mut glium::glutin::event_loop::ControlFlow,
    ) {
        let ui: Ui<'_> = imgui.frame();

        // Get the size of the application window
        let gl_window = display.gl_window();
        let size = gl_window.window().inner_size();

        let mut open: bool = true;

        // Write code that creates a new window with a text saying "hello world" and a button that says "click me" and prints "clicked" to the console when clicked.
        let window = imgui::Window::new(" ");
        window
            .size([size.width as f32, size.height as f32], Condition::Always)
            .position([0.0, 0.0], Condition::Always)
            .resizable(false)
            .collapsible(false)
            .menu_bar(true)
            .scrollable(true)
            .opened(&mut open)
            .no_decoration()
            .build(&ui, || {
                let mut clip_history = ClipboardHistory::get_instance();
                let mut config = Config::get_instance();

                if let Some(menu_bar_token) = ui.begin_menu_bar() {
                    if let Some(menu_token) = ui.begin_menu("File") {
                        let menu_item = imgui::MenuItem::new("Start on Startup")
                            .selected(self.auto_launch.is_enabled().unwrap());
                        if menu_item.build(&ui) {
                            if self.auto_launch.is_enabled().unwrap() {
                                assert_eq!(self.auto_launch.disable().is_ok(), true);
                            } else {
                                assert_eq!(self.auto_launch.enable().is_ok(), true);
                            }
                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }
                        if imgui::MenuItem::new("Exit").build(&ui) {
                            // Exit application
                            *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
                        }

                        menu_token.end();
                    }

                    if let Some(menu_token) = ui.begin_menu("Edit") {
                        let trim_clips_menu_item =
                            imgui::MenuItem::new("Trim Clips").selected(config.get_trim_clips());
                        if trim_clips_menu_item.build(&ui) {
                            // Toggle trim clips
                            let new_trim_clips = !config.get_trim_clips();
                            config.set_trim_clips(new_trim_clips);
                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }

                        let save_history_menu_item = imgui::MenuItem::new("Save History")
                            .selected(config.get_save_history());
                        if save_history_menu_item.build(&ui) {
                            // If we currently save history, clear the history file
                            if config.get_save_history() {
                                // Clear history
                                clip_history
                                    .delete_file()
                                    .expect("Failed to delete history file");
                            }

                            // Toggle save history
                            let new_save_history = !config.get_save_history();
                            config.set_save_history(new_save_history);

                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }

                        if imgui::MenuItem::new("Clear History").build(&ui) {
                            // Clear history
                            clip_history.clear_items();
                            let _ = clip_history.delete_file();
                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }

                        menu_token.end();
                    }

                    if let Some(menu_token) = ui.begin_menu("Help") {
                        if imgui::MenuItem::new("About").build(&ui) {
                            self.show_about_dialog = true;
                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }

                        menu_token.end();
                    }

                    menu_bar_token.end();
                }

                let mut selected_item: Option<usize> = None;

                let clip_history_items = clip_history.get_items();
                for (i, item) in clip_history_items.iter().enumerate() {
                    let selected = Some(i) == selected_item;
                    let display_item = if item.len() > 100 {
                        if config.get_trim_clips() {
                            format!("{}...", item.chars().take(100).collect::<String>())
                        } else {
                            item.clone()
                        }
                    } else {
                        item.clone()
                    };
                    if imgui::Selectable::new(&display_item)
                        .selected(selected)
                        .build(&ui)
                    {
                        selected_item = Some(i);
                    }

                    let mut clipboard = Clipboard::new().unwrap();

                    // Right-click context menu
                    if ui.is_item_hovered() && ui.is_mouse_clicked(imgui::MouseButton::Right) {
                        ui.open_popup(&format!("item_context_{}", i));
                    }

                    // Show right click context menu & copy on Click
                    ui.popup(&format!("item_context_{}", i), || {
                        if imgui::MenuItem::new(&"Copy").build(&ui) {
                            clipboard.set_text(item).unwrap();

                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }

                        if imgui::MenuItem::new(&"Remove").build(&ui) {
                            clip_history.remove_item(i);
                            if config.get_save_history() {
                                clip_history
                                    .save_to_file()
                                    .expect("[Remove Item] Failed to save history to file");
                            }
                            // Refresh the UI
                            *control_flow = ControlFlow::Poll;
                        }
                    });

                    // Copy on double click
                    if ui.is_item_hovered() && ui.is_mouse_double_clicked(imgui::MouseButton::Left)
                    {
                        let _ = clipboard.set_text(item.clone());
                    }
                }
            });

        if self.show_about_dialog {
            ui.open_popup("About");
            imgui::PopupModal::new(&"About")
                .resizable(false)
                .build(&ui, || {
                    ui.text("Clipboard Manager");
                    ui.spacing();
                    ui.separator();
                    ui.spacing();
                    ui.text("Version: 0.1.0");
                    ui.text("Author: Michelangelo Sarafis");
                    ui.text("A Clipboard Manager written in Rust with ImGui and glium.");
                    ui.spacing();
                    ui.separator();
                    ui.spacing();
                    if ui.button_with_size("OK", [30.0, 25.0]) {
                        self.show_about_dialog = false;
                        ui.close_current_popup();
                    }
                });
            // This causes the UI to refresh for as long as
            // the popup is open, which is not ideal.
            *control_flow = ControlFlow::Poll;
        }

        if !open {
            *control_flow = glium::glutin::event_loop::ControlFlow::Exit;
        }

        let gl_window = display.gl_window();
        let mut target = display.draw();
        target.clear_color_srgb(1.0, 1.0, 1.0, 1.0);
        platform.prepare_render(&ui, gl_window.window());
        let draw_data = ui.render();
        renderer
            .render(&mut target, draw_data)
            .expect("UI rendering failed");
        target.finish().expect("Failed to swap buffers");
    }
}
