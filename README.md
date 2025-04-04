# Terminal174
The last AI-powered terminal - made with AI.

In the domain of terminal operations, we've found AI's perfect niche. Terminal commands are fundamentally completion tasks - we know exactly what we want to achieve, but the exact commands can be tedious to recall or write. While AI may struggle with creative tasks scoring only 65% on complex software engineering benchmarks, it excels at terminal operations with near-perfect accuracy. Ever needed to install a specific SDK but couldn't remember the exact command? Now you can simply type "npm install google cloud sdk" and let AI handle the details.

Unlike other AI terminals that require human approval for each command, Terminal174 is fully autonomous. Since we're dealing with trivial, well-defined tasks, why add extra clicks for approval? The focus is on streamlining your workflow, not adding unnecessary checkpoints. Plus, with local AI support and the fact that terminal commands are repetitive and non-sensitive, privacy concerns are minimal. You're free to focus on the creative aspects of your work while AI handles the routine terminal operations.

## Features

- 🚀 **Local First** - Bring your own model: Support for any OpenAI-compatible endpoints
- 🤖 **Fully Autonomous** - No approvals needed for command execution (Ctrl+C to interrupt)
- 📝 **Dead simple** - Code created with just a few prompts.

## Installation

```bash
cargo install --path .
```

The binary will be installed as `t174`.

## Configuration

On first run, the app will create a default configuration file at:
- Linux: `~/.config/terminal174/config.toml`
- macOS: `~/Library/Application Support/terminal174/config.toml`
- Windows: `%APPDATA%\terminal174\config.toml`

Edit this file to set your:
- API endpoint (OpenAI or compatible)
- API key
- System prompt (how you want the AI to behave)

Example config.toml:
```toml
endpoint = "https://api.openai.com/v1/chat/completions"
api_key = "your-api-key-here"
model = "gpt-3.5-turbo"  # Or any other model supported by your endpoint
system_prompt = """You are a helpful command-line assistant.
You can either talk or run commands.
To talk, use <talk>your message</talk>.
To run a command, use <run_command>command here</run_command>."""
```

## Usage
Just run `t174` and start typing what you want to do. The AI will either:
- Talk to you 
- Execute commands
- Or both!

## Control

- Type `exit` to quit
- Press `Ctrl+C` to interrupt AI processing or command execution

## License

MIT