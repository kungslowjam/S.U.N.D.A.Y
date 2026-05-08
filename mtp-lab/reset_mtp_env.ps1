# Clears MTP lab environment overrides in the current PowerShell process.

Remove-Item Env:SUNDAY_MTP_PROFILE -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_CACHE_TYPE -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_DRAFT_BLOCK_SIZE -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_DRAFT_MAX -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_DRAFT_MIN -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_CONTEXT_SIZE -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_TARGET_MODEL -ErrorAction SilentlyContinue
Remove-Item Env:SUNDAY_MTP_HEAD_MODEL -ErrorAction SilentlyContinue

Write-Host "[MTP] Environment overrides cleared." -ForegroundColor Green
