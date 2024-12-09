#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use serde_json::json;
use std::error::Error;
use lingua::{Language, LanguageDetector, LanguageDetectorBuilder};
use std::env;
use reqwest::{Client};
use std::path::Path;
use std::fs::File;
use zip::{write::FileOptions, ZipWriter};
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;
use dotenv::dotenv;

#[derive(Debug, serde::Deserialize)]
struct QwenResponse {
    output: Output,
    #[serde(default)]
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Output {
    text: String,
}

// 添加日志记录函数
fn log_to_file(message: &str) {
    let now = Local::now();
    let log_path = std::env::temp_dir().join("translator_app.log");
    
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path) 
    {
        let log_message = format!("[{}] {}\n", now.format("%Y-%m-%d %H:%M:%S"), message);
        let _ = file.write_all(log_message.as_bytes());
    }
}

async fn translate_text(text: &str) -> Result<String, Box<dyn Error>> {
    log_to_file(&format!("Translating text: {}", text));
    
    let detector: LanguageDetector = LanguageDetectorBuilder::from_languages(
        &[Language::English, Language::Chinese, Language::Japanese, Language::Korean]
    ).build();
    
    let detected_language = detector.detect_language_of(text)
        .ok_or("Could not detect language")?;
    
    log_to_file(&format!("Detected language: {:?}", detected_language));
    
    if detected_language == Language::English {
        return Ok(text.to_string());
    }

    let api_key = match env::var("QWEN_API_KEY") {
        Ok(key) => key,
        Err(e) => {
            log_to_file(&format!("Failed to get QWEN_API_KEY in translate_text: {}", e));
            return Err("QWEN_API_KEY environment variable not set".into());
        }
    };

    let client = Client::new();

    let response = client
        .post("https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&json!({
            "model": "qwen-max",
            "input": {
                "messages": [
                    {
                        "role": "system",
                        "content": "You are a translator. Translate the following text to English. Only respond with the translation, no explanations or additional text."
                    },
                    {
                        "role": "user",
                        "content": text
                    }
                ]
            }
        }))
        .send()
        .await?;

    let response_text = response.text().await?;
    let parsed_response: Result<QwenResponse, _> = serde_json::from_str(&response_text);
    
    match parsed_response {
        Ok(response) => {
            if let Some(code) = response.code {
                if code != "200" {
                    return Err(format!("API Error: {}", response.message.unwrap_or_default()).into());
                }
            }
            Ok(response.output.text.trim().to_string())
        },
        Err(e) => {
            eprintln!("Response text: {}", response_text);
            Err(format!("Failed to parse API response: {}", e).into())
        }
    }
}

#[tauri::command]
async fn translate_filename(filename: &str) -> Result<String, String> {
    log_to_file(&format!("Attempting to translate filename: {}", filename));
    
    let parts: Vec<&str> = filename.rsplitn(2, '.').collect();
    let (name, ext) = match parts.as_slice() {
        [ext, name] => {
            log_to_file(&format!("Split filename: name='{}', ext='{}'", name, ext));
            (name, Some(ext))
        },
        [name] => {
            log_to_file(&format!("No extension found, name='{}'", name));
            (name, None)
        },
        _ => unreachable!(),
    };

    match translate_text(name).await {
        Ok(translated_name) => {
            let translated_name = translated_name.replace(" ", "_");
            let result = match ext {
                Some(ext) => format!("{}.{}", translated_name, ext),
                None => translated_name,
            };
            log_to_file(&format!("Successfully translated to: {}", result));
            Ok(result)
        },
        Err(e) => {
            let error_msg = format!("Translation error for '{}': {}", name, e);
            log_to_file(&error_msg);
            Err(error_msg)
        },
    }
}

#[tauri::command]
async fn create_zip_file(
    files: Vec<(String, String)>,
    zip_path: String,
) -> Result<(), String> {
    let path = Path::new(&zip_path);
    let file = File::create(path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Stored);

    for (src_path, filename) in files {
        let mut src_file = File::open(&src_path).map_err(|e| e.to_string())?;
        let mut buffer = Vec::new();
        src_file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
        
        zip.start_file(filename, options).map_err(|e| e.to_string())?;
        zip.write_all(&buffer).map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn create_temp_file(fileName: String, content: Vec<u8>) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let temp_path = temp_dir.join(&fileName);
    
    std::fs::write(&temp_path, content)
        .map_err(|e| e.to_string())?;
    
    Ok(temp_path.to_string_lossy().into_owned())
}

#[tauri::command]
fn get_temp_dir() -> String {
    std::env::temp_dir()
        .to_string_lossy()
        .into_owned()
}

fn main() {
    if let Err(e) = dotenv() {
        log_to_file(&format!("Failed to load .env file: {}", e));
    } else {
        log_to_file("Successfully loaded .env file");
    }
    
    match env::var("QWEN_API_KEY") {
        Ok(key) => {
            let masked_key = if key.len() > 4 {
                format!("{}...", &key[0..4])
            } else {
                "***".to_string()
            };
            log_to_file(&format!("QWEN_API_KEY found: {}", masked_key));
        },
        Err(e) => {
            log_to_file(&format!("Failed to read QWEN_API_KEY: {}", e));
        }
    }
    
    log_to_file("Application starting...");
    
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)]
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            translate_filename,
            create_zip_file,
            create_temp_file,
            get_temp_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

