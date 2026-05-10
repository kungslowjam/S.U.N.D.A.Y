# Start a small, separate llama-server for low-latency Voice Live.

$ErrorActionPreference = "Stop"

$ProjectRoot = "C:\Users\hello\Desktop\Project_me\SUNDAY"
$LlamaCppPath = "$ProjectRoot\llama-cpp"

# 1. Engine Detection (Atomic vs Standard)
$AtomicBinPath = "$ProjectRoot\mtp-lab\build-ninja\bin"
$AtomicServer = Join-Path $AtomicBinPath "llama-server.exe"
$UseAtomic = if ($env:SUNDAY_VOICE_USE_ATOMIC -eq "0") { $false } else { Test-Path $AtomicServer }

$ServerPath = if ($UseAtomic) { $AtomicServer } else { Join-Path $LlamaCppPath "llama-server.exe" }

# 2. Model Configuration
$DefaultModel = ".\models\Qwen3.5-0.8B-Q4_K_M.gguf"
$ModelPath = if ($env:SUNDAY_VOICE_MODEL_PATH) { $env:SUNDAY_VOICE_MODEL_PATH } else { $DefaultModel }
$Port = if ($env:SUNDAY_VOICE_LLM_PORT) { [int]$env:SUNDAY_VOICE_LLM_PORT } else { 8082 }
$GpuLayers = if ($env:SUNDAY_VOICE_GPU_LAYERS) { [int]$env:SUNDAY_VOICE_GPU_LAYERS } else { 99 }
$ContextSize = if ($env:SUNDAY_VOICE_CONTEXT_SIZE) { [int]$env:SUNDAY_VOICE_CONTEXT_SIZE } else { 1024 }

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

function Invoke-VoiceWarmup {
    param([int]$Port)
    $body = @{
        model = "local-model"
        messages = @(
            @{ role = "system"; content = "You are a fast voice assistant. Reply with one word." },
            @{ role = "user"; content = "warmup" }
        )
        stream = $false
        max_tokens = 1
        temperature = 0
        chat_template_kwargs = @{ enable_thinking = $false }
    } | ConvertTo-Json -Depth 8 -Compress
    try {
        Invoke-WebRequest `
            -Uri "http://127.0.0.1:$Port/v1/chat/completions" `
            -Method Post `
            -ContentType "application/json" `
            -Body $body `
            -UseBasicParsing `
            -TimeoutSec 20 | Out-Null
        Write-Host "[VOICE LLM] Warmup complete." -ForegroundColor DarkGreen
    } catch {
        Write-Host "[VOICE LLM] Warmup skipped: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

$AbsModelPath = if ([System.IO.Path]::IsPathRooted($ModelPath)) { $ModelPath } else { Join-Path $LlamaCppPath $ModelPath }
if (-not (Test-Path $AbsModelPath)) {
    throw "Voice model not found: $AbsModelPath"
}

if (Test-Http "http://127.0.0.1:$Port/v1/models" 2) {
    Write-Host "[VOICE LLM] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} elseif (Test-Port $Port) {
    Write-Host "[VOICE LLM] Port $Port is busy. Waiting for server..." -ForegroundColor Yellow
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 90
} else {
    $EngineName = if ($UseAtomic) { "Atomic TurboQuant" } else { "Standard llama-cpp" }
    Write-Host "[VOICE LLM] Starting $EngineName on http://127.0.0.1:$Port" -ForegroundColor Cyan
    Write-Host "            Model: $ModelPath"
    
    if ($UseAtomic) {
        $Args = @(
            "`"$ServerPath`"",
            "-m", "`"$AbsModelPath`"",
            "--port", "$Port",
            "-ngl", "$GpuLayers",
            "-c", "$ContextSize",
            "-np", "1",
            "-ctk", "turbo3",
            "-ctv", "turbo3",
            "-fa", "off",
            "--no-warmup"
        )
    } else {
        $Args = @(
            "`"$ServerPath`"",
            "-m", "`"$AbsModelPath`"",
            "--port", "$Port",
            "-ngl", "$GpuLayers",
            "-c", "$ContextSize",
            "-np", "1",
            "--cache-type-k", "q4_0",
            "--cache-type-v", "q4_0",
            "--no-warmup"
        )
    }

    $FullArgs = $Args -join " "
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $FullArgs `
        -WorkingDirectory (Split-Path $ServerPath -Parent) `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 90
}

Invoke-VoiceWarmup -Port $Port
Write-Host "[VOICE LLM] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
