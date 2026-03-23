use chrono::Local;
use clap::{Parser, Subcommand};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::json;
use std::env;
use std::fs::{File, OpenOptions, read_to_string};
use std::io::Write;
use std::path::PathBuf;

const COACH_PROMPT: &str = r#"
    You are my Russian language coach. I will give you a short Russian essay or paragraph.
    Your task:
    1. Correct **only grammar, spelling, and clearly unnatural phrasing**.
    2. English is my native language, please make my sentences sounds more natural
    3. Show corrections using **bold formatting** for changed words or endings.
    4. After the corrected text, provide a **short, precise explanation** of the mistakes, one per bullet.
    5. Keep the original structure and wording as much as possible.
    Output format:
    Corrected text:
    [Your corrected text with bold changes]
    Mistakes and explanations:
    - [Rule 1: explanation]
    ...
    "#;

const GENERATOR_PROMPT: &str = r#"
    You are a Russian language professor.
    Generate one interesting and specific C1-level Russian writing prompt (essay topic).
    The topic should be suitable for a 200-word response.
    Format your response exactly like this:
    Тема: [Topic name]
    Задание: [Short description of what to write about]
    "#;

#[derive(Parser)]
#[command(name = "text-cleaner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check an existing file for Russian grammar errors
    Check { path: PathBuf },
    /// Fetch a new B2 writing prompt from ChatGPT and create a file
    Prompt { path: Option<PathBuf> },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let api_key =
        env::var("OPENAI_API_KEY").expect("Please set the OPENAI_API_KEY environment variable");

    match cli.command {
        Commands::Check { path } => run_check(path, &api_key).await?,
        Commands::Prompt { path } => run_prompt(path, &api_key).await?,
    }

    Ok(())
}

async fn run_check(file_path: PathBuf, api_key: &str) -> Result<(), Box<dyn std::error::Error>> {
    let original_text = read_to_string(&file_path)?;
    println!("Consulting ChatGPT 5.4 regarding your grammar...");

    let response = call_gpt(api_key, COACH_PROMPT, &original_text, 0.3).await?;

    let mut file = OpenOptions::new().append(true).open(&file_path)?;
    writeln!(file, "\n\n==========================================")?;
    writeln!(file, "{}", response)?;

    println!("Done! Check {:?} for your corrections.", file_path);
    Ok(())
}

async fn run_prompt(
    dir_path: Option<PathBuf>,
    api_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Fetching a fresh B2 prompt from ChatGPT...");

    // 1. Get the prompt from AI
    let ai_prompt = call_gpt(
        api_key,
        GENERATOR_PROMPT,
        "Please generate a new B2 prompt.",
        1.0,
    )
    .await?;

    // 2. Prepare file path
    let target_dir = dir_path.unwrap_or_else(|| PathBuf::from("."));
    let date_str = Local::now().format("%Y-%m-%d").to_string();
    let final_path = target_dir.join(format!("{}.txt", date_str));

    // 3. Write to file
    let mut file = File::create(&final_path)?;
    writeln!(file, "{}\n", ai_prompt)?;
    writeln!(file, "(Напишите ваш ответ ниже)\n---")?;

    println!("Success! New prompt created in: {:?}", final_path);
    Ok(())
}

/// Helper function to handle OpenAI API calls
async fn call_gpt(
    api_key: &str,
    system_msg: &str,
    user_msg: &str,
    temperature: f64,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let body = json!({
        "model": "gpt-5.4",
        "messages": [
            {"role": "system", "content": system_msg},
            {"role": "user", "content": user_msg}
        ],
        "temperature": temperature
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    let response_json: serde_json::Value = res.json().await?;

    // Track tokens (internal helper)
    if let Some(usage) = response_json.get("usage") {
        println!("[Tokens used: {}]", usage["total_tokens"]);
    }

    let content = response_json["choices"][0]["message"]["content"]
        .as_str()
        .ok_or("Failed to get content from GPT")?;

    Ok(content.to_string())
}
