# Run llama-omni-cli with full TTS/omni modules enabled.

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$ModelDir = Join-Path $LabRoot "models\MiniCPM-o-4_5-gguf"
$ModelPath = Join-Path $ModelDir "MiniCPM-o-4_5-Q4_K_M.gguf"
$OutputDir = Join-Path $LabRoot "output"
$Candidates = @(
    (Join-Path $LabRoot "llama.cpp-omni\build\bin\Release\llama-omni-cli.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\bin\llama-omni-cli.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\examples\omni\Release\llama-omni-cli.exe"),
    (Join-Path $LabRoot "llama.cpp-omni\build\examples\omni\llama-omni-cli.exe")
)

$Exe = $Candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $Exe) {
    throw "llama-omni-cli.exe not found. Run .\omni-lab\build_omni.ps1 first."
}
if (-not (Test-Path $ModelPath)) {
    throw "Model not found. Run .\omni-lab\setup_omni_lab.ps1 first."
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

Write-Host "[RUN] Full omni test" -ForegroundColor Cyan
Write-Host "Note: full Q4_K_M omni is documented around 9GB VRAM; RTX 4050 6GB may be slow or fail." -ForegroundColor Yellow
& $Exe -m $ModelPath -ngl 35 -c 2048
