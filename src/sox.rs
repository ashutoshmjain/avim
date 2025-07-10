use std::{env, fs, process::{Command, Stdio, Child}};
use crate::app::App;

pub fn play_clip_with_sox(path: &str, start: f64, end: f64) -> Result<Child, String> {
    let duration = end - start;
    Command::new("sox")
        .arg(path)
        .arg("-d") // Output to default audio device
        .arg("trim")
        .arg(start.to_string())
        .arg(duration.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())
}

pub fn play_playlist(app: &App, start_index: usize) -> Result<Child, String> {
    let temp_dir = env::temp_dir().join("avim_playlist");
    // Clean up old playlist files before creating new ones
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;

    let mut temp_files = Vec::new();
    for (i, clip) in app.clips.iter().skip(start_index).enumerate() {
        let temp_filename = temp_dir.join(format!("playlist_clip_{}.wav", i));
        let duration = clip.end_time - clip.start_time;

        let status = Command::new("sox")
            .arg(&app.original_audio_path)
            .arg(temp_filename.to_str().unwrap())
            .arg("trim")
            .arg(clip.start_time.to_string())
            .arg(duration.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err(format!("SoX failed to trim clip for playlist at index {}", i));
        }
        temp_files.push(temp_filename);
    }

    if temp_files.is_empty() {
        return Err("No clips to play.".to_string());
    }

    let concatenated_playlist_path = temp_dir.join("playlist.wav");
    if temp_files.len() > 1 {
        let mut concat_cmd = Command::new("sox");
        for temp_file in &temp_files {
            concat_cmd.arg(temp_file.to_str().unwrap());
        }
        concat_cmd.arg(concatenated_playlist_path.to_str().unwrap());
        
        let concat_status = concat_cmd.status().map_err(|e| e.to_string())?;
        if !concat_status.success() {
            return Err("Failed to create temporary playlist file.".to_string());
        }
    } else {
        fs::copy(&temp_files[0], &concatenated_playlist_path).map_err(|e| e.to_string())?;
    }

    Command::new("sox")
        .arg(concatenated_playlist_path.to_str().unwrap())
        .arg("-d")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())
}


pub fn stop_playback(pid: u32) {
    Command::new("kill")
        .arg("-9") 
        .arg(pid.to_string())
        .output()
        .ok();
}

pub fn export_audio(app: &App, output_filename: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let temp_dir = env::temp_dir().join("avim");
    fs::create_dir_all(&temp_dir)?;

    if app.clips.is_empty() {
        Command::new("sox")
            .arg("-n")
            .arg("-r").arg("44100")
            .arg("-c").arg("2")
            .arg(output_filename)
            .arg("trim").arg("0").arg("0")
            .status()?;
        return Ok(());
    }

    let mut temp_files = Vec::new();
    for (i, clip) in app.clips.iter().enumerate() {
        let temp_filename = temp_dir.join(format!("clip_{}.wav", i));
        let duration = clip.end_time - clip.start_time;

        let status = Command::new("sox")
            .arg(&app.original_audio_path)
            .arg(temp_filename.to_str().unwrap())
            .arg("trim")
            .arg(clip.start_time.to_string())
            .arg(duration.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        if !status.success() {
            fs::remove_dir_all(&temp_dir)?;
            return Err(format!("SoX failed to trim clip #{}", i).into());
        }
        temp_files.push(temp_filename);
    }

    if temp_files.len() == 1 {
        fs::copy(&temp_files[0], output_filename)?;
    } else {
        let mut sox_concat_cmd = Command::new("sox");
        sox_concat_cmd.arg("--combine").arg("concatenate");
        for temp_file in &temp_files {
            sox_concat_cmd.arg(temp_file.to_str().unwrap());
        }
        sox_concat_cmd.arg(output_filename);

        let final_status = sox_concat_cmd.status()?;
        
        if !final_status.success() {
            fs::remove_dir_all(&temp_dir)?;
            return Err("SoX failed to concatenate final audio.".into());
        }
    }

    fs::remove_dir_all(&temp_dir)?;
    Ok(())
}

