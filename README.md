use crate::app::App;

const ADJUSTMENT_AMOUNT: f64 = 0.05; // 50ms

pub fn delete_clip(app: &mut App) {
    if !app.clips.is_empty() {
        app.save_undo_state();
        let deleted_clip = app.clips.remove(app.current_clip_index);
        app.register = Some(deleted_clip);
        if app.current_clip_index >= app.clips.len() && !app.clips.is_empty() {
            app.current_clip_index = app.clips.len() - 1;
        }
        app.status_message = "1 clip deleted.".to_string();
    }
}

pub fn yank_clip(app: &mut App) {
    if let Some(clip) = app.clips.get(app.current_clip_index) {
        app.register = Some(clip.clone());
        app.status_message = "1 clip yanked.".to_string();
    }
}

pub fn paste_clip(app: &mut App) {
    if let Some(clip_to_paste) = app.register.clone() {
        app.save_undo_state();
        let paste_index = if app.clips.is_empty() { 0 } else { app.current_clip_index + 1 };
        app.clips.insert(paste_index, clip_to_paste);
        app.clips[paste_index].is_manually_adjusted = true; // Pasted clips are considered manual
        app.current_clip_index = paste_index;
        app.status_message = "1 clip pasted.".to_string();
    }
}

pub fn next_clip(app: &mut App) {
    if !app.clips.is_empty() && app.current_clip_index < app.clips.len() - 1 {
        app.current_clip_index += 1;
    }
}

pub fn previous_clip(app: &mut App) {
    if app.current_clip_index > 0 {
        app.current_clip_index -= 1;
    }
}

pub fn undo(app: &mut App) {
    if let Some(previous_state) = app.undo_stack.pop() {
        let current_state = app.clips.clone();
        app.redo_stack.push(current_state);
        app.clips = previous_state;
        app.status_message = "Undo.".to_string();
        if app.current_clip_index >= app.clips.len() {
            app.current_clip_index = app.clips.len().saturating_sub(1);
        }
    } else {
        app.status_message = "Already at oldest change.".to_string();
    }
}

pub fn redo(app: &mut App) {
    if let Some(next_state) = app.redo_stack.pop() {
        let current_state = app.clips.clone();
        app.undo_stack.push(current_state);
        app.clips = next_state;
        app.status_message = "Redo.".to_string();
        if app.current_clip_index >= app.clips.len() {
            app.current_clip_index = app.clips.len().saturating_sub(1);
        }
    } else {
        app.status_message = "Already at newest change.".to_string();
    }
}


pub fn adjust_start_time(app: &mut App, increase: bool) {
    app.save_undo_state();
    let adjustment = if increase { ADJUSTMENT_AMOUNT } else { -ADJUSTMENT_AMOUNT };
    
    let prev_clip_end_time = if app.current_clip_index > 0 {
        app.clips.get(app.current_clip_index - 1).map(|c| c.end_time)
    } else { None };

    if let Some(clip) = app.clips.get_mut(app.current_clip_index) {
        let new_start_time = clip.start_time + adjustment;
        if new_start_time >= 0.0 && new_start_time < clip.end_time {
            if let Some(prev_end) = prev_clip_end_time {
                if new_start_time > prev_end { 
                    clip.start_time = new_start_time;
                    clip.is_manually_adjusted = true;
                }
            } else { 
                clip.start_time = new_start_time;
                clip.is_manually_adjusted = true;
            }
        }
    }
}

pub fn adjust_end_time(app: &mut App, increase: bool) {
    app.save_undo_state();
    let adjustment = if increase { ADJUSTMENT_AMOUNT } else { -ADJUSTMENT_AMOUNT };

    let next_clip_start_time = if app.current_clip_index < app.clips.len() - 1 {
        app.clips.get(app.current_clip_index + 1).map(|c| c.start_time)
    } else { None };

    if let Some(clip) = app.clips.get_mut(app.current_clip_index) {
        let new_end_time = clip.end_time + adjustment;
        if new_end_time > clip.start_time {
            if let Some(next_start) = next_clip_start_time {
                if new_end_time < next_start { 
                    clip.end_time = new_end_time; 
                    clip.is_manually_adjusted = true;
                }
            } else { 
                clip.end_time = new_end_time;
                clip.is_manually_adjusted = true;
            }
        }
    }
}

pub fn append_to_comment(app: &mut App, c: char) {
    app.save_undo_state();
    if let Some(clip) = app.clips.get_mut(app.current_clip_index) {
        clip.comment.push(c);
        clip.is_manually_adjusted = true;
    }
}

pub fn pop_from_comment(app: &mut App) {
    app.save_undo_state();
    if let Some(clip) = app.clips.get_mut(app.current_clip_index) {
        clip.comment.pop();
        clip.is_manually_adjusted = true;
    }
}

