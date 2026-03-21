# Self-Smart

An AI coding agent powered by local LLM (vLLM/Ollama) that aims to rival Claude Code.

## Features

- **Code Generation**: Generate code from natural language descriptions
- **Code Analysis**: Analyze code structure, metrics, and quality
- **Debugging**: Detect and fix bugs automatically
- **Refactoring**: Improve code quality with safe transformations
- **Testing**: Generate and run tests
- **Documentation**: Generate documentation automatically
- **Git Integration**: Automatic commits and version tracking

## Requirements

- Rust 1.70+
- vLLM or Ollama running locally
- Git

## Installation

```bash
cargo install --path .
```

## Usage

### Single prompt mode

```bash
self-smart --prompt "Analyze src/main.rs and suggest improvements"
```

### Interactive mode

```bash
self-smart --interactive
```

### Auto-commit mode

```bash
self-smart --auto-commit --prompt "Refactor the code in src/"
```

### Custom endpoint

```bash
self-smart --endpoint http://localhost:8000 --model Qwen/Qwen2.5-Coder-32B
```

## Configuration

| Flag | Default | Description |
|------|---------|-------------|
| `--endpoint` | `http://localhost:48000` | vLLM API endpoint |
| `--model` | `Intel/Qwen3.5-9B-int4-AutoRound` | Model ID |
| `--workdir` | `.` | Working directory |
| `--auto-commit` | `false` | Enable auto-commit mode |

## Architecture

```
Self-Smart Agent
├── LLM Layer (vLLM/Ollama client)
├── Tool System (pluggable tools)
│   ├── Code Analyzer
│   ├── Code Generator
│   ├── Debugger
│   ├── Refactorer
│   ├── Test Runner
│   └── Doc Generator
├── Git Integration (gix)
└── Agent Loop (ReAct pattern)
```

## License

MIT
