# Open the latest llama.cpp-omni output folder.

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$OutputRoot = Join-Path $LabRoot "llama.cpp-omni\tools\omni\output"

if (-not (Test-Path $OutputRoot)) {
    throw "No omni output folder found. Run .\omni-lab\run_omni_full.ps1 first."
}

$LatestRound = Get-ChildItem -LiteralPath $OutputRoot -Directory -Filter "round_*" |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1

if (-not $LatestRound) {
    Write-Host "[OPEN] No round_* folder found; opening output root." -ForegroundColor Yellow
    explorer $OutputRoot
    exit 0
}

$TtsWav = Join-Path $LatestRound.FullName "tts_wav"
if (Test-Path $TtsWav) {
    Write-Host "[OPEN] $TtsWav" -ForegroundColor Cyan
    explorer $TtsWav
    exit 0
}

Write-Host "[OPEN] $($LatestRound.FullName)" -ForegroundColor Cyan
explorer $LatestRound.FullName
