# WSL2 Install

SUNDAY runs in WSL2 on Windows. Native Windows is not supported.

## One-time WSL setup

In an admin PowerShell:

```powershell
wsl --install
```

Then open the Ubuntu (or Debian) shell that gets installed.

## Install SUNDAY

```bash
curl -fsSL https://sunday.ai/install.sh | bash
```

About 3 minutes. Type `sunday` to start.

## WSL-specific notes

- The installer detects WSL via `/proc/sys/kernel/osrelease` and uses `nohup ollama serve &` instead of systemd to start the Ollama daemon (WSL2 doesn't ship systemd by default).
- The first time you run `sunday`, the WSL kernel may show a "process running in background" notification — that's the bg-orchestrator detaching. It's expected.
- Models are stored in WSL's filesystem (`~/.sunday/`), not your Windows drive. To free up space later: `sunday-uninstall` removes everything.

## See also

- [Full installer reference](install.md)
