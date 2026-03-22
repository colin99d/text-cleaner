use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde_json::json;
use std::env;
use std::fs::{OpenOptions, read_to_string};
use std::io::Write;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read command line argument (file path)
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];

    // 2. Read the Russian text inside the file
    let original_text = read_to_string(file_path)?;

    // Get API key from environment variable for security
    let api_key =
        env::var("OPENAI_API_KEY").expect("Please set the OPENAI_API_KEY environment variable");

    println!("Consulting ChatGPT 5.4 regarding your Russian grammar...");

    // 3. Call the API
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let prompt = format!(
        "You are a professional Russian language tutor. Below is text written by a student. \
        Please do the following:\n\
        1. Correct the text with minimal changes to keep it natural but accurate.\n\
        2. Provide a clear and very brief explanation of the grammatical concepts the student is struggling with. Be concise.
        If there are no mistakes say Нет ошибок with nothing else.\n\n\
        Format your response exactly like this:\n\
        <FIXED>\n(The corrected text)\n</FIXED>\n\
        <EXPLANATION>\n(The explanation)\n</EXPLANATION>\n\n\
        Student text:\n\"{}\"",
        original_text
    );

    let body = json!({
        "model": "gpt-5.4-mini",
        "messages": [
            {"role": "system", "content": "You are a helpful Russian tutor specializing in minimal corrections and grammatical clarity."},
            {"role": "user", "content": prompt}
        ],
        "temperature": 0.3
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    let response_json: serde_json::Value = res.json().await?;
    println!("{response_json:?}");
    let full_response = response_json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap();

    println!("{full_response}");

    // Parse the response parts (simple string splitting)
    let fixed_text = full_response
        .split("<FIXED>")
        .nth(1)
        .and_then(|s| s.split("</FIXED>").next())
        .unwrap_or("Correction not found.")
        .trim();

    let explanation = full_response
        .split("<EXPLANATION>")
        .nth(1)
        .and_then(|s| s.split("</EXPLANATION>").next())
        .unwrap_or("Explanation not found.")
        .trim();

    // 4. Append to the same file
    let mut file = OpenOptions::new().append(true).open(file_path)?;

    writeln!(file, "\n\n==========================================")?;
    writeln!(file, "CORRECTED RUSSIAN TEXT")?;
    writeln!(file, "==========================================")?;
    writeln!(file, "{}", fixed_text)?;
    writeln!(file, "\n==========================================")?;
    writeln!(file, "GRAMMATICAL EXPLANATION")?;
    writeln!(file, "==========================================")?;
    writeln!(file, "{}", explanation)?;

    println!("Done! Check {} for your corrections.", file_path);

    Ok(())
}
