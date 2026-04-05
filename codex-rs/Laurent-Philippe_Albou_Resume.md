# Laurent-Philippe Albou

## 🎯 Profile

**Rust & AI Systems Engineer** | **Codex Agent Orchestration Specialist**

Passionate about building robust, long-running agentic workflows that prioritize
quality and correctness over speed. Experienced in Rust, TypeScript, Python, and
AI/ML operations with a focus on practical application of cutting-edge AI models
and agent frameworks.

📧 Contact: [GitHub](https://github.com/Laurent-Philippe-Albou) |
🌐 [Codex Unleashed Fork](https://github.com/lpalbou/codex)

---

## 🛠️ Technical Expertise

### Programming Languages
* **Rust** — Advanced (workspace maintainance, CLI tooling, agent core)
* **TypeScript/JavaScript** — Expert (workflow orchestration, browser-based agents)
* **Python** — Advanced (ML operations, data science)
* **Go** — Intermediate (llama-swap integration, deployment)

### AI/ML & Agent Systems
* **Model Context Protocol (MCP)** — Client & Server implementation
* **OpenAI Codex CLI** — Core business logic, TUI, protocol layers
* **LangChain/LlamaIndex patterns** — Agent orchestration via open-source frameworks
* **LLM Inference** — llama.cpp, vLLM, TabbyAPI integration
* **Model Decensoring** — Abliteration techniques, Optuna optimization
* **Prompt Engineering** — Advanced workflows, skill-based agents

### Development Tools
* **Cargo/Rust** — Workspace management, crate development
* **npm/yarn** — Package management, NPM ecosystem
* **Bazel** — Build system (codex-rs)
* **CI/CD** — GitHub Actions, release automation

---

## 💼 Featured Projects

### 🚀 **codex-unleashed** (Maintenance)
*Personal fork of OpenAI's Codex CLI (rust-v0.87.0)*

**Description:**
Empowers agentic workflows with a philosophy of "depth over speed" —
long-running sessions (1h+) that produce higher-quality outputs through careful
orchestration. Features a dedicated `/context dashboard`, hardened OSS provider
support, and enhanced collaboration capabilities.

**Key Contributions:**
* Added `/context dashboard` for improved session observability
* Implemented `/save` and `/collab` features for workflow persistence
* TUI improvements including `--full` export mode
* Enhanced model configuration with `gpt-5.2 xhigh` as default
* Added `worker_model_override` feature flag for flexible deployment
* Optimized collaboration thread management

**Links:**
* [Repository](https://github.com/lpalbou/codex)
* [Installation](https://github.com/lpalbou/codex#quick-start)

---

### 🧠 **Heretic** (Contributor)
*Decensoring Language Models via Abliteration*

**Description:**
Advanced tool for removing safety alignment from transformer models using
directional ablation ("abliteration") combined with TPE-based optimization.
Works completely automatically, producing decensored models that retain original
intelligence while minimizing refusals.

**Technical Details:**
* Optuna-powered parameter optimization
* Multi-GPU support with VRAM usage tracking
* Qwen3.5 MoE hybrid layer support
* Integrated benchmarking system
* Co-minimizing refusals and KL divergence

**Links:**
* [Repository](https://github.com/heretic-org/heretic)
* [Discord](https://discord.gg/gdXc48gSyT)
* [Trendshift](https://trendshift.io/repositories/20538)

---

### 🎭 **Oh-My-Codex (OMX)** (Contributor)
*Workflow Layer for Codex with Skills & Agents*

**Description:**
Extends Codex CLI with advanced workflow capabilities, reusable role/task
invocations, and skill-based agents. Manages project guidance, plans, logs,
and state in `.omx/` directories.

**Features:**
* Start stronger Codex sessions with optimized defaults
* Invoke workflows with skill keywords (`$plan`, `$ralph`, `$team`)
* MCP server support for cross-client agent collaboration
* Runtime help system for complex tasks
* Multi-agent orchestration patterns

**Links:**
* [Repository](https://github.com/Yeachan-Heo/oh-my-codex)
* [Website](https://yeachan-heo.github.io/oh-my-codex-website/)
* [Discord](https://discord.gg/PUwSMR9XNk)

---

### 🤖 **OpenCode** (Contributor)
*Open Source AI Coding Agent*

**Description:**
Full-featured AI coding agent supporting multiple providers (OpenAI, Anthropic),
with sophisticated file operations, tool execution, and context management.

**Contributions:**
* Account token management and refresh
* Model display name handling
* Bash tool execution via Effect ChildProcess
* Plugin/config loading improvements
* Theme-only plugin package support
* TUI scroll configuration

**Links:**
* [Website](https://opencode.ai)
* [Discord](https://opencode.ai/discord)
* [NPM](https://www.npmjs.com/package/opencode-ai)

---

### 🎬 **Open Claude Cowork** (Contributor)
*Claude Agent SDK Integration*

**Description:**
Secure database and tool integration layer for Claude agents, featuring
advanced tool routing via Composio and demonstrating secure agent patterns.

**Links:**
* [Composio Integration](https://platform.composio.dev)
* [Claude Agent SDK](https://platform.claude.com/docs/en/agent-sdk/overview)

---

### 🌫️ **Dust.tt** (Contributor)
*Custom AI Agent Platform*

**Description:**
Enterprise-grade AI agent platform with features including graceful agent stopping,
scheduling, blocked action support, email agent integration (SendGrid), Slack
channel linking, and comprehensive activity tracking.

**Features:**
* Graceful stop patterns for agent loops
* Flexible scheduling system
* Transaction ID uniqueness guarantees
* Email agent webhook handling
* Multi-agent workspace management

**Links:**
* [Platform](https://dust.tt)
* [User Guides](https://docs.dust.tt)
* [Jobs](https://jobs.ashbyhq.com/dust)

---

### 🔄 **llama-swap** (User/Integrator)
*Model Hot-Swap Infrastructure*

**Description:**
Deploy multiple generative AI models and hot-swap between them on demand.
Zero dependencies, one binary, configuration file — works with any OpenAI/
Anthropic-compatible server (llama.cpp, vLLM, tabbyAPI, stable-diffusion.cpp).

**Supported APIs:**
* OpenAI: `/v1/completions`, `/v1/chat/completions`, `/v1/responses`, `/v1/embeddings`
* Anthropic: `/v1/messages`, `/v1/messages/count_tokens`
* Speech: `/v1/audio/speech`
* Transcription: `/v1/audio/transcriptions`
* Images: `/v1/images/generations`, `/v1/images/edits`

**Links:**
* [Repository](https://github.com/mostlygeek/llama-swap)

---

### ⚔️ **Claw Code** (Rust Port in Progress)
*Harness Tool Optimizations*

**Description:**
Contributing to the Rust port of `claw-code` — the fastest repository to
surpass 50K GitHub stars. Focus on memory-safe harness runtime and improved
tool integration.

**Links:**
* [Main Repo](https://github.com/instructkr/claw-code)
* [Rust Port Branch](https://github.com/instructkr/claw-code/tree/dev/rust)

---

## 🧪 Technical Focus Areas

### Agent Orchestration
* Long-running sessions with state persistence
* Context management and optimization
* Multi-agent collaboration patterns
* Tool calling and result handling

### Rust Systems Programming
* Workspace and crate management
* CLI tool development
* TUI applications with Ratatui
* Protocol and serialization layers

### AI/ML Operations
* Local model inference management
* Model swapping and hot-reloading
* Decensoring and safety alignment modification
* Performance optimization and benchmarking

---

## 📚 Philosophy

**Depth over speed:** One carefully orchestrated agentic session (even 1h+)
produces more valuable output than many fast iterations that introduce avoidable
mistakes. Quality and correctness take precedence over quick turnaround.

---

## 🔮 Current Interests

* Advancing Rust in the AI agent ecosystem
* Improving long-running agentic workflows
* Enhancing MCP protocol interoperability
* Building better tool harnesses for LLMs
* Open-source agent platform development

---

## 📜 License

MIT License — feel free to use, modify, and distribute.

---

> "The best code is the code that gets it right, not the code that gets it
> fast. Patience and careful orchestration win in the long run."
