---
title: Deployment
description: Deploy SUNDAY in production environments
---

# Deployment

SUNDAY supports multiple deployment strategies for different environments
and scales.

## Docker

The recommended way to deploy SUNDAY in production. Multi-stage builds
with CPU and GPU (NVIDIA CUDA, AMD ROCm) variants.

[:octicons-arrow-right-24: Docker deployment](docker.md)

## systemd (Linux)

Run SUNDAY as a managed system service on Linux servers.

[:octicons-arrow-right-24: systemd setup](systemd.md)

## launchd (macOS)

Register SUNDAY as a launch agent on macOS.

[:octicons-arrow-right-24: launchd setup](launchd.md)

## API Server

Run SUNDAY as an OpenAI-compatible HTTP server via `sunday serve`.

[:octicons-arrow-right-24: API server guide](api-server.md)
