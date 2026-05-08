# Starts an isolated Atomic Gemma 4 MTP server for Voice Live testing.

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
$LlamaModelsPath = Join-Path $ProjectRoot "llama-cpp\models"

$Port = if ($env:SUNDAY_MTP_PORT) { [int]$env:SUNDAY_MTP_PORT } else { 8084 }
$TargetModel = if ($env:SUNDAY_MTP_TARGET_MODEL) {
    $env:SUNDAY_MTP_TARGET_MODEL
} else {
    $TargetCandidates = @(
        (Join-Path $LlamaModelsPath "gemma-4-E4B-it-Q4_K_M.gguf"),
        (Join-Path $LlamaModelsPath "gemma-4-E4B-it-ultra-uncensored-heretic-Q4_K_M.gguf")
    )
    $TargetCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$MtpHeadCandidates = @(
    (Join-Path $LlamaModelsPath "gemma-4-E4B-it-assistant.Q4_K_M.gguf"),
    (Join-Path $LlamaModelsPath "gemma-4-E4B-it-assistant-Q4_K_M.gguf")
)
$MtpHead = if ($env:SUNDAY_MTP_HEAD_MODEL) {
    $env:SUNDAY_MTP_HEAD_MODEL
} else {
    $MtpHeadCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
}
$ContextSize = if ($env:SUNDAY_MTP_CONTEXT_SIZE) { [int]$env:SUNDAY_MTP_CONTEXT_SIZE } else { 1024 }
$GpuLayers = if ($env:SUNDAY_MTP_GPU_LAYERS) { [int]$env:SUNDAY_MTP_GPU_LAYERS } else { 99 }
$DraftBlockSize = if ($env:SUNDAY_MTP_DRAFT_BLOCK_SIZE) { [int]$env:SUNDAY_MTP_DRAFT_BLOCK_SIZE } else { 3 }
$DraftMax = if ($env:SUNDAY_MTP_DRAFT_MAX) { [int]$env:SUNDAY_MTP_DRAFT_MAX } else { 8 }
$DraftMin = if ($env:SUNDAY_MTP_DRAFT_MIN) { [int]$env:SUNDAY_MTP_DRAFT_MIN } else { 0 }
$CacheType = if ($env:SUNDAY_MTP_CACHE_TYPE) { $env:SUNDAY_MTP_CACHE_TYPE } else { "turbo3" }
$Profile = if ($env:SUNDAY_MTP_PROFILE) { $env:SUNDAY_MTP_PROFILE.ToLowerInvariant() } else { "turbo" }
if ($Profile -eq "stable") {
    if (-not $env:SUNDAY_MTP_CACHE_TYPE) { $CacheType = "f16" }
    if (-not $env:SUNDAY_MTP_DRAFT_BLOCK_SIZE) { $DraftBlockSize = 2 }
    if (-not $env:SUNDAY_MTP_DRAFT_MAX) { $DraftMax = 4 }
}
$ConsoleWindowStyle = if ($env:SUNDAY_CONSOLE_STYLE) { $env:SUNDAY_CONSOLE_STYLE } else { "Minimized" }
if (@("Normal", "Hidden", "Minimized", "Maximized") -notcontains $ConsoleWindowStyle) {
    $ConsoleWindowStyle = "Minimized"
}

$ServerCandidates = @(
    (Join-Path $BuildPath "bin\Release\llama-server.exe"),
    (Join-Path $BuildPath "bin\llama-server.exe"),
    (Join-Path $BuildPath "examples\server\Release\llama-server.exe"),
    (Join-Path $BuildPath "examples\server\llama-server.exe")
)
$ServerPath = $ServerCandidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $ServerPath) {
    throw "Atomic llama-server.exe not found. Run .\mtp-lab\setup_atomic_mtp.ps1 first."
}
if (-not $TargetModel -or -not (Test-Path $TargetModel)) {
    throw "Target model not found. Put gemma-4-E4B-it-Q4_K_M.gguf in llama-cpp\models or set SUNDAY_MTP_TARGET_MODEL."
}
if (-not $MtpHead -or -not (Test-Path $MtpHead)) {
    throw "Gemma 4 MTP assistant head not found. Atomic --mtp-head requires a GGUF with architecture gemma4_assistant, not HackAfterDark ultralight gemma4. Put a compatible assistant file in llama-cpp\models or set SUNDAY_MTP_HEAD_MODEL:`n - $($MtpHeadCandidates -join "`n - ")"
}

$HeadInfo = Get-Item $MtpHead
if ($HeadInfo.Length -lt 10MB) {
    throw "MTP assistant head looks incomplete ($([math]::Round($HeadInfo.Length / 1MB, 1)) MB): $MtpHead"
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
    Write-Host "[MTP] Already running on http://127.0.0.1:$Port" -ForegroundColor Yellow
} else {
    Write-Host "[MTP] Starting Atomic Gemma 4 MTP on http://127.0.0.1:$Port" -ForegroundColor Cyan
    Write-Host "      Target : $TargetModel"
    Write-Host "      MTP    : $MtpHead"
    Write-Host "      Profile: $Profile (cache=$CacheType, draft-block=$DraftBlockSize, draft=$DraftMin..$DraftMax, ctx=$ContextSize)"
    $Args = @(
        "`"$ServerPath`"",
        "-m", "`"$TargetModel`"",
        "--mtp-head", "`"$MtpHead`"",
        "--spec-type", "mtp",
        "--draft-block-size", "$DraftBlockSize",
        "--draft-max", "$DraftMax",
        "--draft-min", "$DraftMin",
        "--port", "$Port",
        "--host", "127.0.0.1",
        "-ngl", "$GpuLayers",
        "-ngld", "$GpuLayers",
        "-c", "$ContextSize",
        "-np", "1",
        "-fa", "on",
        "--swa-full",
        "-ctk", "$CacheType",
        "-ctv", "$CacheType",
        "-ctkd", "$CacheType",
        "-ctvd", "$CacheType",
        "--cache-ram", "0",
        "--no-warmup"
    ) -join " "
    Start-Process powershell `
        -ArgumentList "-NoExit", "-Command", $Args `
        -WorkingDirectory (Split-Path $ServerPath -Parent) `
        -WindowStyle $ConsoleWindowStyle
    Wait-ForHttp "http://127.0.0.1:$Port/v1/models" 120
}

Write-Host "[MTP] Ready: http://127.0.0.1:$Port/v1/chat/completions" -ForegroundColor Green
