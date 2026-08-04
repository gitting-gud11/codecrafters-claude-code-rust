use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use serde_json::{Value, json};
use std::fs;
use std::{env, process};

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let args = Args::parse();

    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    let client = Client::with_config(config);

    let model = env::var("LOCAL_MODEL").unwrap_or("anthropic/claude-haiku-4.5".to_string());

    let read_tool = json!(
                        {
                    "type":"function",
                    "function":{
                        "name":"Read",
                        "description": "Read and return the contents of a file",
                        "parameters":{
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type":"string",
                                    "description": "The path to the file to read"
                                }
                            },
                            "required":["file_path"]
                        }
                    }
                }
    );

    #[allow(unused_variables)]
    let response: Value = client
        .chat()
        .create_byot(json!({
            "messages": [
                {
                    "role": "user",
                    "content": args.prompt
                }
            ],
            "model": model,
            "tools": [read_tool]
        }))
        .await?;

    if let Some(tool_calls) = response["choices"][0]["message"]["tool_calls"].as_array() {
        let first_call = &tool_calls[0];
        let call_type = &first_call["type"];
        assert_eq!(call_type.as_str(), Some("function"));
        let name = first_call["function"]["name"]
            .as_str()
            .expect("Function name must be provided for a tool call");
        if name == "Read" {
            let arguments = serde_json::from_str::<serde_json::Value>(
                first_call["function"]["arguments"].as_str().unwrap(),
            )?;
            let message = fs::read_to_string(arguments["file_path"].as_str().unwrap())?;
            println!("{}", message);
        }
    } else if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
        println!("{}", content);
    }

    Ok(())
}
