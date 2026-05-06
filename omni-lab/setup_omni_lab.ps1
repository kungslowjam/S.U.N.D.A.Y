# Prepare isolated llama.cpp-omni runtime and MiniCPM-o omni model files.

$ErrorActionPreference = "Stop"

$ProjectRoot = Split-Path -Parent $PSScriptRoot
$LabRoot = $PSScriptRoot
$RepoDir = Join-Path $LabRoot "llama.cpp-omni"
$ModelDir = Join-Path $LabRoot "models\MiniCPM-o-4_5-gguf"
$ExistingLlm = Join-Path $ProjectRoot "llama-cpp\models\MiniCPM-o-4_5-Q4_K_M.gguf"
$TargetLlm = Join-Path $ModelDir "MiniCPM-o-4_5-Q4_K_M.gguf"

function Ensure-Dir {
    param([string]$Path)
    if (-not (Test-Path $Path)) {
        New-Item -ItemType Directory -Path $Path | Out-Null
    }
}

function Download-File {
    param([string]$Url, [string]$OutPath)
    if (Test-Path $OutPath) {
        Write-Host "[SKIP] $OutPath" -ForegroundColor Yellow
        return
    }
    Ensure-Dir (Split-Path -Parent $OutPath)
    Write-Host "[GET] $Url" -ForegroundColor Cyan
    & curl.exe -L --fail --continue-at - --retry 3 --output $OutPath $Url
    if ($LASTEXITCODE -ne 0) {
        throw "Download failed: $Url"
    }
}

Ensure-Dir $LabRoot
Ensure-Dir $ModelDir
Ensure-Dir (Join-Path $LabRoot "output")
Ensure-Dir (Join-Path $LabRoot "build-logs")

if (-not (Test-Path $RepoDir)) {
    Write-Host "[CLONE] tc-mb/llama.cpp-omni" -ForegroundColor Cyan
    git clone https://github.com/tc-mb/llama.cpp-omni.git $RepoDir
} else {
    Write-Host "[SKIP] Runtime repo already exists: $RepoDir" -ForegroundColor Yellow
}

if (Test-Path $ExistingLlm) {
    if (-not (Test-Path $TargetLlm)) {
        Write-Host "[COPY] Reusing existing LLM GGUF" -ForegroundColor Cyan
        Copy-Item -LiteralPath $ExistingLlm -Destination $TargetLlm
    } else {
        Write-Host "[SKIP] LLM GGUF already exists: $TargetLlm" -ForegroundColor Yellow
    }
} else {
    Download-File `
        "https://huggingface.co/openbmb/MiniCPM-o-4_5-gguf/resolve/main/MiniCPM-o-4_5-Q4_K_M.gguf" `
        $TargetLlm
}

$Base = "https://huggingface.co/openbmb/MiniCPM-o-4_5-gguf/resolve/main"
$Files = @(
    @("$Base/audio/MiniCPM-o-4_5-audio-F16.gguf", "audio\MiniCPM-o-4_5-audio-F16.gguf"),
    @("$Base/vision/MiniCPM-o-4_5-vision-F16.gguf", "vision\MiniCPM-o-4_5-vision-F16.gguf"),
    @("$Base/tts/MiniCPM-o-4_5-tts-F16.gguf", "tts\MiniCPM-o-4_5-tts-F16.gguf"),
    @("$Base/tts/MiniCPM-o-4_5-projector-F16.gguf", "tts\MiniCPM-o-4_5-projector-F16.gguf"),
    @("$Base/token2wav-gguf/encoder.gguf", "token2wav-gguf\encoder.gguf"),
    @("$Base/token2wav-gguf/flow_matching.gguf", "token2wav-gguf\flow_matching.gguf"),
    @("$Base/token2wav-gguf/flow_extra.gguf", "token2wav-gguf\flow_extra.gguf"),
    @("$Base/token2wav-gguf/hifigan2.gguf", "token2wav-gguf\hifigan2.gguf"),
    @("$Base/token2wav-gguf/prompt_cache.gguf", "token2wav-gguf\prompt_cache.gguf")
)

foreach ($item in $Files) {
    Download-File $item[0] (Join-Path $ModelDir $item[1])
}

Write-Host ""
Write-Host "[OK] Omni lab is prepared." -ForegroundColor Green
Write-Host "Runtime: $RepoDir"
Write-Host "Models : $ModelDir"
