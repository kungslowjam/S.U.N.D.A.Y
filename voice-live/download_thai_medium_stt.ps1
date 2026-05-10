# Download/cache the Thai medium faster-whisper CTranslate2 model used by Voice Live.

$ErrorActionPreference = "Stop"

$Python = if (Get-Command py -ErrorAction SilentlyContinue) { "py" } elseif (Get-Command python -ErrorAction SilentlyContinue) { "python" } else { $null }
if (-not $Python) {
    throw "Python launcher not found. Install Python or make sure 'py' is available in PATH."
}

& $Python -c "import importlib.util, sys; sys.exit(0 if importlib.util.find_spec('faster_whisper') else 1)"
if ($LASTEXITCODE -ne 0) {
    Write-Host "[THAI MEDIUM STT] Installing faster-whisper..." -ForegroundColor Cyan
    & $Python -m pip install --user faster-whisper
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to install faster-whisper."
    }
}

Write-Host "[THAI MEDIUM STT] Downloading/loading Vinxscribe/biodatlab-whisper-th-medium-faster" -ForegroundColor Cyan
Write-Host "Saving to: $ProjectRoot\voice-live\stt_models\thai-medium" -ForegroundColor Gray
Write-Host "This may take several minutes on first run." -ForegroundColor Yellow

@'
import os
from faster_whisper import WhisperModel
from huggingface_hub import snapshot_download

model_id = "Vinxscribe/biodatlab-whisper-th-medium-faster"
local_dir = os.path.join("voice-live", "stt_models", "thai-medium")

print(f"Downloading {model_id} to {local_dir}...", flush=True)
try:
    # Ensure local directory exists
    if not os.path.exists(local_dir):
        os.makedirs(local_dir, exist_ok=True)
    
    # Download using huggingface_hub to specific local dir
    snapshot_download(repo_id=model_id, local_dir=local_dir, local_dir_use_symlinks=False)
    
    print(f"Verifying load from {local_dir}...", flush=True)
    WhisperModel(local_dir, device="cuda", compute_type="int8_float16")
    print("Loaded with CUDA successfully.", flush=True)
except Exception as exc:
    print(f"CUDA load or download failed: {exc}", flush=True)
    try:
        WhisperModel(local_dir, device="cpu", compute_type="int8")
        print("Loaded with CPU successfully.", flush=True)
    except Exception as exc2:
        print(f"Critical error: {exc2}", flush=True)
        exit(1)
'@ | & $Python -

if ($LASTEXITCODE -ne 0) {
    throw "Thai medium STT model download/load failed."
}

Write-Host "[THAI MEDIUM STT] Ready. Saved to voice-live/stt_models/thai-medium" -ForegroundColor Green
