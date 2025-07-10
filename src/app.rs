use crate::sox;
use serde::{Deserialize, Serialize};
use arboard::Clipboard;
use std::fs;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Mode {
    Normal,
    Command,
    Insert,
    Adjust,
    Visual,
}

pub enum AppEvent {
    Input(crossterm::event::KeyEvent),
    TranscriptionSuccess(Vec<Clip>, f64),
    TranscriptionFailure(String),
    StatusUpdate(String),
}

pub enum AppState {
    Loading(String),
    Ready,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Clip {
    pub id: usize,
    pub speaker: String,
    pub transcript: String,
    pub start_time: f64,
    pub end_time: f64,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub is_manually_adjusted: bool,
}

pub struct App {
    pub state: AppState,
    pub original_audio_path: String,
    pub project_path: Option<String>,
    pub clips: Vec<Clip>,
    pub current_clip_index: usize,
    pub mode: Mode,
    pub command_input: String,
    pub should_quit: bool,
    pub register: Option<Clip>,
    pub status_message: String,
    pub undo_stack: Vec<Vec<Clip>>,
    pub redo_stack: Vec<Vec<Clip>>,
    pub last_error: Option<String>,
    pub playback_pid: Option<u32>,
    pub adjust_word_index: usize,
    pub debug_mode: bool,
    pub debug_log: Vec<String>,
    pub total_time_discrepancy: f64,
    pub adjustments: Vec<usize>,
}

impl App {
    pub fn new(audio_path: String, project_path: Option<String>, debug_mode: bool) -> App {
        App {
            state: AppState::Loading("Checking cache or transcribing...".to_string()),
            original_audio_path: audio_path,
            project_path,
            clips: vec![],
            current_clip_index: 0,
            mode: Mode::Normal,
            command_input: String::new(),
            should_quit: false,
            register: None,
            status_message: "Welcome to avim!".to_string(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            last_error: None,
            playback_pid: None,
            adjust_word_index: 0,
            debug_mode,
            debug_log: Vec::new(),
            total_time_discrepancy: 0.0,
            adjustments: Vec::new(),
        }
    }

    pub fn log_debug(&mut self, message: String) {
        if self.debug_mode {
            self.debug_log.push(message);
        }
    }

    pub fn load_project(path: &str) -> Result<(String, Vec<Clip>), Box<dyn std::error::Error + Send + Sync>> {
        let file_contents = fs::read_to_string(path)?;
        let (audio_path, clips): (String, Vec<Clip>) = serde_json::from_str(&file_contents)?;
        Ok((audio_path, clips))
    }

    pub fn save_undo_state(&mut self) {
        self.undo_stack.push(self.clips.clone());
        self.redo_stack.clear();
    }

    pub fn load_clips(&mut self, clips: Vec<Clip>, total_duration: f64) {
        let transcription_duration = clips.last().map_or(0.0, |c| c.end_time);
        self.total_time_discrepancy = transcription_duration - total_duration;
        self.log_debug(format!("Total Audio Duration: {:.2}s", total_duration));
        self.log_debug(format!("Total Transcript Duration: {:.2}s", transcription_duration));
        self.log_debug(format!("Discrepancy: {:.2}s", self.total_time_discrepancy));
        self.clips = clips;
        self.state = AppState::Ready;
        
        if self.total_time_discrepancy > 1.0 {
            self.status_message = format!("Loaded {} clips. Warning: Tx is {:.2}s longer than audio.", self.clips.len(), self.total_time_discrepancy);
        } else {
            self.status_message = format!("Loaded {} clips.", self.clips.len());
        }
    }
    
    pub fn set_error_state(&mut self, message: String) {
        self.last_error = Some(message.clone());
        self.state = AppState::Loading(format!("ERROR: {}. Press 'q' or Ctrl+C to quit.", message));
    }
    
    pub fn process_command(&mut self) {
        let parts: Vec<&str> = self.command_input.split_whitespace().collect();
        if let Some(command) = parts.get(0) {
            match *command {
                "w" => self.save_project(parts.get(1).map(|s| s.to_string())),
                "export" => {
                    if let Some(filename) = parts.get(1) {
                        self.status_message = format!("Exporting to {}...", filename);
                        match sox::export_audio(self, filename) {
                            Ok(_) => self.status_message = format!("Successfully exported to {}.", filename),
                            Err(e) => {
                                let err_msg = e.to_string();
                                self.status_message = format!("Export failed: {}", err_msg);
                                self.last_error = Some(err_msg);
                            }
                        }
                    } else {
                        self.status_message = "Export error: No filename provided.".to_string();
                    }
                }
                "q" | "q!" => self.should_quit = true,
                "help" => {
                    self.status_message = "Commands: :w, :export, :q, :autofix, :lasterror, :help".to_string();
                }
                "lasterror" => {
                    if let Some(err) = &self.last_error {
                        if let Ok(mut clipboard) = Clipboard::new() {
                            if clipboard.set_text(err.clone()).is_ok() {
                                self.status_message = "Last error copied to clipboard.".to_string();
                            } else {
                                self.status_message = "Failed to copy error to clipboard.".to_string();
                            }
                        } else {
                            self.status_message = "Failed to initialize clipboard.".to_string();
                        }
                    } else {
                        self.status_message = "No last error to copy.".to_string();
                    }
                }
                "autofix" => crate::autofix::autofix_transcripts(self),
                _ => self.status_message = format!("Unknown command: {}", self.command_input),
            }
        }
        self.command_input.clear();
        self.mode = Mode::Normal;
    }

    fn save_project(&mut self, new_path: Option<String>) {
        let path_to_save = match new_path {
            Some(p) => {
                self.project_path = Some(p.clone());
                Some(p)
            }
            None => self.project_path.clone(),
        };

        if let Some(path) = path_to_save {
            let data_to_save = (self.original_audio_path.clone(), self.clips.clone());
            match serde_json::to_string_pretty(&data_to_save) {
                Ok(json_data) => {
                    match fs::write(&path, json_data) {
                        Ok(_) => self.status_message = format!("Project saved to {}", path),
                        Err(e) => self.status_message = format!("Failed to save project: {}", e),
                    }
                }
                Err(e) => self.status_message = format!("Failed to serialize project data: {}", e),
            }
        } else {
            self.status_message = "No project file specified. Use :w <filename.avim>".to_string();
        }
    }
}

