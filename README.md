# S.U.N.D.A.Y

Personal local AI assistant experiments for Windows, based on the OpenJarvis/SUNDAY codebase and adapted for a local-first desktop workflow.

<p align="center">
  <img alt="SUNDAY" src="assets/SUNDAY_Horizontal_Logo.png" width="420">
</p>

## What This Repo Is

This is a personal fork/customization of the OpenJarvis/SUNDAY assistant stack. The current focus is practical local usage:

- local `llama.cpp` inference through `llama-server`
- a FastAPI backend with OpenAI-compatible chat routes
- a Vite/Tauri-style desktop frontend
- local config and startup scripts for Windows
- speech input/output experiments
- isolated MiniCPM-o omni experiments under `omni-lab/`

This repository is not the official upstream project.

## Credits

This project is built on top of and inspired by:

- OpenJarvis / SUNDAY upstream project: https://github.com/open-jarvis/OpenJarvis
- MiniCPM-o 4.5 GGUF model by OpenBMB: https://huggingface.co/openbmb/MiniCPM-o-4_5-gguf
- llama.cpp: https://github.com/ggml-org/llama.cpp
- llama.cpp-omni fork for MiniCPM-o omni experiments: https://github.com/tc-mb/llama.cpp-omni

Most of the original architecture, agent/tool abstractions, server structure, and documentation foundations come from the upstream OpenJarvis/SUNDAY project. This fork mainly renames, trims, patches, and wires the stack for my local SUNDAY setup.

## Local Setup

This repo expects local runtime artifacts to stay outside git:

- `llama-cpp/`
- `*.gguf`
- `*.exe`
- `*.dll`
- `omni-lab/models/`
- `omni-lab/llama.cpp-omni/`

Those files are ignored intentionally because model files and Windows binaries are too large for normal GitHub commits.

## Run The Local Assistant

From PowerShell:

```powershell
.\start_sunday_all.ps1
```

The script starts:

- `llama-server` on `127.0.0.1:8081`
- SUNDAY backend on `127.0.0.1:8000`
- frontend dev server on `127.0.0.1:5173`

The active local config is:

```text
configs/sunday/config.toml
```

## Current Local Model

The local setup is currently built around:

```text
MiniCPM-o-4_5-Q4_K_M.gguf
```

Text chat runs through the normal `llama-server` OpenAI-compatible endpoint. Full MiniCPM-o omni behavior, such as native audio/vision/TTS, requires separate runtime support and extra model modules.

## Omni Lab

MiniCPM-o omni testing is isolated under:

```text
omni-lab/
```

Prepare and test it separately:

```powershell
.\omni-lab\setup_omni_lab.ps1
.\omni-lab\build_omni.ps1
.\omni-lab\run_omni_text_only.ps1
```

To remove generated omni files:

```powershell
.\omni-lab\cleanup_omni_lab.ps1
```

This keeps the main assistant setup separate, so omni experiments can be deleted without affecting the normal local chat system.

## Speech

The frontend includes browser speech fallback:

- microphone input via browser speech recognition when available
- assistant message readout via browser `speechSynthesis`
- backend `/v1/speech/*` routes for future server-side STT integration

Server-side Whisper is optional and not required for the browser fallback.

## License

This fork keeps the upstream Apache 2.0 license. See [LICENSE](LICENSE).