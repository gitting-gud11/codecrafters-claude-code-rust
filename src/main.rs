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

    let tool_calls_json = response["choices"][0]["message"]["tool_calls"].as_array();

    #[allow(unused_variables)]
    match tool_calls_json {
        Option::Some(tool_calls) => {
            let first_call = &tool_calls[0];
            let call_id = &first_call["id"];
            let call_type = &first_call["type"];
            assert_eq!(call_type, "function");
            let function_name = &first_call["function"]["name"];
            let arguments_json_str=first_call["function"]["arguments"].as_str().unwrap_or_default();
            let arguments_res=serde_json::from_str::<serde_json::Map<String,Value>>(arguments_json_str);

            // if(arguments_res.is_err()){
            //     eprintln!("{}",arguments_res.unwrap_err());
            //     return Ok(()); //this is a temporary place holder
            // }
            if let Some(error_occurred) = arguments_res.as_ref().err(){
                eprintln!("{}",error_occurred);
                return Ok(()); //temp place holder
            }
            // let file_path_opt=arguments["file_path"].as_str()
            // let arguments : serde_json::Map<String,serde_json::Value>= serde_json::from_str(first_call["function"]["arguments"].as_str().unwrap_or_default());
            // let arguments = json!(
            //     first_call["function"]["arguments"]
            //         .as_str()
            //         .unwrap_or_default()
            // );
            // println!("arguments json:{}",arguments.as_str().unwrap_or_default());
            // let m=arguments_res.ok()
            let argument=arguments_res.unwrap();
            let file_path_opt=argument["file_path"].as_str();
            // let file_path_opt = arguments_res.unwrap()["file_path"].as_str();

            match file_path_opt {
                Option::Some(path) => {
                    //Need to condense this into helper functions
                    match fs::read_to_string(path) {
                        Ok(message) => println!("{}", message),
                        Err(e) => eprintln!("{}", e),
                    }
                }
                Option::None => eprintln!("File path not provided"),
            }
        }
        Option::None => {
            if let Some(content) = response["choices"][0]["message"]["content"].as_str() {
                println!("{}", content);
            }
        }
    }

    Ok(())
}
