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

    let write_tool = json!(
            {
      "type": "function",
      "function": {
        "name": "Write",
        "description": "Write content to a file",
        "parameters": {
          "type": "object",
          "required": ["file_path", "content"],
          "properties": {
            "file_path": {
              "type": "string",
              "description": "The path of the file to write to"
            },
            "content": {
              "type": "string",
              "description": "The content to write to the file"
            }
          }
        }
      }
    }

        );

    let bash_tool = json!(
            {
      "type": "function",
      "function": {
        "name": "Bash",
        "description": "Execute a shell command",
        "parameters": {
          "type": "object",
          "required": ["command"],
          "properties": {
            "command": {
              "type": "string",
              "description": "The command to execute"
            }
          }
        }
      }
    }
        );

    let mut messages = vec![json!({
        "role" : "user",
        "content" : args.prompt
    })];
    loop {
        let response: Value = client
            .chat()
            .create_byot(json!({
                "messages": messages,
                "model": model,
                "tools": [read_tool,write_tool,bash_tool]
            }))
            .await?;

        let received_message = &response["choices"][0]["message"];
        messages.push(received_message.clone());

        if let Some(tool_calls) = received_message["tool_calls"].as_array()
            && (!tool_calls.is_empty())
        {
            for tool_call in tool_calls {
                let result = execute_tool_call(tool_call)?;
                if !result.is_null() {
                    messages.push(result);
                }
            }
        } else {
            print_content(received_message);
            break;
        }
    }

    Ok(())
}

fn execute_tool_call(
    tool_call: &serde_json::Value,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    //Only execute supported tools
    let id = tool_call["id"]
        .as_str()
        .expect("Tool call must have an id")
        .to_string();
    let name = tool_call["function"]["name"]
        .as_str()
        .expect("Tool function call must have a provided name");

    let arguments: serde_json::Value =
        serde_json::from_str(tool_call["function"]["arguments"].as_str().unwrap())?;

    if name == "Read" {
        let content = fs::read_to_string(arguments["file_path"].as_str().unwrap())?;
        let result = json!({
            "role": "tool",
            "tool_call_id": id,
            "content" : content
        });
        Ok(result)
    } else if name == "Write" {
        fs::write(
            arguments["file_path"].as_str().unwrap(),
            arguments["content"].as_str().unwrap(),
        )?;
        let result = json!({
            "role": "tool",
            "tool_call_id" : id,
            "content" : (arguments["content"].as_str().expect("Write to file path succeeded").to_string())
        });
        Ok(result)
    } else if name == "Bash" {
        todo!();
    } else {
        Ok(serde_json::Value::Null)
    }
}

fn print_content(received_message: &serde_json::Value) {
    if let Some(content) = received_message["content"].as_str() {
        println!("{}", content);
    }
}
