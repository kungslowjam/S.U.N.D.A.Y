# Run the dedicated llama.cpp-omni duplex test binary.

param(
    [int] $GpuLayers = 35,
    [int] $ContextSize = 2048,
    [int] $Chunks = 2,
    [switch] $NoTts,
    [switch] $Omni
)

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$RepoDir = Join-Path $LabRoot "llama.cpp-omni"
$ModelDir = Join-Path $LabRoot "models\MiniCPM-o-4_5-gguf"
$ModelPath = Join-Path $ModelDir "MiniCPM-o-4_5-Q4_K_M.gguf"
$OutputDir = Join-Path $LabRoot "output\duplex"
$Candidates = @(
    (Join-Path $LabRoot "llama.cpp-omni\build-cuda\bin\Release\llama-omni-test-duplex.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build-cuda\bin\llama-omni-test-duplex.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\bin\Release\llama-omni-test-duplex.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\bin\llama-omni-test-duplex.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\tools\omni\Release\llama-omni-test-duplex.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\tools\omni\llama-omni-test-duplex.exe")
)

$Exe = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Exe) {
    throw "llama-omni-test-duplex.exe not found. Run .\omni-lab\build_omni.ps1 -Cuda first."
}
if (-not (Test-Path $ModelPath)) {
    throw "Model not found. Run .\omni-lab\setup_omni_lab.ps1 first."
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$TestPrefix = "tools/omni/assets/test_case/audio_test_case/audio_test_case_"
$Args = @(
    "-m", $ModelPath,
    "-ngl", "$GpuLayers",
    "-c", "$ContextSize",
    "-o", $OutputDir,
    "--test", $TestPrefix, "$Chunks"
)

if ($NoTts) {
    $Args += "--no-tts"
}
if ($Omni) {
    $Args += "--omni"
    $TestPrefix = "tools/omni/assets/test_case/omni_test_case/omni_test_case_"
    $Args = @(
        "-m", $ModelPath,
        "-ngl", "$GpuLayers",
        "-c", "$ContextSize",
        "-o", $OutputDir,
        "--omni",
        "--test", $TestPrefix, "$Chunks"
    )
    if ($NoTts) {
        $Args += "--no-tts"
    }
}

Write-Host "[RUN] Full duplex omni test" -ForegroundColor Cyan
Write-Host "Note: this uses llama-omni-test-duplex, not llama-omni-cli." -ForegroundColor Yellow
Write-Host "Note: RTX 4050 6GB may need -NoTts or lower -GpuLayers if VRAM is tight." -ForegroundColor Yellow
Write-Host "Output: $OutputDir" -ForegroundColor Cyan

Push-Location $RepoDir
try {
    & $Exe @Args
} finally {
    Pop-Location
}
