# zcode

CLI coding agent powered by LLMs. Like Claude Code or Cursor, but in the terminal.

## Setup

Choose a provider (**OpenAI** or **Gemini**) and set the matching API key.

### OpenAI (default)

```bash
export OPENAI_API_KEY="sk-..."
```

### Gemini

```bash
export GEMINI_API_KEY="your-gemini-api-key"
```

Or create `~/.config/zcode/config.toml`:

```toml
provider = "openai"   # or "gemini"
api_key = "sk-..."   # for OpenAI
gemini_api_key = "..."  # for Gemini
```

## Usage

```bash
# Single prompt (uses default provider from config/env)
zcode -p "Create a hello world in Rust"

# Force OpenAI
zcode --provider openai -p "List files in current dir"

# Force Gemini
zcode --provider gemini -p "List files in current dir"

# Interactive REPL
zcode
```

Provider can also be set via `ZCODE_PROVIDER` env var (e.g. `openai` or `gemini`).

## Tools

The agent can create files, read files, write files, list directories, run shell commands, and create directories. It works in the current directory.
