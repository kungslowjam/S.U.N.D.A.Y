# Starts Atomic TurboQuant with llama.cpp-style draft/speculative model.
# This uses --model-draft with the HackAfterDark ultralight drafter, not --mtp-head.

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

$Port = if ($env:SUNDAY_MTP_PORT) { [int]$env:SUNDAY_MTP_PORT } else { 8084 }
$TargetModel = if ($env:SUNDAY_MTP_TARGET_MODEL) {
    $env:SUNDAY_MTP_TARGET_MODEL
} else {
    Join-Path $ModelsPath "gemma-4-E4B-it-Q4_K_M.gguf"
}
$DraftModel = if ($env:SUNDAY_MTP_DRAFT_MODEL) {
    $env:SUNDAY_MTP_DRAFT_MODEL
} else {
    Join-Path $ModelsPath "gemma-4-e4b-it-mtp-assistant-ultralight.f16.gguf"
}
$ContextSize = if ($env:SUNDAY_MTP_CONTEXT_SIZE) { [int]$env:SUNDAY_MTP_CONTEXT_SIZE } else { 4096 }
$GpuLayers = if ($env:SUNDAY_MTP_GPU_LAYERS) { [int]$env:SUNDAY_MTP_GPU_LAYERS } else { 43 }
$DraftGpuLayers = if ($env:SUNDAY_MTP_DRAFT_GPU_LAYERS) { [int]$env:SUNDAY_MTP_DRAFT_GPU_LAYERS } else { 99 }
$DraftMax = if ($env:SUNDAY_MTP_DRAFT_MAX) { [int]$env:SUNDAY_MTP_DRAFT_MAX } else { 8 }
$BatchSize = if ($env:SUNDAY_MTP_BATCH_SIZE) { [int]$env:SUNDAY_MTP_BATCH_SIZE } else { 128 }
$UBatchSize = if ($env:SUNDAY_MTP_UBATCH_SIZE) { [int]$env:SUNDAY_MTP_UBATCH_SIZE } else { 128 }
$Threads = if ($env:SUNDAY_MTP_THREADS) { [int]$env:SUNDAY_MTP_THREADS } else { 6 }
$CacheType = if ($env:SUNDAY_MTP_CACHE_TYPE) { $env:SUNDAY_MTP_CACHE_TYPE } else { "turbo3" }
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
if (-not (Test-Path $TargetModel)) {
    throw "Target model not found: $TargetModel"
}
if (-not (Test-Path $DraftModel)) {
    throw "Draft model not found: $DraftModel"
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
    Write-Host "[ATOMIC+DRAFT] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} else {
    Write-Host "[ATOMIC+DRAFT] Starting Atomic TurboQuant + draft on http://127.0.0.1:$Port" -ForegroundColor Cyan
    Write-Host "               Target: $TargetModel"
    Write-Host "               Draft : $DraftModel"
    Write-Host "               Params: ctx=$ContextSize, cache=$CacheType, draft=$DraftMax, batch=$BatchSize/$UBatchSize"
    $Args = @(
        "`"$ServerPath`"",
        "-m", "`"$TargetModel`"",
        "--model-draft", "`"$DraftModel`"",
        "--spec-type", "draft",
        "--draft-max", "$DraftMax",
        "--draft-min", "0",
        "--port", "$Port",
        "--host", "127.0.0.1",
        "-ngl", "$GpuLayers",
        "-ngld", "$DraftGpuLayers",
        "-c", "$ContextSize",
        "-np", "1",
        "-b", "$BatchSize",
        "-ub", "$UBatchSize",
        "-t", "$Threads",
        "-tb", "$Threads",
        "--flash-attn", "on",
        "--swa-full",
        "--kv-unified",
        "-ctk", "$CacheType",
        "-ctv", "$CacheType",
        "-ctkd", "$CacheType",
        "-ctvd", "$CacheType",
        "--cache-ram", "0",
        "--no-warmup",
        "--jinja"
    ) -join " "
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $Args `
        -WorkingDirectory (Split-Path $ServerPath -Parent) `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 120
}

Write-Host "[ATOMIC+DRAFT] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
