param(
  [string]$Python = "D:\exe\environment\anaconda\envs\Aliyun39\python.exe",
  [string]$RepoPath = "F:\aw-omni\src\third_party\OmniParser",
  [string]$WeightsRoot = "F:\aw-omni\models\omniparser",
  [string]$Mode = "real_local_aliyun39",
  [int]$Port = 8000,
  [int]$Retries = 15,
  [string]$OutPath = ""
)

$Root = Resolve-Path "$PSScriptRoot\.."
if (-not $OutPath) {
  $OutPath = Join-Path $Root "docs\step5_preflight_report.json"
}

$entry = Join-Path $Root "sidecar\omni_sidecar_entry.py"

if (-not (Test-Path $Python)) {
  Write-Error "Python not found: $Python"
  exit 2
}
if (-not (Test-Path $entry)) {
  Write-Error "Entry script not found: $entry"
  exit 2
}

$report = @{
  python = $Python
  repo_exists = (Test-Path $RepoPath)
  weights_exists = (Test-Path $WeightsRoot)
  probe_ok = $false
  probe_response = $null
  error = $null
}

if (-not $report.repo_exists) {
  $report.error = "repo_missing"
  $report | ConvertTo-Json -Depth 6 | Set-Content -Path $OutPath -Encoding UTF8
  Write-Host "preflight_written=$OutPath"
  exit 0
}

$proc = $null
try {
  $proc = Start-Process -FilePath $Python -ArgumentList @(
    $entry,
    "--mode", $Mode,
    "--host", "127.0.0.1",
    "--port", "$Port",
    "--real-repo", $RepoPath,
    "--weights-root", $WeightsRoot
  ) -PassThru -WindowStyle Hidden

  $uri = "http://127.0.0.1:$Port/probe"
  $resp = $null
  for ($i = 0; $i -lt $Retries; $i++) {
    Start-Sleep -Milliseconds 1000
    try {
      $resp = Invoke-RestMethod -Uri $uri -Method GET -TimeoutSec 3
      break
    } catch {
      $resp = $null
    }
  }
  if ($resp -ne $null) {
    $report.probe_ok = $true
    $report.probe_response = $resp
  } else {
    $report.error = "probe_unreachable"
  }
} catch {
  $report.error = $_.Exception.Message
} finally {
  if ($proc -and -not $proc.HasExited) {
    $proc.Kill()
  }
}

$report | ConvertTo-Json -Depth 6 | Set-Content -Path $OutPath -Encoding UTF8
Write-Host "preflight_written=$OutPath"
