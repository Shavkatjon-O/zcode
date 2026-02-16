# zcode

CLI coding agent powered by OpenAI — multi-step reasoning in the terminal.

## Demo

zcode plans and runs tasks: create projects, edit files, run commands — from a single prompt.

![zcode demo — creating a React app from a prompt](assets/demo.png)

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
zcode -p "Create a hello world in Rust"   # one-off prompt
zcode                                    # interactive REPL
```

## Capabilities

Runs in the current directory. Can create/edit files, list dirs, run shell commands.
