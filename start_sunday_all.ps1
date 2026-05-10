# SUNDAY All-in-One Runner
# Runs Llama-Server (GPU) + SUNDAY Backend + Frontend

$ProjectRoot = "C:\Users\hello\Desktop\Project_me\SUNDAY"
$LlamaCppPath = "$ProjectRoot\llama-cpp"
$DefaultHfModel = "llmfan46/gemma-4-E2B-it-ultra-uncensored-heretic-GGUF:Q4_K_M"
$DefaultHfFile = "gemma-4-E2B-it-ultra-uncensored-heretic-Q4_K_M.gguf"
$DefaultLocalGemma = ".\models\Qwen3.5-4B-Q4_K_M.gguf"
$DefaultFallbackModel = ".\models\MiniCPM-o-4_5-Q4_K_M.gguf"
$ModelSource = if ($env:SUNDAY_MODEL_SOURCE) { $env:SUNDAY_MODEL_SOURCE } elseif (Test-Path (Join-Path $LlamaCppPath $DefaultLocalGemma)) { "local" } else { "hf" }
$ModelPath = if ($env:SUNDAY_MODEL_PATH) { $env:SUNDAY_MODEL_PATH } elseif (Test-Path (Join-Path $LlamaCppPath $DefaultLocalGemma)) { $DefaultLocalGemma } else { $DefaultFallbackModel }
$HfModel = if ($env:SUNDAY_HF_MODEL) { $env:SUNDAY_HF_MODEL } else { $DefaultHfModel }
$HfFile = if ($env:SUNDAY_HF_FILE) { $env:SUNDAY_HF_FILE } else { $DefaultHfFile }
$GpuLayers = if ($env:SUNDAY_GPU_LAYERS) { [int]$env:SUNDAY_GPU_LAYERS } else { 35 }
$ContextSize = if ($env:SUNDAY_CONTEXT_SIZE) { [int]$env:SUNDAY_CONTEXT_SIZE } else { 8192 }
$ParallelSlots = 1
$LlamaPort = 8081
$VoiceLlamaPort = 8082
$BackendPort = 8000
$FrontendPort = 5173
$VoiceLivePort = 8098
$ConfigPath = "$ProjectRoot\configs\sunday\config.toml"
$ModelName = if ($ModelSource -eq "hf") { $HfModel } else { Split-Path $ModelPath -Leaf }
$StartVoiceLive = if ($env:SUNDAY_VOICE_LIVE) { $env:SUNDAY_VOICE_LIVE -ne "0" } else { $true }
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Hidden" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Hidden"
}

# Function to check if port is in use
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

function Wait-ForPort {
    param([int]$Port, [int]$TimeoutSec = 60)
    $elapsed = 0
    while (-not (Test-Port $Port)) {
        Start-Sleep -Seconds 1
        $elapsed++
        if ($elapsed -ge $TimeoutSec) {
            throw "Timeout waiting for port $Port"
        }
        Write-Host "." -NoNewline
    }
    Write-Host ""
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
    param([string]$Url, [int]$TimeoutSec = 60)
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

function Clear-Port {
    param([int]$Port, [string]$Name)
    $connections = Get-NetTCPConnection -LocalPort $Port -ErrorAction SilentlyContinue |
        Where-Object { $_.OwningProcess -ne 0 } |
        Select-Object -ExpandProperty OwningProcess -Unique

    if (-not $connections) {
        return
    }

    Write-Host "[CLEAN] Clearing $Name on port $Port..." -ForegroundColor Yellow
    foreach ($processId in $connections) {
        try {
            $proc = Get-Process -Id $processId -ErrorAction Stop
            Write-Host "       stopping $($proc.ProcessName) ($processId)" -ForegroundColor DarkYellow
            Stop-Process -Id $processId -Force
        } catch {
            Write-Host "       could not stop process $processId" -ForegroundColor DarkYellow
        }
    }

    Start-Sleep -Seconds 1
}

function Write-Elapsed {
    param([datetime]$StartTime)
    $elapsed = ((Get-Date) - $StartTime).TotalSeconds
    Write-Host ("       took {0:N1}s" -f $elapsed) -ForegroundColor DarkGray
}

function Start-ServiceProcess {
    param(
        [string]$Command,
        [string]$WorkingDirectory
    )
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $Command `
        -WorkingDirectory $WorkingDirectory `
        -WindowStyle $ConsoleWindowStyle
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   SUNDAY AUTOMATIC STARTUP SYSTEM" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Model      : $ModelName" -ForegroundColor DarkGray
Write-Host " Model path : $ModelPath" -ForegroundColor DarkGray
Write-Host " Consoles   : $ConsoleWindowStyle (set SUNDAY_CONSOLE_STYLE=Normal to show logs)" -ForegroundColor DarkGray

Clear-Port $LlamaPort "AI Engine"
Clear-Port $BackendPort "SUNDAY Backend"
Clear-Port $FrontendPort "Frontend Dashboard"
if ($StartVoiceLive) {
    Clear-Port $VoiceLlamaPort "Voice LLM"
    Clear-Port $VoiceLivePort "Voice Live Overlay"
}

# 1. Start Llama-Server (AI Core)
Write-Host "[1/4] Starting AI Engine (llama-server) on port $LlamaPort..." -ForegroundColor Cyan
$StepStart = Get-Date
if (Test-Http "http://localhost:$LlamaPort/v1/models" 2) {
    Write-Host "[SKIP] AI Engine is already running." -ForegroundColor Yellow
} elseif (Test-Port $LlamaPort) {
    Write-Host "[WAIT] AI Engine process is already starting." -ForegroundColor Yellow
    Wait-ForHttp "http://localhost:$LlamaPort/v1/models" 180
} else {
    $ModelArgs = if ($ModelSource -eq "hf") {
        @("-hf", $HfModel, "-hff", $HfFile)
    } else {
        @("-m", "'$ModelPath'")
    }
    $LlamaArgs = @(".\llama-server.exe") + $ModelArgs + @(
        "--port", "$LlamaPort",
        "-ngl", "$GpuLayers",
        "-c", "$ContextSize",
        "-t", "4",
        "--parallel", "$ParallelSlots",
        "--cache-ram", "0",
        "--no-warmup"
    )
    $LlamaCommand = $LlamaArgs -join " "
    Start-ServiceProcess -Command "cd '$LlamaCppPath'; $LlamaCommand" -WorkingDirectory $LlamaCppPath
    Wait-ForHttp "http://localhost:$LlamaPort/v1/models" 180
}
Write-Elapsed $StepStart
Write-Host "[OK] AI Engine is ready." -ForegroundColor Green

# 2. Start SUNDAY Backend
Write-Host "[2/4] Starting SUNDAY Backend on port $BackendPort..." -ForegroundColor Cyan
$StepStart = Get-Date
$SundayExe = "$ProjectRoot\.venv\Scripts\sunday.exe"
if (Test-Http "http://localhost:$BackendPort/v1/models" 2) {
    Write-Host "[SKIP] SUNDAY Backend is already running." -ForegroundColor Yellow
} else {
    Start-ServiceProcess -Command "`$env:OPENSUNDAY_CONFIG='$ConfigPath'; & '$SundayExe' serve --engine llamacpp --model '$ModelName' --host 127.0.0.1 --port $BackendPort" -WorkingDirectory $ProjectRoot
    Wait-ForHttp "http://localhost:$BackendPort/v1/models" 90
}
Write-Elapsed $StepStart
Write-Host "[OK] SUNDAY Backend is ready." -ForegroundColor Green

# 3. Start Frontend
Write-Host "[3/4] Starting Frontend Dashboard on port $FrontendPort..." -ForegroundColor Cyan
$StepStart = Get-Date
if (Test-Http "http://localhost:$FrontendPort" 2) {
    Write-Host "[SKIP] Frontend Dashboard is already running." -ForegroundColor Yellow
} else {
    Start-ServiceProcess -Command "npm run dev" -WorkingDirectory "$ProjectRoot\frontend"
    Wait-ForHttp "http://localhost:$FrontendPort" 120
}
Write-Elapsed $StepStart
Write-Host "[OK] Frontend is ready." -ForegroundColor Green

# 4. Start Voice Live Overlay (optional sidecar)
if ($StartVoiceLive) {
    Write-Host "[4/4] Starting Voice Live Overlay on port $VoiceLivePort..." -ForegroundColor Cyan
    $StepStart = Get-Date
    & "$ProjectRoot\voice-live\start_voice_live.ps1"
    Write-Elapsed $StepStart
    Write-Host "[OK] Voice Live is ready." -ForegroundColor Green
} else {
    Write-Host "[4/4] Voice Live disabled (SUNDAY_VOICE_LIVE=0)." -ForegroundColor Yellow
}

# 5. Open Browser
Write-Host "Opening Dashboard..." -ForegroundColor Cyan
Start-Process "http://localhost:$FrontendPort"

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "       ALL SYSTEMS ARE GO! 🚀" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host " AI Engine  : http://localhost:$LlamaPort"
Write-Host " Backend API: http://localhost:$BackendPort"
Write-Host " Dashboard  : http://localhost:$FrontendPort"
if ($StartVoiceLive) {
    Write-Host " Voice Live : http://localhost:$VoiceLivePort"
}
Write-Host "========================================"
