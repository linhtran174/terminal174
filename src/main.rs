use anyhow::{Context, Result};
use colored::*;
use regex::Regex;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::{
    io::{self, Write},
    pin::Pin,
    process::Stdio,
};
use std::future::Future;
use tokio::fs;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    endpoint: String,
    api_key: String,
    model: String,
    system_prompt: String,
}

#[derive(Debug, Serialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

struct Session {
    id: String,
    messages: Vec<Message>,
}

impl Session {
    fn new() -> Self {
        Session {
            id: Uuid::new_v4().to_string(),
            messages: Vec::new(),
        }
    }

    fn add_message(&mut self, role: &str, content: String) {
        self.messages.push(Message {
            role: role.to_string(),
            content,
        });
    }
}

async fn load_config() -> Result<Config> {
    let config_dir = dirs::config_dir()
        .context("Could not find config directory")?
        .join("terminal174");
    fs::create_dir_all(&config_dir).await?;

    let config_path = config_dir.join("config.toml");
    if !config_path.exists() {
        let default_config = Config {
            endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            api_key: "your-api-key-here".to_string(),
            model: "gpt-3.5-turbo".to_string(),
            system_prompt: "You live inside a terminal, and everything typed by the human user will be forwarded to you first. 

Your task is to understand what they want to achieve, and assist them by:
- Talk to them in <talk></talk> tag
- Run terminal commands using <run_command></run_command> tag. The run result of each command run in each step will be provided to you in the next user prompt. 

Please reduce your talking to a minimal. For example, do not ask the user if they are typing in a correct command, instead just run that in the terminal.".to_string(),
        };
        let toml = toml::to_string(&default_config)?;
        fs::write(&config_path, toml).await?;
        println!("Created default config at {:?}", config_path);
        println!("Please edit it with your API key and settings before continuing.");
        std::process::exit(1);
    }

    let content = fs::read_to_string(&config_path).await?;
    let config: Config = toml::from_str(&content).unwrap_or_else(|_| {
        // Parse the existing content into a more flexible Value type
        let table = toml::from_str::<toml::Value>(&content)
            .map(|v| v.as_table().cloned().unwrap_or_default())
            .unwrap_or_else(|_| toml::value::Table::new());
        
        Config {
            endpoint: table.get("endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.openai.com/v1/chat/completions")
                .to_string(),
            api_key: table.get("api_key")
                .and_then(|v| v.as_str())
                .unwrap_or("your-api-key-here")
                .to_string(),
            model: table.get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("claude-3.5-sonnet")
                .to_string(),
            system_prompt: table.get("system_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("You live inside a terminal, and everything typed by the human user will be forwarded to you first. 

Your task is to understand what they want to achieve, and assist them by:
- Talk to them in <talk></talk> tag
- Run terminal commands using <run_command></run_command> tag. The run result of each command run in each step will be provided to you in the next user prompt. 

Please reduce your talking to a minimal. For example, do not ask the user if they are typing in a correct command, instead just run that in the terminal.")
                .to_string(),
        }
    });
    
    // Save the complete config back to file
    let toml = toml::to_string(&config)?;
    fs::write(&config_path, toml).await?;
    
    Ok(config)
}

async fn send_chat_request(config: &Config, session: &Session) -> Result<String> {
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", config.api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let request = ChatRequest {
        model: config.model.clone(),
        messages: session.messages.clone(),
    };

    let response = client
        .post(&config.endpoint)
        .headers(headers)
        .json(&request)
        .send()
        .await?
        .json::<ChatResponse>()
        .await?;

    Ok(response.choices[0].message.content.clone())
}

fn parse_system_info() -> String {
    let os = if cfg!(target_os = "windows") {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    
    format!(
        "<system_information>Operating System: {}\nShell: {}\nWorking Directory: {}</system_information>",
        os,
        std::env::var("SHELL").unwrap_or_else(|_| "unknown".to_string()),
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    )
}
async fn execute_command(command: &str) -> Result<String> {
    use tokio::io::{BufReader, AsyncBufReadExt};
    use tokio::process::Command;

    let mut child = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .stdin(Stdio::inherit())  // Allow user input
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    } else {
        Command::new("sh")
            .args(["-c", command])
            .stdin(Stdio::inherit())  // Allow user input
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
    };

    let mut output = String::new();

    // Read stdout and stderr concurrently
    if let Some(stdout) = child.stdout.take() {
        let mut stdout_reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = stdout_reader.next_line().await {
            println!("{}", line);
            output.push_str(&line);
            output.push('\n');
        }
    }

    if let Some(stderr) = child.stderr.take() {
        let mut stderr_reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = stderr_reader.next_line().await {
            eprintln!("{}", line.red());
            output.push_str(&line);
            output.push('\n');
        }
    }

    // Wait for the command to finish
    let status = child.wait().await?;
    
    if !status.success() {
        if let Some(code) = status.code() {
            output.push_str(&format!("\nProcess exited with code: {}", code));
        }
    }

    Ok(output)
}

fn process_command_chain<'a>(
    command: String,
    session: &'a mut Session,
    config: &'a Config,
    talk_re: &'a Regex,
    cmd_re: &'a Regex,
) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
    Box::pin(async move {
        println!("{} {}", "Running:".green(), &command);
        
        // Execute command
        match execute_command(&command).await {
            Ok(output) => {
                session.add_message(
                    "user",
                    format!("<command_result>{}</command_result>", output)
                );
            }
            Err(e) => {
                let error = format!("Error executing command: {}", e);
                println!("{}", error.red());
                session.add_message(
                    "user",
                    format!("<command_result>ERROR: {}</command_result>", error)
                );
            }
        }

        // Get AI response for command result
        let ai_response = send_chat_request(&config, &session).await?;
        session.add_message("assistant", ai_response.clone());

        // Process talk tags
        for cap in talk_re.captures_iter(&ai_response) {
            println!("{}", cap[1].yellow());
        }

        // Process any new commands
        for cap in cmd_re.captures_iter(&ai_response) {
            let new_cmd = cap[1].to_string();
            // Process the new command recursively
            process_command_chain(new_cmd, session, config, talk_re, cmd_re).await?;
        }

        Ok(())
    })
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config().await?;
    let mut session = Session::new();
    
    // Add system prompt
    session.add_message("system", config.system_prompt.clone());

    println!("{}", "Terminal174 - AI-powered terminal".green());
    println!("Type 'exit' to quit. Press Ctrl+C to interrupt AI or command execution.\n");

    let talk_re = Regex::new(r"<talk>([\s\S]*?)</talk>")?;
    let cmd_re = Regex::new(r"<run_command>([\s\S]*?)</run_command>")?;

    loop {
        print!("{} ", ">".blue());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" {
            break;
        }

        // Add system info and user input to conversation
        let sys_info = parse_system_info();
        session.add_message("user", format!("{}\n{}", input, sys_info));

        // Get AI response
        let ai_response = send_chat_request(&config, &session).await?;
        session.add_message("assistant", ai_response.clone());

        // Process talk tags
        for cap in talk_re.captures_iter(&ai_response) {
            println!("{}", cap[1].yellow());
        }

        // Process command tags with continuous chain
        for cap in cmd_re.captures_iter(&ai_response) {
            let cmd = cap[1].to_string();
            process_command_chain(cmd, &mut session, &config, &talk_re, &cmd_re).await?;
        }
    }

    Ok(())
}