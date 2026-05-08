# Starts Atomic TurboQuant llama-server without MTP.
# Use this when --spec-type mtp crashes on Windows/CUDA, but you still want
# to test the Atomic fork and TurboQuant KV cache separately.

$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$BuildCandidates = @(
    (Join-Path $PSScriptRoot "build-ninja"),
    (Join-Path $PSScriptRoot "build")
)
$BuildPath = if ($env:SUNDAY_MTP_BUILD_DIR) {
    $env:SUNDAY_MTP_BUILD_DIR
} else {
    $BuildCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$ModelsPath = Join-Path $ProjectRoot "llama-cpp\models"

$Port = if ($env:SUNDAY_ATOMIC_PORT) { [int]$env:SUNDAY_ATOMIC_PORT } else { 8084 }
$TargetModel = if ($env:SUNDAY_ATOMIC_MODEL) {
    $env:SUNDAY_ATOMIC_MODEL
} else {
    $TargetCandidates = @(
        (Join-Path $ModelsPath "gemma-4-E4B-it-Q4_K_M.gguf"),
        (Join-Path $ModelsPath "gemma-4-E4B-it-ultra-uncensored-heretic-Q4_K_M.gguf")
    )
    $TargetCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$ContextSize = if ($env:SUNDAY_ATOMIC_CONTEXT_SIZE) { [int]$env:SUNDAY_ATOMIC_CONTEXT_SIZE } else { 1024 }
$GpuLayers = if ($env:SUNDAY_ATOMIC_GPU_LAYERS) { [int]$env:SUNDAY_ATOMIC_GPU_LAYERS } else { 99 }
$CacheType = if ($env:SUNDAY_ATOMIC_CACHE_TYPE) { $env:SUNDAY_ATOMIC_CACHE_TYPE } else { "turbo3" }
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Minimized" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Minimized"
}

$ServerCandidates = @(
    (Join-Path $BuildPath "bin\Release\llama-server.exe"),
    (Join-Path $BuildPath "bin\llama-server.exe")
)
$ServerPath = $ServerCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $ServerPath) {
    throw "Atomic llama-server.exe not found. Run .\mtp-lab\setup_atomic_mtp.ps1 first."
}
if (-not $TargetModel -or -not (Test-Path $TargetModel)) {
    throw "Target model not found. Put gemma-4-E4B-it-Q4_K_M.gguf in llama-cpp\models or set SUNDAY_ATOMIC_MODEL."
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
    param([string]$Url, [int]$TimeoutSec = 120)
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

if (Test-Http "http://127.0.0.1:$Port/v1/models" 2) {
    Write-Host "[ATOMIC] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} else {
    Write-Host "[ATOMIC] Starting Atomic TurboQuant without MTP on http://127.0.0.1:$Port" -ForegroundColor Cyan
    Write-Host "         Target: $TargetModel"
    Write-Host "         Cache : $CacheType"
    $Args = @(
        "`"$ServerPath`"",
        "-m", "`"$TargetModel`"",
        "--port", "$Port",
        "--host", "127.0.0.1",
        "-ngl", "$GpuLayers",
        "-c", "$ContextSize",
        "-np", "1",
        "-fa", "on",
        "--swa-full",
        "-ctk", "$CacheType",
        "-ctv", "$CacheType",
        "--cache-ram", "0",
        "--no-warmup"
    ) -join " "
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $Args `
        -WorkingDirectory (Split-Path $ServerPath -Parent) `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 120
}

Write-Host "[ATOMIC] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
