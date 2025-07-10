use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};
use sha2::{Digest, Sha256};
use crate::app::Clip;

fn get_cache_dir() -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let cache_dir = dirs::cache_dir()
        .ok_or("Could not find cache directory")?
        .join("avim"); // Renamed from audiovim
    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

fn get_cache_path_for_file(audio_path: &str) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let absolute_path = Path::new(audio_path).canonicalize()?;
    let mut hasher = Sha256::new();
    hasher.update(absolute_path.to_str().unwrap().as_bytes());
    let result = hasher.finalize();
    let hash_hex = hex::encode(result);
    let cache_filename = format!("{}.json", hash_hex);
    Ok(get_cache_dir()?.join(cache_filename))
}

pub async fn load_from_cache(audio_path: &str) -> Option<Vec<Clip>> {
    if let Ok(cache_path) = get_cache_path_for_file(audio_path) {
        if cache_path.exists() {
            if let Ok(file_contents) = fs::read_to_string(cache_path) {
                if let Ok(clips) = serde_json::from_str(&file_contents) {
                    return Some(clips);
                }
            }
        }
    }
    None
}

pub async fn save_to_cache(audio_path: &str, clips: &[Clip]) -> Result<(), Box<dyn Error + Send + Sync>> {
    let cache_path = get_cache_path_for_file(audio_path)?;
    let json_data = serde_json::to_string_pretty(clips)?;
    fs::write(cache_path, json_data)?;
    Ok(())
}

