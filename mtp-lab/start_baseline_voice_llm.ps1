# Starts a baseline llama-server with no TurboQuant and no draft model.
# Use this for fair latency comparison against Atomic TurboQuant and draft/speculative.

$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LlamaCppPath = Join-Path $ProjectRoot "llama-cpp"
$ModelsPath = Join-Path $LlamaCppPath "models"

$Port = if ($env:SUNDAY_BASELINE_PORT) { [int]$env:SUNDAY_BASELINE_PORT } else { 8084 }
$TargetModel = if ($env:SUNDAY_BASELINE_MODEL) {
    $env:SUNDAY_BASELINE_MODEL
} else {
    $TargetCandidates = @(
        (Join-Path $ModelsPath "gemma-4-E4B-it-Q4_K_M.gguf"),
        (Join-Path $ModelsPath "gemma-4-E4B-it-ultra-uncensored-heretic-Q4_K_M.gguf")
    )
    $TargetCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$ContextSize = if ($env:SUNDAY_BASELINE_CONTEXT_SIZE) { [int]$env:SUNDAY_BASELINE_CONTEXT_SIZE } else { 4096 }
$GpuLayers = if ($env:SUNDAY_BASELINE_GPU_LAYERS) { [int]$env:SUNDAY_BASELINE_GPU_LAYERS } else { 43 }
$BatchSize = if ($env:SUNDAY_BASELINE_BATCH_SIZE) { [int]$env:SUNDAY_BASELINE_BATCH_SIZE } else { 128 }
$UBatchSize = if ($env:SUNDAY_BASELINE_UBATCH_SIZE) { [int]$env:SUNDAY_BASELINE_UBATCH_SIZE } else { 128 }
$Threads = if ($env:SUNDAY_BASELINE_THREADS) { [int]$env:SUNDAY_BASELINE_THREADS } else { 6 }
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Minimized" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Minimized"
}

if (-not (Test-Path (Join-Path $LlamaCppPath "llama-server.exe"))) {
    throw "llama-server.exe not found in $LlamaCppPath"
}
if (-not $TargetModel -or -not (Test-Path $TargetModel)) {
    throw "Target model not found. Put gemma-4-E4B-it-Q4_K_M.gguf in llama-cpp\models or set SUNDAY_BASELINE_MODEL."
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
    Write-Host "[BASELINE] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} else {
    Write-Host "[BASELINE] Starting llama.cpp without TurboQuant/draft on http://127.0.0.1:$Port" -ForegroundColor Cyan
    Write-Host "           Target: $TargetModel"
    Write-Host "           Params: ctx=$ContextSize, ngl=$GpuLayers, batch=$BatchSize/$UBatchSize"
    $Args = @(
        ".\llama-server.exe",
        "-m", "`"$TargetModel`"",
        "--port", "$Port",
        "--host", "127.0.0.1",
        "-ngl", "$GpuLayers",
        "-c", "$ContextSize",
        "-np", "1",
        "-b", "$BatchSize",
        "-ub", "$UBatchSize",
        "-t", "$Threads",
        "-tb", "$Threads",
        "--flash-attn", "on",
        "--swa-full",
        "--cache-ram", "0",
        "--no-warmup",
        "--jinja"
    ) -join " "
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", "cd '$LlamaCppPath'; $Args" `
        -WorkingDirectory $LlamaCppPath `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 120
}

Write-Host "[BASELINE] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
