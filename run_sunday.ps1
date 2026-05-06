# SUNDAY GPU Runner for llama.cpp
# This script sets up the PATH and runs SUNDAY with GPU acceleration.

$LlamaCppPath = "c:\Users\hello\Desktop\Project_me\SUNDAY\llama-cpp"
$ModelPath = "models\your-model.gguf" # <-- แก้ไขที่อยู่ไฟล์โมเดลตรงนี้
$GpuLayers = 33

# 1. เพิ่ม llama.cpp ใน PATH ของ Session นี้
if ($env:PATH -notlike "*$LlamaCppPath*") {
    $env:PATH = "$LlamaCppPath;$env:PATH"
    Write-Host "[info] Added llama-cpp to PATH" -ForegroundColor Cyan
}

# 2. ตรวจสอบ venv
if (Test-Path ".venv\Scripts\activate.ps1") {
    Write-Host "[info] Activating virtual environment..." -ForegroundColor Cyan
    & .venv\Scripts\activate.ps1
} else {
    Write-Host "[warn] Virtual environment (.venv) not found. Trying global 'sunday' command..." -ForegroundColor Yellow
}

# 3. รัน SUNDAY Host
Write-Host "[info] Starting SUNDAY with llama.cpp GPU ($GpuLayers layers)..." -ForegroundColor Green
sunday host $ModelPath --backend llamacpp --gpu-layers $GpuLayers
