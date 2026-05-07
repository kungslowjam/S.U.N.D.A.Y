# Download/cache the Thai faster-whisper CTranslate2 model used by Voice Live.

$ErrorActionPreference = "Stop"

$Python = if (Get-Command py -ErrorAction SilentlyContinue) { "py" } elseif (Get-Command python -ErrorAction SilentlyContinue) { "python" } else { $null }
if (-not $Python) {
    throw "Python launcher not found. Install Python or make sure 'py' is available in PATH."
}

& $Python -c "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('faster_whisper') else 1)"
if ($LASTEXITCODE -ne 0) {
    Write-Host "[THAI STT] Installing faster-whisper..." -ForegroundColor Cyan
    & $Python -m pip install --user faster-whisper
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to install faster-whisper."
    }
}

Write-Host "[THAI STT] Downloading/loading pariya47/distill-whisper-th-large-v3-ct2" -ForegroundColor Cyan
Write-Host "This is a large model. The first run can take several minutes." -ForegroundColor Yellow

@'
from faster_whisper import WhisperModel

model_id = "pariya47/distill-whisper-th-large-v3-ct2"
print(f"Loading {model_id} ...", flush=True)
try:
    WhisperModel(model_id, device="cuda", compute_type="int8_float16")
    print("Loaded with CUDA.", flush=True)
except Exception as exc:
    print(f"CUDA load failed: {exc}", flush=True)
    WhisperModel(model_id, device="cpu", compute_type="int8")
    print("Loaded with CPU.", flush=True)
'@ | & $Python -

if ($LASTEXITCODE -ne 0) {
    throw "Thai STT model download/load failed."
}

Write-Host "[THAI STT] Ready." -ForegroundColor Green
