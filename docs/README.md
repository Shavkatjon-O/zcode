# zcode

CLI coding agent powered by LLMs. Like Claude Code or Cursor, but in the terminal.

## Setup

Set your OpenAI API key:

```bash
export OPENAI_API_KEY="sk-..."
```

Or create `~/.config/zcode/config.toml`:

```toml
api_key = "sk-..."
```

## Usage

```bash
# Single prompt
zcode -p "Create a hello world in Rust"

# Interactive REPL
zcode
```

## Tools

The agent can create files, read files, write files, list directories, run shell commands, and create directories. It works in the current directory.