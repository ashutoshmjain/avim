use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph, ListState},
    Frame,
};
use crate::app::{App, AppState, Mode};

pub fn ui(f: &mut Frame, app: &mut App) {
    let main_chunks = if app.debug_mode {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
            .split(f.size())
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)].as_ref())
            .split(f.size())
    };

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)].as_ref())
        .split(main_chunks[0]);

    render_transcript_panel(f, app, left_chunks[0]);
    render_status_panel(f, app, left_chunks[1]);

    if app.debug_mode {
        render_debug_panel(f, app, main_chunks[1]);
    }
}

fn render_transcript_panel(f: &mut Frame, app: &mut App, area: Rect) {
    match &app.state {
        AppState::Loading(message) => {
            let loading_text = Paragraph::new(message.as_str())
                .style(Style::default().fg(Color::Yellow))
                .alignment(ratatui::layout::Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title("Loading"));
            f.render_widget(loading_text, area);
        }
        AppState::Ready => {
            let list_width = area.width.saturating_sub(6);
            let mut list_items = Vec::new();
            
            for (i, clip) in app.clips.iter().enumerate() {
                let line_number = format!("{:>4} ", i + 1);
                let timestamp = format!("[{:0>5.2}-{:0>5.2}]", clip.start_time, clip.end_time);
                
                let mut full_text_spans = vec![
                    Span::raw(timestamp.clone()),
                    Span::raw(format!(" [{}] ", clip.speaker)),
                ];

                if app.mode == Mode::Adjust && i == app.current_clip_index + 1 {
                    let words: Vec<&str> = clip.transcript.split_whitespace().collect();
                    for (word_idx, word) in words.iter().enumerate() {
                        let style = if word_idx <= app.adjust_word_index {
                            Style::default().bg(Color::Yellow).fg(Color::Black)
                        } else {
                            Style::default()
                        };
                        full_text_spans.push(Span::styled(format!("{} ", word), style));
                    }
                } else {
                    full_text_spans.push(Span::raw(clip.transcript.clone()));
                }
                
                let text_content_for_wrap = full_text_spans.iter().map(|s| s.content.as_ref()).collect::<String>();
                let wrapped_lines = textwrap::wrap(&text_content_for_wrap, list_width as usize);
                
                let mut lines = Vec::new();
                for (idx, line_str) in wrapped_lines.iter().enumerate() {
                     let prefix = if idx == 0 { line_number.clone() } else { "     ".to_string() };
                     lines.push(Line::from(vec![Span::raw(prefix), Span::raw(line_str.to_string())]));
                }
                
                if !clip.comment.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("     // {}", clip.comment),
                        Style::default().fg(Color::Green),
                    )));
                }
                
                let text = if i == app.current_clip_index {
                    Text::from(lines).patch_style(Style::default().fg(Color::Black).bg(Color::LightCyan))
                } else if app.mode == Mode::Adjust && i == app.current_clip_index + 1 {
                    Text::from(lines).patch_style(Style::default().add_modifier(Modifier::BOLD))
                } else {
                    Text::from(lines)
                };
                list_items.push(ListItem::new(text));
            }

            let list_title = format!("Transcript ({} clips)", app.clips.len());
            let mut state = ListState::default();
            state.select(Some(app.current_clip_index));
            
            let list = List::new(list_items)
                .block(Block::default().borders(Borders::ALL).title(list_title));
            f.render_stateful_widget(list, area, &mut state);
        }
    }
}

fn render_status_panel(f: &mut Frame, app: &mut App, area: Rect) {
    let status_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)].as_ref())
        .split(area);
    
    let mode_text = match app.mode {
        Mode::Normal => "-- NORMAL --".to_string(),
        Mode::Command => format!(":{}", app.command_input),
        Mode::Insert => "-- INSERT --".to_string(),
        Mode::Adjust => "-- ADJUST --".to_string(),
        Mode::Visual => "-- VISUAL --".to_string(),
    };
    let mode_bar = Paragraph::new(mode_text)
        .style(Style::default().fg(Color::White).bg(Color::DarkGray));
    f.render_widget(mode_bar, status_chunks[0]);

    match app.mode {
        Mode::Command => {
            f.set_cursor(
                status_chunks[0].x + 1 + app.command_input.len() as u16,
                status_chunks[0].y,
            );
        }
        _ => {}
    }

    let message_bar = Paragraph::new(app.status_message.as_str())
        .style(Style::default().fg(Color::White));
    f.render_widget(message_bar, status_chunks[1]);
}

fn render_debug_panel(f: &mut Frame, app: &App, area: Rect) {
    let log_items: Vec<ListItem> = app.debug_log.iter()
        .map(|msg| ListItem::new(Text::raw(msg)))
        .collect();

    let log_list = List::new(log_items)
        .block(Block::default().borders(Borders::ALL).title("Debug Log"));
    
    f.render_widget(log_list, area);
}

