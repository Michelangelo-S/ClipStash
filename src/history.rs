use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;

static CLIPBOARD_HISTORY: Lazy<Mutex<ClipboardHistory>> =
    Lazy::new(|| Mutex::new(ClipboardHistory::new()));

#[derive(Serialize, Deserialize)]
pub struct ClipboardHistory {
    list_items: Vec<String>,
    history_file_path: String,
}

impl ClipboardHistory {
    fn new() -> Self {
        // TODO: Change this, it's bad :(
        let history_dir = ProjectDirs::from("", "", "clipstash")
            .unwrap()
            .data_dir()
            .to_path_buf();

        // Create the directory if it doesn't exist
        fs::create_dir_all(&history_dir).expect("Failed to create config directory");

        let file_path = history_dir
            .join("history.json")
            .to_str()
            .unwrap()
            .to_string();

        if Path::new(&file_path).exists() {
            let mut file = File::open(file_path).unwrap();
            let mut json = String::new();
            file.read_to_string(&mut json).unwrap();
            let data: ClipboardHistory = serde_json::from_str(&json).unwrap();
            return data;
        }

        Self {
            list_items: Vec::new(),
            history_file_path: file_path,
        }
    }

    pub fn get_instance() -> std::sync::MutexGuard<'static, ClipboardHistory> {
        CLIPBOARD_HISTORY.lock().unwrap()
    }

    pub fn add_item(&mut self, new_item: String) {
        self.list_items.push(new_item);
    }

    pub fn remove_item(&mut self, index: usize) {
        self.list_items.remove(index);
    }

    pub fn clear_items(&mut self) {
        self.list_items.clear();
    }

    pub fn get_items(&self) -> Vec<String> {
        self.list_items.clone()
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let json = serde_json::to_string(&self).unwrap();
        let mut file = File::create(&self.history_file_path)?;
        let result = file.write_all(json.as_bytes())?;
        Ok(result)
    }

    pub fn delete_file(&self) -> std::io::Result<()> {
        // If file doesn't exist, return early
        if !Path::new(&self.history_file_path).exists() {
            return Ok(());
        }
        std::fs::remove_file(&self.history_file_path)
    }
}
