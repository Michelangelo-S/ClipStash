#![windows_subsystem = "windows"]

mod history;
mod preferences;
mod ui;
mod window;

use arboard::Clipboard;
use auto_launch::AutoLaunchBuilder;
use std::rc::Rc;
use std::{cell::RefCell, env};
use window::{ImguiWindow, Skin};

use crate::ui::UI;

fn main() {
    std::thread::spawn(|| {
        let mut clipboard = Clipboard::new().unwrap();
        let mut last_element = history::ClipboardHistory::get_instance()
            .get_items()
            .last()
            .unwrap_or(&"".to_string())
            .clone();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(100));

            let clipboard_contents = match clipboard.get_text() {
                Ok(contents) => contents,
                Err(_) => {
                    std::thread::sleep(std::time::Duration::from_millis(200));
                    continue;
                }
            };
            let trimmed_contents = clipboard_contents.trim();

            if trimmed_contents.is_empty() {
                continue;
            }

            if trimmed_contents == last_element {
                continue;
            }

            // Acquire a lock to the clipboard history & config only when we need to,
            // and release it immediately, before the thread sleeps.
            let mut clip_history = history::ClipboardHistory::get_instance();
            let config = preferences::Config::get_instance();

            clip_history.add_item(trimmed_contents.to_string());
            last_element = trimmed_contents.to_string();

            if config.get_save_history() {
                clip_history
                    .save_to_file()
                    .expect("Failed to save history to file");
            }
        }
    });

    let current_exe_path = env::current_exe().unwrap();
    let autostarter = AutoLaunchBuilder::new()
        .set_app_name("clipboard-manager")
        .set_app_path(current_exe_path.to_str().unwrap())
        .build()
        .unwrap();

    let ui = Rc::new(RefCell::new(UI::new(autostarter)));
    let mut window = ImguiWindow::new();

    // RGB values for pink color normalized
    let title_color = [0.0, 0.035, 0.0, 1.0];
    let pink_rgba = [0.949, 0.31, 0.82, 0.8];
    let deep_pink = [0.569, 0.035, 0.486, 1.0];

    let default_skin = Skin {
        title_bg: title_color,
        title_bg_collapsed: title_color,
        title_bg_active: title_color,
        button: pink_rgba,
        button_hovered: deep_pink,
        button_active: pink_rgba,
        header: pink_rgba,
        header_hovered: pink_rgba,
        header_active: deep_pink,
        frame_rounding: 1.0,
    };
    window.set_skin(default_skin);

    window.render(move |imgui, display, platform, renderer, control_flow| {
        ui.borrow_mut()
            .on_draw(imgui, display, platform, renderer, control_flow);
    });
}
