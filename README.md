<div align="center">

# ⚡ Aeonic

### The most advanced AI/AGI/LLM Router & Orchestrator  
### in the distributed statespaces of the known universe

[![License: MIT](https://img.shields.io/badge/License-MIT-cyan.svg)](https://opensource.org/licenses/MIT)
[![Website](https://img.shields.io/badge/Website-aeonic.space-blueviolet)](https://aeonic.space)
[![GitHub Stars](https://img.shields.io/github/stars/AeronicWarrior/aeonic?style=flat&color=yellow)](https://github.com/AeronicWarrior/aeonic/stargazers)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/AeronicWarrior/aeonic/pulls)

**Model-agnostic · Blazing fast · Built in the open**

[Website](https://aeonic.space) · [Docs](https://github.com/AeronicWarrior/aeonic/wiki) · [Discussions](https://github.com/AeronicWarrior/aeonic/discussions) · [Report a Bug](https://github.com/AeronicWarrior/aeonic/issues)

</div>

---

## What is Aeonic?

Aeonic is an open-source AI/AGI/LLM router and orchestrator designed to intelligently route, compose, and manage calls across any combination of language models — local or cloud, small or frontier — with a unified API, zero vendor lock-in, and full observability.

Whether you're building a simple app that switches between cheap and powerful models based on task complexity, or orchestrating a galaxy-brain multi-agent pipeline across distributed statespaces, Aeonic gives you the primitives to do it cleanly.

---

## Features

| Capability | Description |
|---|---|
| **Intelligent Routing** | Semantically route prompts to the optimal model based on cost, capability, latency, and task type |
| **Multi-Agent Orchestration** | Compose agent pipelines with fan-out, fan-in, debate loops, and critic/verifier patterns |
| **Distributed Statespaces** | Manage context, vector stores, and episodic memory across distributed graph nodes |
| **Provider Abstraction** | Unified API across OpenAI, Anthropic, Gemini, Mistral, Ollama, Groq, Bedrock, and custom endpoints |
| **Policy Engine** | Declarative OPA/Rego-based routing policies — cost ceilings, compliance rules, rate limits as code |
| **Observability** | Full OpenTelemetry tracing, cost attribution, token accounting, and latency heatmaps |

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  Ingress Layer                  │
│  REST API · gRPC · WebSocket · Python · TS SDK  │
└──────────────────────┬──────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────┐
│              Core Orchestration                 │
│  Semantic Classifier → AEONIC ROUTER → State    │
│  Policy Engine ↕ Cost Optimizer ↕ Fallbacks     │
└──────────────────────┬──────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────┐
│                 Agent Layer                     │
│  Orchestrator → Workers → Memory/RAG → Critic   │
└──────────────────────┬──────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────┐
│               Provider Layer                    │
│  Anthropic · OpenAI · Gemini · Mistral · Ollama │
│  Groq · Bedrock · Custom Endpoints              │
└─────────────────────────────────────────────────┘
```

---

## Roadmap

- [x] **Phase 0** — Core Router & Provider Abstraction  
  Unified API, provider adapters, basic cost-aware routing, OpenAI-compatible endpoint

- [ ] **Phase 1** — Policy Engine & Semantic Routing *(in progress)*  
  OPA integration, task-type classifiers, declarative policies, latency-aware load balancing

- [ ] **Phase 2** — Multi-Agent Orchestration *(Q3 2025)*  
  Native agent primitives, fan-out/fan-in, debate loops, shared statespaces

- [ ] **Phase 3** — Distributed Memory & Long-Horizon Tasks *(Q4 2025)*  
  Vector-backed episodic memory, cross-session context, graph-of-thought execution

- [ ] **Phase 4** — AGI Orchestration Primitives *(2026)*  
  Self-improving routing policies, recursive agent spawning, full statespace exploration

---

## Getting Started

> **Note:** Aeonic is in early development. The API is not yet stable. Star the repo to follow along.

```bash
# Clone the repo
git clone https://github.com/AeronicWarrior/aeonic.git
cd aeonic

# Install dependencies (coming soon)
pip install aeonic
```

---

## Contributing

Aeonic is built in the open and contributions of all kinds are welcome — code, docs, ideas, and bug reports.

1. Fork the repo
2. Create a feature branch: `git checkout -b feat/your-feature`
3. Commit your changes: `git commit -m 'feat: add your feature'`
4. Push and open a Pull Request

See [CONTRIBUTING.md](CONTRIBUTING.md) for full guidelines.

---

## License

MIT — see [LICENSE](LICENSE) for details.

---

<div align="center">
  <strong>aeonic.space</strong> · Built with ⚡ by <a href="https://github.com/AeronicWarrior">AeronicWarrior</a> and contributors
</div>
