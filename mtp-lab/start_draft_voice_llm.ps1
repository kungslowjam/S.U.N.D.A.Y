# Starts an isolated draft/speculative server using the existing llama-cpp build.
# This is the safer path for HackAfterDark Gemma 4 E4B MTP assistant files.

$ErrorActionPreference = "Stop"

$ProjectRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$LlamaCppPath = Join-Path $ProjectRoot "llama-cpp"
$LlamaServer = Join-Path $LlamaCppPath "llama-server.exe"
$ModelsPath = Join-Path $LlamaCppPath "models"

$Port = if ($env:SUNDAY_MTP_PORT) { [int]$env:SUNDAY_MTP_PORT } else { 8084 }
$TargetModel = if ($env:SUNDAY_MTP_TARGET_MODEL) {
    $env:SUNDAY_MTP_TARGET_MODEL
} else {
    $TargetCandidates = @(
        (Join-Path $ModelsPath "gemma-4-E4B-it-Q4_K_M.gguf"),
        (Join-Path $ModelsPath "gemma-4-E4B-it-ultra-uncensored-heretic-Q4_K_M.gguf")
    )
    $TargetCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
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
$CacheType = if ($env:SUNDAY_MTP_CACHE_TYPE) { $env:SUNDAY_MTP_CACHE_TYPE } else { "q4_0" }
$UseMlock = if ($env:SUNDAY_MTP_MLOCK) { $env:SUNDAY_MTP_MLOCK -ne "0" } else { $false }
$UseNuma = if ($env:SUNDAY_MTP_NUMA) { $env:SUNDAY_MTP_NUMA } else { "" }
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Minimized" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Minimized"
}

if (-not (Test-Path $LlamaServer)) {
    throw "llama-server.exe not found: $LlamaServer"
}
if (-not $TargetModel -or -not (Test-Path $TargetModel)) {
    throw "Target model not found. Put gemma-4-E4B-it-Q4_K_M.gguf in llama-cpp\models or set SUNDAY_MTP_TARGET_MODEL."
}
if (-not (Test-Path $DraftModel)) {
    throw "Draft/MTP assistant model not found: $DraftModel"
}

$DraftInfo = Get-Item $DraftModel
if ($DraftInfo.Length -lt 700MB) {
    throw "Draft/MTP assistant looks incomplete ($([math]::Round($DraftInfo.Length / 1MB, 1)) MB): $DraftModel"
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
    Write-Host "[DRAFT] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} else {
    Write-Host "[DRAFT] Starting Gemma 4 draft/speculative test on http://127.0.0.1:$Port" -ForegroundColor Cyan
    Write-Host "        Target: $TargetModel"
    Write-Host "        Draft : $DraftModel"
    Write-Host "        Params: ctx=$ContextSize, ngl=$GpuLayers, draft=$DraftMax, batch=$BatchSize/$UBatchSize, cache=$CacheType"

    $Args = @(
        ".\llama-server.exe",
        "-m", "`"$TargetModel`"",
        "--model-draft", "`"$DraftModel`"",
        "--spec-draft-n-max", "$DraftMax",
        "--spec-draft-n-min", "0",
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
        "--cache-type-k", "$CacheType",
        "--cache-type-v", "$CacheType",
        "--cache-ram", "0",
        "--no-warmup",
        "--jinja"
    ) -join " "
    if ($UseMlock) {
        $Args = "$Args --mlock"
    }
    if ($UseNuma) {
        $Args = "$Args --numa $UseNuma"
    }

    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", "cd '$LlamaCppPath'; $Args" `
        -WorkingDirectory $LlamaCppPath `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 120
}

Write-Host "[DRAFT] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
