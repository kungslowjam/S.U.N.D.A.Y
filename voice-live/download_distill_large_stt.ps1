$ProjectRoot = Get-Location
$Python = "python"
if (Get-Command "py" -ErrorAction SilentlyContinue) { $Python = "py" }

Write-Host "[DISTILL LARGE STT] Downloading pariya47/distill-whisper-th-large-v3-ct2" -ForegroundColor Cyan
Write-Host "Saving to: $ProjectRoot\voice-live\stt_models\distill-large" -ForegroundColor Gray

@'
import os
from faster_whisper import WhisperModel
from huggingface_hub import snapshot_download

model_id = "pariya47/distill-whisper-th-large-v3-ct2"
local_dir = os.path.join("voice-live", "stt_models", "distill-large")

print(f"Downloading {model_id} to {local_dir}...", flush=True)
try:
    if not os.path.exists(local_dir):
        os.makedirs(local_dir, exist_ok=True)
    
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
    throw "Distill Large STT model download failed."
}

Write-Host "[DISTILL LARGE STT] Ready. Saved to voice-live/stt_models/distill-large" -ForegroundColor Green
