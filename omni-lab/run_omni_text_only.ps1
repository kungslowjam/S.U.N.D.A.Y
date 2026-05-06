# Run llama-omni-cli with TTS disabled. This is the safest first test on 6GB VRAM.

$ErrorActionPreference = "Stop"

$LabRoot = $PSScriptRoot
$ModelPath = Join-Path $LabRoot "models\MiniCPM-o-4_5-gguf\MiniCPM-o-4_5-Q4_K_M.gguf"
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

Write-Host "[RUN] Text-only omni test" -ForegroundColor Cyan
& $Exe -m $ModelPath --no-tts -ngl 35 -c 2048
