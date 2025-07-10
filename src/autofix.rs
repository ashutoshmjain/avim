use crate::app::App;

pub fn enter_adjust_mode(app: &mut App) {
    if app.current_clip_index < app.clips.len() - 1 {
        app.mode = crate::app::Mode::Adjust;
        app.adjust_word_index = 0;
        app.status_message = "ADJUST MODE: Use 'w'/'b' to select word, 'Enter' to confirm, 'Esc' to cancel.".to_string();
    } else {
        app.status_message = "Cannot adjust the last clip.".to_string();
    }
}

pub fn adjust_next_word(app: &mut App) {
    if let Some(next_clip) = app.clips.get(app.current_clip_index + 1) {
        let words: Vec<&str> = next_clip.transcript.split_whitespace().collect();
        if app.adjust_word_index < words.len().saturating_sub(1) {
            app.adjust_word_index += 1;
        }
    }
}

pub fn adjust_previous_word(app: &mut App) {
    if app.adjust_word_index > 0 {
        app.adjust_word_index -= 1;
    }
}

pub fn confirm_adjustment(app: &mut App) {
    if app.current_clip_index >= app.clips.len() - 1 {
        app.mode = crate::app::Mode::Normal;
        return;
    }
    
    app.save_undo_state();

    let next_clip_transcript = app.clips[app.current_clip_index + 1].transcript.clone();
    let words: Vec<&str> = next_clip_transcript.split_whitespace().collect();

    if app.adjust_word_index < words.len() {
        let words_to_move = app.adjust_word_index + 1;
        let text_to_append = words.iter().take(words_to_move).map(|&s| s).collect::<Vec<&str>>().join(" ");
        
        app.adjustments.push(words_to_move);
        app.log_debug(format!("Adjustment {}: Moved {} words.", app.adjustments.len(), words_to_move));

        let remaining_text = words.iter().skip(words_to_move).map(|&s| s).collect::<Vec<&str>>().join(" ");
        
        app.clips[app.current_clip_index].transcript.push_str(" ");
        app.clips[app.current_clip_index].transcript.push_str(&text_to_append);
        app.clips[app.current_clip_index + 1].transcript = remaining_text;
        
        // Mark both clips as manually adjusted
        app.clips[app.current_clip_index].is_manually_adjusted = true;
        app.clips[app.current_clip_index + 1].is_manually_adjusted = true;
    }

    app.mode = crate::app::Mode::Normal;
    
    if app.adjustments.len() < 3 {
         app.status_message = format!("Adjustment learned. Adjust {} more to find a pattern.", 3 - app.adjustments.len());
    } else {
         app.status_message = "Pattern learned. You can now try :autofix".to_string();
    }
}

pub fn autofix_transcripts(app: &mut App) {
    if app.adjustments.is_empty() {
        app.status_message = "Not enough data to autofix. Please adjust a few clips first.".to_string();
        return;
    }

    let sum: usize = app.adjustments.iter().sum();
    let mean = sum as f64 / app.adjustments.len() as f64;
    let std_dev = {
        let variance = app.adjustments.iter().map(|value| {
            let diff = mean - (*value as f64);
            diff * diff
        }).sum::<f64>() / app.adjustments.len() as f64;
        variance.sqrt()
    };

    app.log_debug(format!("Autofix: Mean words moved: {:.2}, Std Dev: {:.2}", mean, std_dev));

    if std_dev > 1.0 { // Confidence threshold
        app.status_message = format!("Pattern is not consistent enough (Std Dev: {:.2}). Please adjust more clips.", std_dev);
        return;
    }

    app.save_undo_state();
    let words_to_move_avg = mean.round() as usize;
    app.log_debug(format!("Applying autofix, moving avg {} words.", words_to_move_avg));
    let mut total_moved = 0;

    for i in (0..app.clips.len() - 1).rev() {
        // Check if the clip has been manually adjusted
        if app.clips[i].is_manually_adjusted || app.clips[i + 1].is_manually_adjusted {
            continue;
        }

        let next_clip_transcript = app.clips[i + 1].transcript.clone();
        let next_clip_words: Vec<&str> = next_clip_transcript.split_whitespace().collect();
        
        if next_clip_words.len() > words_to_move_avg {
            let text_to_append = next_clip_words.iter().take(words_to_move_avg).map(|&s| s).collect::<Vec<_>>().join(" ");
            let remaining_text = next_clip_words.iter().skip(words_to_move_avg).map(|&s| s).collect::<Vec<_>>().join(" ");

            app.clips[i].transcript.push_str(" ");
            app.clips[i].transcript.push_str(&text_to_append);
            app.clips[i + 1].transcript = remaining_text;
            total_moved += words_to_move_avg;
        }
    }
    
    app.clips.retain(|clip| !clip.transcript.trim().is_empty());
    app.status_message = format!("Autofix complete. Moved approx {} words.", total_moved);
}

