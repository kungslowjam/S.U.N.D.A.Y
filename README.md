<div align="center">
  <img alt="SUNDAY" src="assets/SUNDAY_Horizontal_Logo.png" width="400">

  <p><i>Personal AI, On Personal Devices.</i></p>

  <p>
    <a href="https://sunday-ai.com/"><img src="https://img.shields.io/badge/project-SUNDAY-blue" alt="Project"></a>
    <a href="https://sunday-ai.github.io/SUNDAY/"><img src="https://img.shields.io/badge/docs-mkdocs-blue" alt="Docs"></a>
    <img src="https://img.shields.io/badge/python-%3E%3D3.10-blue" alt="Python">
    <img src="https://img.shields.io/badge/license-Apache%202.0-green" alt="License">
    <a href="https://discord.gg/YZZRxCAhmm"><img src="https://img.shields.io/badge/discord-join-7289da?logo=discord&logoColor=white" alt="Discord"></a>
  </p>
</div>

---

> **[Documentation](https://sunday-ai.github.io/SUNDAY/)**
>
> **[Project Site](https://sunday-ai.com/)**
>
> **[Leaderboard](https://sunday-ai.github.io/SUNDAY/leaderboard/)**
>
> **[Roadmap](https://sunday-ai.github.io/SUNDAY/development/roadmap/)**

## Why SUNDAY?

Personal AI agents are exploding in popularity, but nearly all of them still route intelligence through cloud APIs. Your "personal" AI continues to depend on someone else's server. At the same time, our [Intelligence Per Watt](https://www.intelligence-per-watt.ai/) research showed that local language models already handle 88.7% of single-turn chat and reasoning queries, with intelligence efficiency improving 5.3× from 2023 to 2025. The models and hardware are increasingly ready. What has been missing is the software stack to make local-first personal AI practical.

SUNDAY is that stack. It is an opinionated framework for local-first personal AI, built around three core ideas: shared primitives for building on-device agents; evaluations that treat energy, FLOPs, latency, and dollar cost as first-class constraints alongside accuracy; and a learning loop that improves models using local trace data. The goal is simple: make it possible to build personal AI agents that run locally by default, calling the cloud only when truly necessary. SUNDAY aims to be both a research platform and a production foundation for local AI, in the spirit of PyTorch.

## Installation

```bash
curl -fsSL https://sunday-ai.com/install.sh | bash
```

That's it. The installer handles everything: uv, the Python venv, Ollama, and pulling a small starter model. About 3 minutes on a typical broadband connection. Then:

```bash
sunday
```

The Rust extension and bigger models continue downloading in the background while you chat. Run `sunday doctor` to see status.

**Platforms:** macOS (Intel + Apple Silicon), Linux, WSL2 on Windows.

**Manual install / contributors:** see [docs/getting-started/install.md](docs/getting-started/install.md).

## Quick Start

```bash
curl -fsSL https://sunday.ai/install.sh | bash
sunday
```

`sunday init --preset <name>` switches to a starter config. Available presets: `morning-digest-mac`, `morning-digest-linux`, `morning-digest-minimal`, `deep-research`, `code-assistant`, `scheduled-monitor`, `chat-simple`.

## Starter Configs

Install any preset with one command:

```bash
uv run sunday init --preset morning-digest-mac   # or any preset below
```

> Prefix every `sunday ...` invocation with `uv run`, or activate the venv first (`source .venv/bin/activate`) so plain `sunday ...` works for the rest of your shell session.

| Preset | Use Case | What it does |
|--------|----------|-------------|
| `morning-digest-mac` | Daily Briefing (Mac) | Spoken briefing from email, calendar, health, news with Jarvis voice |
| `morning-digest-linux` | Daily Briefing (Linux) | Same, with vLLM support for GPU servers |
| `morning-digest-minimal` | Daily Briefing (minimal) | Just Gmail + Calendar, runs on any machine |
| `deep-research` | Research Assistant | Multi-hop research across indexed docs with citations |
| `code-assistant` | Code Companion | Agent with code execution, file I/O, and shell access |
| `scheduled-monitor` | Persistent Monitor | Stateful agent that runs on a schedule with memory |
| `chat-simple` | Simple Chat | Lightweight conversation, no tools needed |

```bash
# Example: Morning Digest on Mac
uv run sunday init --preset morning-digest-mac
uv run sunday connect gdrive          # one OAuth flow covers Gmail, Calendar, Tasks
uv run sunday digest --fresh          # generate and play your first briefing

# Example: Deep Research
uv run sunday init --preset deep-research
uv run sunday memory index ./docs/    # requires the Rust extension — see Setup above
uv run sunday ask "Summarize all emails about Project X"
```

### Skills

Skills teach agents how to better use tools and improve their reasoning. Every skill is a tool — agents discover them from a catalog and invoke them on demand.

```bash
# Install skills from public sources
sunday skill install hermes:arxiv
sunday skill sync hermes --category research

# Use skills with any agent
sunday ask "Use the code-explainer skill to explain this Python code: for i in range(5): print(i*2)"

# Optimize skills from your trace history
sunday optimize skills --policy dspy

# Benchmark the impact
sunday bench skills --max-samples 5 --seeds 42
```

Import from [Hermes Agent](https://github.com/NousResearch/hermes-agent) (~150 skills), [OpenClaw](https://github.com/openclaw/skills) (~13,700 community skills), or any GitHub repo. Skills follow the [agentskills.io](https://agentskills.io/specification) open standard.

See the [Skills User Guide](https://open-sunday.github.io/SUNDAY/user-guide/skills/) and [Skills Tutorial](https://open-sunday.github.io/SUNDAY/tutorials/skills-workflow/) for details.

### Built-in Agents

| Agent | Type | What it does |
|-------|------|-------------|
| `morning_digest` | Scheduled | Daily briefing from email, calendar, health, news — with TTS audio |
| `deep_research` | On-demand | Multi-hop research with citations across web and local docs |
| `monitor_operative` | Continuous | Long-horizon monitoring with memory, compression, and retrieval |
| `orchestrator` | On-demand | Multi-turn reasoning with automatic tool selection |
| `native_react` | On-demand | ReAct (Thought-Action-Observation) loop agent |
| `operative` | Continuous | Persistent autonomous agent with state management |
| `native_openhands` | On-demand | CodeAct — generates and executes Python code |
| `simple` | On-demand | Single-turn chat, no tools |

See the [User Guide](https://open-sunday.github.io/SUNDAY/user-guide/morning-digest/) and [Tutorials](https://open-sunday.github.io/SUNDAY/tutorials/) for detailed setup instructions.

Full documentation — including Docker deployment, cloud engines, development setup, and tutorials — at **[open-sunday.github.io/SUNDAY](https://open-sunday.github.io/SUNDAY/)**.

## Contributing

We welcome contributions! See the [Contributing Guide](CONTRIBUTING.md) for incentives, contribution types, and the PR process.

Quick start for contributors:

```bash
git clone https://github.com/open-sunday/SUNDAY.git
cd SUNDAY
uv sync --extra dev
uv run pre-commit install
uv run pytest tests/ -v
```

Browse the [Roadmap](https://open-sunday.github.io/SUNDAY/development/roadmap/) for areas where help is needed. Comment **"take"** on any issue to get auto-assigned.

## About

SUNDAY is part of [Intelligence Per Watt](https://www.intelligence-per-watt.ai/), a research initiative studying the efficiency of on-device AI systems. The project is developed at [Hazy Research](https://hazyresearch.stanford.edu/) and the [Scaling Intelligence Lab](https://scalingintelligence.stanford.edu/) at [Stanford SAIL](https://ai.stanford.edu/).

## Sponsors

<p>
  <a href="https://www.laude.org/">Laude Institute</a> &bull;
  <a href="https://datascience.stanford.edu/marlowe">Stanford Marlowe</a> &bull;
  <a href="https://cloud.google.com/">Google Cloud Platform</a> &bull;
  <a href="https://lambda.ai/">Lambda Labs</a> &bull;
  <a href="https://ollama.com/">Ollama</a> &bull;
  <a href="https://research.ibm.com/">IBM Research</a> &bull;
  <a href="https://hai.stanford.edu/">Stanford HAI</a>
</p>

## Citation
```bibtex
@misc{saadfalcon2026sunday,
  title={SUNDAY: Personal AI, On Personal Devices},
  author={Jon Saad-Falcon and Avanika Narayan and Herumb Shandilya and Hakki Orhun Akengin and Robby Manihani and Gabriel Bo and John Hennessy and Christopher R\'{e} and Azalia Mirhoseini},
  year={2026},
  howpublished={\url{https://scalingintelligence.stanford.edu/blogs/sunday/}},
}
```

## License

[Apache 2.0](LICENSE)
