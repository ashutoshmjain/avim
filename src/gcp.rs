use serde::{Deserialize, Serialize};
use std::{env, error::Error};
use base64::{Engine as _, engine::general_purpose};
use crate::app::Clip;

#[derive(Serialize)] pub struct GeminiRequest { pub contents: Vec<Content> }
#[derive(Serialize)] pub struct Content { pub parts: Vec<Part> }
#[derive(Serialize)] #[serde(untagged)] pub enum Part { Text { text: String }, InlineData { inline_data: InlineData } }
#[derive(Serialize)] pub struct InlineData { #[serde(rename = "mimeType")] pub mime_type: String, pub data: String }
#[derive(Deserialize, Debug)] pub struct GeminiResponse { pub candidates: Vec<Candidate> }
#[derive(Deserialize, Debug)] pub struct Candidate { pub content: Option<ResponseContent> }
#[derive(Deserialize, Debug)] pub struct ResponseContent { pub parts: Vec<ResponsePart> }
#[derive(Deserialize, Debug)] pub struct ResponsePart { pub text: String }

pub async fn transcribe_chunk(audio_data: &[u8]) -> Result<Vec<Clip>, Box<dyn Error + Send + Sync>> {
    let api_key = env::var("GEMINI_API_KEY").map_err(|_| "GEMINI_API_KEY not set in environment")?;
    let audio_base64 = general_purpose::STANDARD.encode(audio_data);

    let prompt = "Transcribe this audio. Identify speakers. Segment the audio into clips based on pauses or speaker changes. Provide the output as a valid JSON array of objects, where each object has 'id', 'speaker', 'transcript', 'start_time', and 'end_time'. The JSON should be the only thing in your response.";

    let request_body = GeminiRequest {
        contents: vec![Content {
            parts: vec![
                Part::Text { text: prompt.to_string() },
                Part::InlineData {
                    inline_data: InlineData {
                        mime_type: "audio/wav".to_string(),
                        data: audio_base64,
                    },
                },
            ],
        }],
    };

    let client = reqwest::Client::new();
    let res = client
        .post(format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-1.5-flash-latest:generateContent?key={}", api_key))
        .json(&request_body)
        .send()
        .await?;

    if !res.status().is_success() {
        return Err(format!("API Error: {}", res.text().await?).into());
    }

    let response_body: GeminiResponse = res.json().await?;
    let text_part = response_body.candidates.get(0)
        .and_then(|c| c.content.as_ref())
        .and_then(|c| c.parts.get(0))
        .map(|p| p.text.trim().trim_start_matches("```json").trim_end_matches("```").trim())
        .unwrap_or_default();

    let clips: Vec<Clip> = serde_json::from_str(text_part)
        .map_err(|e| format!("Failed to parse JSON from Gemini response: {}. Response text: '{}'", e, text_part))?;

    Ok(clips)
}

