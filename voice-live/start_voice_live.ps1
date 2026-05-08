# Start the browser-based Gemini Live-style voice overlay.

$ErrorActionPreference = "Stop"

$Root = $PSScriptRoot
$Port = if ($env:SUNDAY_VOICE_LIVE_PORT) { [int]$env:SUNDAY_VOICE_LIVE_PORT } else { 8098 }
$PreloadStt = if ($env:SUNDAY_STT_PRELOAD) { $env:SUNDAY_STT_PRELOAD } else { "base" }
$StartVoiceLlm = if ($env:SUNDAY_START_VOICE_LLM) { $env:SUNDAY_START_VOICE_LLM -ne "0" } else { $true }
$OpenBrowser = if ($env:SUNDAY_OPEN_VOICE_BROWSER) { $env:SUNDAY_OPEN_VOICE_BROWSER -ne "0" } else { $true }
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Hidden" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Hidden"
}
$Python = if (Get-Command py -ErrorAction SilentlyContinue) { "py" } elseif (Get-Command python -ErrorAction SilentlyContinue) { "python" } else { $null }

if (-not $Python) {
    throw "Python launcher not found. Install Python or make sure 'py' is available in PATH."
}

& $Python -c "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('edge_tts') else 1)"
if ($LASTEXITCODE -ne 0) {
    Write-Host "[VOICE] Installing edge-tts for smoother voice output..." -ForegroundColor Cyan
    & $Python -m pip install --user edge-tts
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to install edge-tts. Try manually: $Python -m pip install --user edge-tts"
    }
}

& $Python -c "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('faster_whisper') else 1)"
if ($LASTEXITCODE -ne 0) {
    Write-Host "[VOICE] Installing faster-whisper for local STT..." -ForegroundColor Cyan
    & $Python -m pip install --user faster-whisper
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to install faster-whisper. Try manually: $Python -m pip install --user faster-whisper"
    }
}

if ($StartVoiceLlm) {
    & (Join-Path $Root "start_voice_llm.ps1")
}

function Test-Port {
    param([int]$Port)
    $tcp = New-Object System.Net.Sockets.TcpClient
    try {
        $tcp.Connect("localhost", $Port)
        $tcp.Close()
        return $true
    } catch {
        return $false
    }
}

function Test-Http {
    param([string]$Url, [int]$TimeoutSec = 2)
    try {
        $response = Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec $TimeoutSec
        return $response.StatusCode -eq 200
    } catch {
        return $false
    }
}

function Wait-ForHttp {
    param([string]$Url, [int]$TimeoutSec = 10)
    $elapsed = 0
    while (-not (Test-Http $Url 2)) {
        Start-Sleep -Seconds 1
        $elapsed++
        if ($elapsed -ge $TimeoutSec) {
            throw "Timeout waiting for $Url. Try running: $Python `"$Root\server.py`" --port $Port --preload-stt `"$PreloadStt`""
        }
        Write-Host "." -NoNewline
    }
    Write-Host ""
}

if (-not (Test-Port $Port)) {
    Write-Host "[VOICE] Starting overlay server on http://127.0.0.1:$Port" -ForegroundColor Cyan
    $Command = "$Python `"$Root\server.py`" --port $Port --preload-stt `"$PreloadStt`""
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $Command `
        -WorkingDirectory $Root `
        -WindowStyle $ConsoleWindowStyle
} else {
    Write-Host "[VOICE] Overlay server is already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
}

$Url = "http://127.0.0.1:$Port"
Wait-ForHttp $Url 10
if ($OpenBrowser) {
    Write-Host "[VOICE] Opening $Url" -ForegroundColor Green
    Start-Process $Url
} else {
    Write-Host "[VOICE] Ready: $Url" -ForegroundColor Green
}
