# SUNDAY All-in-One Runner
# Runs Llama-Server (GPU) + SUNDAY Backend + Frontend

$ProjectRoot = "C:\Users\hello\Desktop\Project_me\SUNDAY"
$LlamaCppPath = "$ProjectRoot\llama-cpp"
$ModelPath = ".\models\MiniCPM-o-4_5-Q4_K_M.gguf"
$GpuLayers = 99
$LlamaPort = 8081
$BackendPort = 8000
$FrontendPort = 5173
$ConfigPath = "$ProjectRoot\configs\sunday\config.toml"
$ModelName = "MiniCPM-o-4_5-Q4_K_M.gguf"

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
    foreach ($pid in $connections) {
        try {
            $proc = Get-Process -Id $pid -ErrorAction Stop
            Write-Host "       stopping $($proc.ProcessName) ($pid)" -ForegroundColor DarkYellow
            Stop-Process -Id $pid -Force
        } catch {
            Write-Host "       could not stop process $pid" -ForegroundColor DarkYellow
        }
    }

    Start-Sleep -Seconds 1
}

function Write-Elapsed {
    param([datetime]$StartTime)
    $elapsed = ((Get-Date) - $StartTime).TotalSeconds
    Write-Host ("       took {0:N1}s" -f $elapsed) -ForegroundColor DarkGray
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   SUNDAY AUTOMATIC STARTUP SYSTEM" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

Clear-Port $LlamaPort "AI Engine"
Clear-Port $BackendPort "SUNDAY Backend"
Clear-Port $FrontendPort "Frontend Dashboard"

# 1. Start Llama-Server (AI Core)
Write-Host "[1/3] Starting AI Engine (llama-server) on port $LlamaPort..." -ForegroundColor Cyan
$StepStart = Get-Date
if (Test-Http "http://localhost:$LlamaPort/v1/models" 2) {
    Write-Host "[SKIP] AI Engine is already running." -ForegroundColor Yellow
} elseif (Test-Port $LlamaPort) {
    Write-Host "[WAIT] AI Engine process is already starting." -ForegroundColor Yellow
    Wait-ForHttp "http://localhost:$LlamaPort/v1/models" 180
} else {
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "cd '$LlamaCppPath'; .\llama-server.exe -m '$ModelPath' --port $LlamaPort -ngl $GpuLayers -c 4096" -WorkingDirectory $LlamaCppPath
    Wait-ForHttp "http://localhost:$LlamaPort/v1/models" 180
}
Write-Elapsed $StepStart
Write-Host "[OK] AI Engine is ready." -ForegroundColor Green

# 2. Start SUNDAY Backend
Write-Host "[2/3] Starting SUNDAY Backend on port $BackendPort..." -ForegroundColor Cyan
$StepStart = Get-Date
$SundayExe = "$ProjectRoot\.venv\Scripts\sunday.exe"
if (Test-Http "http://localhost:$BackendPort/v1/models" 2) {
    Write-Host "[SKIP] SUNDAY Backend is already running." -ForegroundColor Yellow
} else {
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "`$env:OPENSUNDAY_CONFIG='$ConfigPath'; & '$SundayExe' serve --engine llamacpp --model '$ModelName' --host 127.0.0.1 --port $BackendPort" -WorkingDirectory $ProjectRoot
    Wait-ForHttp "http://localhost:$BackendPort/v1/models" 90
}
Write-Elapsed $StepStart
Write-Host "[OK] SUNDAY Backend is ready." -ForegroundColor Green

# 3. Start Frontend
Write-Host "[3/3] Starting Frontend Dashboard on port $FrontendPort..." -ForegroundColor Cyan
$StepStart = Get-Date
if (Test-Http "http://localhost:$FrontendPort" 2) {
    Write-Host "[SKIP] Frontend Dashboard is already running." -ForegroundColor Yellow
} else {
    Start-Process powershell -ArgumentList "-NoExit", "-Command", "npm run dev" -WorkingDirectory "$ProjectRoot\frontend"
    Wait-ForHttp "http://localhost:$FrontendPort" 120
}
Write-Elapsed $StepStart
Write-Host "[OK] Frontend is ready." -ForegroundColor Green

# 4. Open Browser
Write-Host "Opening Dashboard..." -ForegroundColor Cyan
Start-Process "http://localhost:$FrontendPort"

Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "       ALL SYSTEMS ARE GO! 🚀" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host " AI Engine  : http://localhost:$LlamaPort"
Write-Host " Backend API: http://localhost:$BackendPort"
Write-Host " Dashboard  : http://localhost:$FrontendPort"
Write-Host "========================================"
