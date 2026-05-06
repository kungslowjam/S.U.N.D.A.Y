# systemd Service (Linux)

SUNDAY includes a systemd unit file for running the API server as a managed background service on Linux. This provides automatic startup on boot, crash recovery, and integration with standard Linux service management tools.

## Prerequisites

Before installing the service, ensure that:

1. SUNDAY is installed in a virtual environment at `/opt/sunday/.venv` (or adjust paths accordingly).
2. A dedicated `sunday` system user exists (recommended for security).
3. An inference engine (such as Ollama) is running and accessible.

Create the user and installation directory:

```bash
sudo useradd --system --create-home --home-dir /opt/sunday sunday
sudo -u sunday python3 -m venv /opt/sunday/.venv
sudo -u sunday git clone https://github.com/open-sunday/SUNDAY.git /opt/sunday/SUNDAY
cd /opt/sunday/SUNDAY && sudo -u sunday uv sync --extra server
```

## Installing the Service

Copy the unit file to the systemd directory, reload the daemon, and enable the service:

```bash
sudo cp deploy/systemd/sunday.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable sunday
sudo systemctl start sunday
```

Verify it is running:

```bash
sudo systemctl status sunday
```

## Service File Reference

The provided unit file at `deploy/systemd/sunday.service`:

```ini
[Unit]
Description=SUNDAY API Server
After=network.target

[Service]
Type=simple
User=sunday
WorkingDirectory=/opt/sunday
ExecStart=/opt/sunday/.venv/bin/sunday serve --host 0.0.0.0 --port 8000
Restart=on-failure
RestartSec=5
Environment=HOME=/opt/sunday

[Install]
WantedBy=multi-user.target
```

### `[Unit]` Section

| Directive     | Value              | Description                                                                 |
|---------------|--------------------|-----------------------------------------------------------------------------|
| `Description` | `SUNDAY API Server` | Human-readable name shown in `systemctl status` and logs.              |
| `After`       | `network.target`   | Delays startup until the network stack is available, since the server binds to a network socket and may need to reach a remote engine. |

### `[Service]` Section

| Directive          | Value                                                              | Description                                                                                     |
|--------------------|--------------------------------------------------------------------|-------------------------------------------------------------------------------------------------|
| `Type`             | `simple`                                                           | The process started by `ExecStart` is the main service process. systemd considers the service started immediately. |
| `User`             | `sunday`                                                       | Runs the server as the `sunday` user rather than root, limiting the blast radius of any security issue. |
| `WorkingDirectory` | `/opt/sunday`                                                  | Sets the working directory for the process. This is where SUNDAY looks for local files and writes data. |
| `ExecStart`        | `/opt/sunday/.venv/bin/sunday serve --host 0.0.0.0 --port 8000` | The command to start the server. Uses the full path to the `sunday` binary inside the virtual environment. |
| `Restart`          | `on-failure`                                                       | Automatically restarts the service if it exits with a non-zero exit code. Does not restart on clean shutdown (`systemctl stop`). |
| `RestartSec`       | `5`                                                                | Waits 5 seconds before attempting a restart, preventing rapid restart loops if the service crashes immediately on startup. |
| `Environment`      | `HOME=/opt/sunday`                                             | Sets the `HOME` environment variable so SUNDAY finds its configuration at `~/.sunday/config.toml` (resolving to `/opt/sunday/.sunday/config.toml`). |

### `[Install]` Section

| Directive    | Value               | Description                                                                                 |
|--------------|---------------------|---------------------------------------------------------------------------------------------|
| `WantedBy`   | `multi-user.target` | The service starts when the system reaches multi-user mode (standard boot target for servers). `systemctl enable` creates a symlink under this target. |

## Configuration Options

### Changing the Bind Address and Port

Edit the `ExecStart` line to change the host or port:

```ini
ExecStart=/opt/sunday/.venv/bin/sunday serve --host 127.0.0.1 --port 9000
```

!!! tip
    Binding to `127.0.0.1` restricts access to localhost only. Use this when running behind a reverse proxy like Nginx or Caddy.

### Setting the Engine and Model

Pass additional flags to `sunday serve`:

```ini
ExecStart=/opt/sunday/.venv/bin/sunday serve --host 0.0.0.0 --port 8000 --engine ollama --model qwen3:8b
```

### Adding Environment Variables

Add multiple `Environment` directives or use `EnvironmentFile` for complex configurations:

```ini
[Service]
Environment=HOME=/opt/sunday
Environment=OPENSUNDAY_ENGINE_DEFAULT=vllm
Environment=OPENSUNDAY_OLLAMA_HOST=http://localhost:11434
```

Or load from a file:

```ini
[Service]
EnvironmentFile=/opt/sunday/.env
```

### Changing the User

If you prefer a different service user, update both the `User` directive and the paths:

```ini
[Service]
User=myuser
WorkingDirectory=/home/myuser/sunday
ExecStart=/home/myuser/sunday/.venv/bin/sunday serve --host 0.0.0.0 --port 8000
Environment=HOME=/home/myuser/sunday
```

### Using a Configuration File

Ensure the configuration file exists at the path where `HOME` points:

```bash
sudo -u sunday mkdir -p /opt/sunday/.sunday
sudo -u sunday cp config.toml /opt/sunday/.sunday/config.toml
```

The server reads `~/.sunday/config.toml` on startup, where `~` resolves from the `HOME` environment variable.

## Viewing Logs

SUNDAY logs are captured by journald. View them with `journalctl`:

```bash
# View all logs for the service
sudo journalctl -u sunday

# Follow logs in real time
sudo journalctl -u sunday -f

# View logs since the last boot
sudo journalctl -u sunday -b

# View logs from the last hour
sudo journalctl -u sunday --since "1 hour ago"

# View only error-level messages
sudo journalctl -u sunday -p err
```

## Managing the Service

### Start, Stop, and Restart

```bash
# Start the service
sudo systemctl start sunday

# Stop the service
sudo systemctl stop sunday

# Restart the service (stop + start)
sudo systemctl restart sunday

# Reload configuration without full restart (sends SIGHUP)
sudo systemctl reload-or-restart sunday
```

### Check Status

```bash
sudo systemctl status sunday
```

Example output:

```
● sunday.service - SUNDAY API Server
     Loaded: loaded (/etc/systemd/system/sunday.service; enabled; preset: enabled)
     Active: active (running) since Fri 2026-02-21 10:00:00 UTC; 2h ago
   Main PID: 12345 (sunday)
      Tasks: 4 (limit: 4915)
     Memory: 256.0M
        CPU: 1min 23s
     CGroup: /system.slice/sunday.service
             └─12345 /opt/sunday/.venv/bin/python /opt/sunday/.venv/bin/sunday serve --host 0.0.0.0 --port 8000
```

### Enable and Disable on Boot

```bash
# Enable automatic start on boot
sudo systemctl enable sunday

# Disable automatic start on boot
sudo systemctl disable sunday
```

### Apply Changes After Editing the Unit File

After modifying `/etc/systemd/system/sunday.service`, reload the systemd daemon and restart the service:

```bash
sudo systemctl daemon-reload
sudo systemctl restart sunday
```

## Running Alongside Ollama

If Ollama is also managed via systemd, you can add an ordering dependency so the SUNDAY service waits for Ollama to start:

```ini
[Unit]
Description=SUNDAY API Server
After=network.target ollama.service
Requires=ollama.service
```

| Directive  | Description                                                              |
|------------|--------------------------------------------------------------------------|
| `After`    | Ensures SUNDAY starts after Ollama.                                  |
| `Requires` | If Ollama fails to start, SUNDAY will not start either.              |

!!! note
    Use `Wants` instead of `Requires` if you want SUNDAY to start even when Ollama is unavailable (for example, if you plan to start Ollama manually later).
