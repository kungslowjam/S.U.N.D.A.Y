# Start a small, separate llama-server for low-latency Voice Live.

$ErrorActionPreference = "Stop"

$ProjectRoot = "C:\Users\hello\Desktop\Project_me\SUNDAY"
$LlamaCppPath = "$ProjectRoot\llama-cpp"
$ModelPath = if ($env:SUNDAY_VOICE_MODEL_PATH) { $env:SUNDAY_VOICE_MODEL_PATH } else { ".\models\Qwen3.5-0.8B-Q4_K_M.gguf" }
$Port = if ($env:SUNDAY_VOICE_LLM_PORT) { [int]$env:SUNDAY_VOICE_LLM_PORT } else { 8082 }
$GpuLayers = if ($env:SUNDAY_VOICE_GPU_LAYERS) { [int]$env:SUNDAY_VOICE_GPU_LAYERS } else { 99 }
$ContextSize = if ($env:SUNDAY_VOICE_CONTEXT_SIZE) { [int]$env:SUNDAY_VOICE_CONTEXT_SIZE } else { 2048 }
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Hidden" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Hidden"
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
        $code = & curl.exe -s -o NUL -w "%{http_code}" --max-time $TimeoutSec $Url
        return $code -eq "200"
    } catch {
        return $false
    }
}

function Wait-ForHttp {
    param([string]$Url, [int]$TimeoutSec = 90)
    $elapsed = 0
    while (-not (Test-Http $Url 2)) {
        Start-Sleep -Seconds 1
        $elapsed++
        if ($elapsed -ge $TimeoutSec) {
            throw "Timeout waiting for $Url"
        }
        Write-Host "." -NoNewline
    }
    Write-Host ""
}

if (-not (Test-Path (Join-Path $LlamaCppPath $ModelPath))) {
    throw "Voice model not found: $LlamaCppPath\$ModelPath"
}

if (Test-Http "http://127.0.0.1:$Port/v1/models" 2) {
    Write-Host "[VOICE LLM] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} elseif (Test-Port $Port) {
    Write-Host "[VOICE LLM] Port $Port is busy. Waiting for server..." -ForegroundColor Yellow
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 90
} else {
    Write-Host "[VOICE LLM] Starting Qwen voice model on http://127.0.0.1:$Port" -ForegroundColor Cyan
    $Args = @(
        ".\llama-server.exe",
        "-m", "'$ModelPath'",
        "--port", "$Port",
        "-ngl", "$GpuLayers",
        "-c", "$ContextSize",
        "-np", "1",
        "--cache-ram", "0",
        "--no-warmup"
    ) -join " "
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", "cd '$LlamaCppPath'; $Args" `
        -WorkingDirectory $LlamaCppPath `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 90
}

Write-Host "[VOICE LLM] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
