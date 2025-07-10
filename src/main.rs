use std::{env, error::Error, time::Duration, process::Command, fs};
use tokio::sync::mpsc;
use crossterm::{
    cursor::SetCursorStyle,
    event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

mod app;
mod ui;
mod gcp;
mod sox;
mod cache;
mod vim;
mod autofix;

use crate::app::{App, AppEvent, AppState, Mode};

const CHUNK_DURATION_SECONDS: f64 = 300.0;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut input_path: Option<String> = None;
    let mut use_cache = true;
    let mut debug_mode = false;

    for arg in env::args().skip(1) {
        if arg == "--no-cache" { use_cache = false; }
        else if arg == "--debug" { debug_mode = true; }
        else if input_path.is_none() { input_path = Some(arg); }
    }

    let input_path = match input_path {
        Some(p) => p,
        None => {
            eprintln!("Usage: avim [--no-cache] [--debug] <audio_file.wav | project_file.avim>");
            std::process::exit(1);
        }
    };

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, SetCursorStyle::SteadyBlock)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (tx, mut rx) = mpsc::channel(100);
    
    let (audio_path, project_path) = if input_path.ends_with(".avim") {
        let (audio_path, clips) = App::load_project(&input_path)?;
        let duration_output = Command::new("soxi").arg("-D").arg(&audio_path).output().unwrap();
        let duration = String::from_utf8(duration_output.stdout).unwrap_or_default().trim().parse::<f64>().unwrap_or(0.0);
        tx.send(AppEvent::TranscriptionSuccess(clips, duration)).await.ok();
        (audio_path, Some(input_path.clone()))
    } else {
        tokio::spawn({
            let tx = tx.clone();
            let path = input_path.clone();
            async move {
                if use_cache {
                    if let Some(clips) = cache::load_from_cache(&path).await {
                        let duration_output = Command::new("soxi").arg("-D").arg(&path).output().unwrap();
                        let duration = String::from_utf8(duration_output.stdout).unwrap_or_default().trim().parse::<f64>().unwrap_or(0.0);
                        tx.send(AppEvent::TranscriptionSuccess(clips, duration)).await.ok();
                        return;
                    }
                }
                
                let duration_output = Command::new("soxi").arg("-D").arg(&path).output();
                let total_duration = match duration_output {
                    Ok(output) => String::from_utf8(output.stdout).unwrap_or_default().trim().parse::<f64>().unwrap_or(0.0),
                    Err(_) => {
                        tx.send(AppEvent::TranscriptionFailure("Failed to execute soxi.".to_string())).await.ok();
                        return;
                    }
                };
                
                if total_duration == 0.0 {
                    tx.send(AppEvent::TranscriptionFailure("Could not get audio duration.".to_string())).await.ok();
                    return;
                }

                let num_chunks = (total_duration / CHUNK_DURATION_SECONDS).ceil() as i32;
                let mut all_clips = Vec::new();

                for i in 0..num_chunks {
                    tx.send(AppEvent::StatusUpdate(format!("Transcribing chunk {} of {}...", i + 1, num_chunks))).await.ok();
                    let chunk_start_time = i as f64 * CHUNK_DURATION_SECONDS;
                    let chunk_path = env::temp_dir().join(format!("avim_chunk_{}.wav", i));

                    let mut trim_cmd = Command::new("sox");
                    trim_cmd.arg(&path).arg(chunk_path.to_str().unwrap()).arg("trim").arg(chunk_start_time.to_string());
                    
                    if chunk_start_time + CHUNK_DURATION_SECONDS < total_duration {
                        trim_cmd.arg(CHUNK_DURATION_SECONDS.to_string());
                    }

                    if !trim_cmd.status().unwrap().success() {
                        tx.send(AppEvent::TranscriptionFailure(format!("Failed to create chunk {}", i))).await.ok();
                        return;
                    }

                    let chunk_data = fs::read(&chunk_path).unwrap();
                    fs::remove_file(chunk_path).ok();

                    match gcp::transcribe_chunk(&chunk_data).await {
                        Ok(mut chunk_clips) => {
                            for clip in &mut chunk_clips {
                                clip.start_time += chunk_start_time;
                                clip.end_time += chunk_start_time;
                            }
                            all_clips.extend(chunk_clips);
                        }
                        Err(e) => {
                            tx.send(AppEvent::TranscriptionFailure(e.to_string())).await.ok();
                            return;
                        }
                    }
                }

                let sanitized_clips: Vec<_> = all_clips.into_iter()
                    .filter(|clip| clip.start_time < total_duration)
                    .map(|mut clip| {
                        if clip.end_time > total_duration { clip.end_time = total_duration; }
                        clip
                    })
                    .collect();

                cache::save_to_cache(&path, &sanitized_clips).await.ok();
                tx.send(AppEvent::TranscriptionSuccess(sanitized_clips, total_duration)).await.ok();
            }
        });
        (input_path, None)
    };

    let mut app = App::new(audio_path, project_path, debug_mode);

    tokio::spawn({
        let tx = tx.clone();
        async move {
            loop {
                if event::poll(Duration::from_millis(250)).unwrap_or(false) {
                    if let Ok(CEvent::Key(key)) = event::read() {
                        if tx.send(AppEvent::Input(key)).await.is_err() { break; }
                    }
                }
            }
        }
    });

    let mut last_key: Option<KeyEvent> = None;
    loop {
        terminal.draw(|f| ui::ui(f, &mut app))?;

        match rx.recv().await {
            Some(AppEvent::Input(key)) => {
                if key.code == KeyCode::Char('c') && key.modifiers == KeyModifiers::CONTROL {
                    app.should_quit = true;
                }

                match app.state {
                    AppState::Loading(_) => {
                        if key.code == KeyCode::Char('q') { app.should_quit = true; }
                    }
                    AppState::Ready => {
                        match app.mode {
                            Mode::Normal => {
                                if let Some(last) = last_key {
                                    if last.code == KeyCode::Char('d') && key.code == KeyCode::Char('d') { vim::delete_clip(&mut app); }
                                    else if last.code == KeyCode::Char('y') && key.code == KeyCode::Char('y') { vim::yank_clip(&mut app); }
                                    last_key = None;
                                } else {
                                    match key.code {
                                        KeyCode::Char(':') => app.mode = Mode::Command,
                                        KeyCode::Char('q') => app.should_quit = true,
                                        KeyCode::Char('j') => vim::next_clip(&mut app),
                                        KeyCode::Char('k') => vim::previous_clip(&mut app),
                                        KeyCode::Char('p') => vim::paste_clip(&mut app),
                                        KeyCode::Char('u') => vim::undo(&mut app),
                                        KeyCode::Char('r') if key.modifiers == KeyModifiers::CONTROL => vim::redo(&mut app),
                                        KeyCode::Char('[') => vim::adjust_start_time(&mut app, false),
                                        KeyCode::Char(']') => vim::adjust_start_time(&mut app, true),
                                        KeyCode::Char('{') => vim::adjust_end_time(&mut app, false),
                                        KeyCode::Char('}') => vim::adjust_end_time(&mut app, true),
                                        KeyCode::Char('i') => app.mode = Mode::Insert,
                                        KeyCode::Char('m') => autofix::enter_adjust_mode(&mut app),
                                        KeyCode::Char('P') => {
                                            if app.playback_pid.is_some() {
                                                sox::stop_playback(app.playback_pid.unwrap());
                                                app.playback_pid = None;
                                                app.status_message = "Playback stopped.".to_string();
                                            } else {
                                                app.status_message = "Playing all from current clip...".to_string();
                                                match sox::play_playlist(&app, app.current_clip_index) {
                                                    Ok(child) => app.playback_pid = Some(child.id()),
                                                    Err(e) => app.status_message = format!("Playback failed: {}", e),
                                                }
                                            }
                                        },
                                        KeyCode::Char(' ') => {
                                            if let Some(pid) = app.playback_pid {
                                                sox::stop_playback(pid);
                                                app.playback_pid = None;
                                                app.status_message = "Playback stopped.".to_string();
                                            } else if let Some(clip) = app.clips.get(app.current_clip_index).cloned() {
                                                let path = app.original_audio_path.clone();
                                                app.status_message = format!("Playing clip {}...", app.current_clip_index + 1);
                                                match sox::play_clip_with_sox(&path, clip.start_time, clip.end_time) {
                                                    Ok(child) => app.playback_pid = Some(child.id()),
                                                    Err(e) => app.status_message = format!("Playback failed: {}", e),
                                                }
                                            }
                                        }
                                        KeyCode::Char('d') | KeyCode::Char('y') => last_key = Some(key),
                                        _ => {}
                                    }
                                }
                            },
                            Mode::Insert => {
                                match key.code {
                                    KeyCode::Esc => app.mode = Mode::Normal,
                                    KeyCode::Char(c) => vim::append_to_comment(&mut app, c),
                                    KeyCode::Backspace => vim::pop_from_comment(&mut app),
                                    _ => {}
                                }
                            }
                            Mode::Command => {
                                match key.code {
                                    KeyCode::Enter => app.process_command(),
                                    KeyCode::Char(c) => app.command_input.push(c),
                                    KeyCode::Backspace => { app.command_input.pop(); },
                                    KeyCode::Esc => { app.mode = Mode::Normal; app.command_input.clear(); }
                                    _ => {}
                                }
                            },
                            Mode::Adjust => {
                                match key.code {
                                    KeyCode::Esc => app.mode = Mode::Normal,
                                    KeyCode::Char('w') => autofix::adjust_next_word(&mut app),
                                    KeyCode::Char('b') => autofix::adjust_previous_word(&mut app),
                                    KeyCode::Enter => autofix::confirm_adjustment(&mut app),
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
            },
            Some(AppEvent::TranscriptionSuccess(clips, duration)) => app.load_clips(clips, duration),
            Some(AppEvent::TranscriptionFailure(err_msg)) => app.set_error_state(err_msg),
            Some(AppEvent::StatusUpdate(msg)) => app.status_message = msg,
            None => break,
        }

        if app.should_quit { break; }
    }

    if let Some(pid) = app.playback_pid {
        sox::stop_playback(pid);
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;
    Ok(())
}

