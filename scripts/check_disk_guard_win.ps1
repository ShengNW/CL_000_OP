param(
  [string]$Drive = "F",
  [int]$MinFreeGB = 8,
  [string]$OutPath = "F:\\aw-omni\\src\\docs\\step6_disk_guard.json"
)

$driveInfo = Get-PSDrive -Name $Drive -ErrorAction SilentlyContinue
$payload = @{
  timestamp = (Get-Date).ToString("o")
  drive = $Drive
  min_free_gb = $MinFreeGB
}

if (-not $driveInfo) {
  $payload.status = "BLOCK"
  $payload.reason = "drive_not_found"
  $payload.free_bytes = 0
  $payload.free_gb = 0
} else {
  $freeBytes = [int64]$driveInfo.Free
  $freeGB = [math]::Round($freeBytes / 1GB, 2)
  $payload.free_bytes = $freeBytes
  $payload.free_gb = $freeGB
  if ($freeBytes -lt ($MinFreeGB * 1GB)) {
    $payload.status = "BLOCK"
    $payload.reason = "insufficient_free_space"
  } else {
    $payload.status = "PASS"
    $payload.reason = "ok"
  }
}

$outDir = Split-Path -Parent $OutPath
if (-not (Test-Path $outDir)) {
  New-Item -ItemType Directory -Force -Path $outDir | Out-Null
}

$payload | ConvertTo-Json -Depth 6 | Set-Content -Path $OutPath -Encoding UTF8
Write-Host "disk_guard_written=$OutPath"

if ($payload.status -ne "PASS") {
  exit 3
}
