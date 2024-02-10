use directories::ProjectDirs;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::ErrorKind;
use std::sync::Mutex;

static CONFIG: Lazy<Mutex<Config>> = Lazy::new(|| Mutex::new(Config::new()));

#[derive(Serialize, Deserialize)]
pub struct Config {
    trim_clips: bool,
    save_history: bool,
    #[serde(skip_serializing)]
    config_file_path: String,
}

impl Config {
    fn new() -> Self {
        // TODO: Change this, it's bad :(
        let config_dir = ProjectDirs::from("", "", "clipstash")
            .unwrap()
            .config_dir()
            .to_path_buf();

        // Create the directory if it doesn't exist
        fs::create_dir_all(&config_dir).expect("Failed to create config directory");

        let file_path = config_dir
            .join("preferences.json")
            .to_str()
            .unwrap()
            .to_string();

        match fs::read(file_path.clone()) {
            Ok(data) => {
                match serde_json::from_slice::<Config>(&data) {
                    Ok(config) => {
                        let trim_clips = config.trim_clips;
                        let save_history = config.save_history;

                        Self {
                            trim_clips,
                            save_history,
                            config_file_path: file_path,
                        }
                    }
                    Err(_) => {
                        // If the file exists but is not valid JSON,
                        Self {
                            trim_clips: true,
                            save_history: true,
                            config_file_path: file_path,
                        }
                    }
                }
            }
            Err(ref e) if e.kind() == ErrorKind::NotFound => {
                // If the file doesn't exist,
                Self {
                    trim_clips: true,
                    save_history: true,
                    config_file_path: file_path,
                }
            }
            Err(_) => {
                // If there's another kind of error,
                Self {
                    trim_clips: false,
                    save_history: false,
                    config_file_path: file_path,
                }
            }
        }
    }

    pub fn get_instance() -> std::sync::MutexGuard<'static, Config> {
        CONFIG.lock().unwrap()
    }

    pub fn get_save_history(&self) -> bool {
        self.save_history
    }

    pub fn set_save_history(&mut self, save_history: bool) {
        self.save_history = save_history;
        self.save()
            .expect("[set_save_history] Failed to save config to file");
    }

    pub fn get_trim_clips(&self) -> bool {
        self.trim_clips
    }

    pub fn set_trim_clips(&mut self, trim_clips: bool) {
        self.trim_clips = trim_clips;
        self.save()
            .expect("[set_trim_clips] Failed to save config to file");
    }

    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let data = serde_json::to_vec(self)?;
        fs::write(&self.config_file_path, data)?;
        Ok(())
    }
}
