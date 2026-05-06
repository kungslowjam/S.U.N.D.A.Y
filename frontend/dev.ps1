# SUNDAY Frontend — Dev Server
# เปิด dev server (hot reload) พร้อม proxy ไป backend ที่ localhost:8000

Set-Location -LiteralPath "$PSScriptRoot"

Write-Host "Starting SUNDAY frontend dev server..." -ForegroundColor Cyan
Write-Host "  Frontend : http://localhost:5173" -ForegroundColor Green
Write-Host "  Backend  : http://localhost:8000 (proxy)" -ForegroundColor Gray
Write-Host ""

npm run dev
