param(
  [string]$Python = "D:\\exe\\environment\\anaconda\\envs\\Aliyun39\\python.exe",
  [string]$Script = "F:\\aw-omni\\src\\sidecar\\omni_sidecar_entry.py",
  [string]$Mode = "real_local_aliyun39",
  [string]$BindHost = "127.0.0.1",
  [int]$Port = 8000,
  [string]$RealRepo = "F:\\aw-omni\\src\\third_party\\OmniParser",
  [string]$WeightsRoot = "F:\\aw-omni\\models\\omniparser",
  [string]$ImagePath = "",
  [string]$OutPath = "F:\\aw-omni\\src\\docs\\step6_first_parse_result.json"
)

if (-not (Test-Path $Python)) {
  Write-Error "Python not found: $Python"
  exit 2
}
if (-not (Test-Path $Script)) {
  Write-Error "Entry script not found: $Script"
  exit 2
}

if (-not $ImagePath) {
  $candidates = @(
    "F:\\aw-omni\\src\\third_party\\OmniParser\\imgs\\google_page.png",
    "F:\\aw-omni\\src\\third_party\\OmniParser\\imgs\\excel.png",
    "F:\\aw-omni\\src\\third_party\\OmniParser\\imgs\\ios.png"
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      $ImagePath = $candidate
      break
    }
  }
}

if (-not (Test-Path $ImagePath)) {
  Write-Error "Image not found for parse test."
  exit 3
}

$proc = Start-Process -FilePath $Python -ArgumentList @(
  $Script,
  "--mode", $Mode,
  "--host", $BindHost,
  "--port", $Port,
  "--real-repo", $RealRepo,
  "--weights-root", $WeightsRoot
) -PassThru -WindowStyle Hidden

$probe = $null
for ($i = 0; $i -lt 60; $i++) {
  try {
    $probe = Invoke-RestMethod -Uri "http://$BindHost`:$Port/probe" -TimeoutSec 2
    break
  } catch {
    Start-Sleep -Seconds 1
  }
}

if (-not $probe) {
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  Write-Error "Probe failed to respond."
  exit 4
}

if (-not $probe.ready) {
  $result = @{
    timestamp = (Get-Date).ToString("o")
    ok = $false
    reason = $probe.reason
    probe = $probe
    image = $ImagePath
  }
  $result | ConvertTo-Json -Depth 8 | Set-Content -Path $OutPath -Encoding UTF8
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  Write-Host "parse_result_written=$OutPath"
  exit 5
}

$imageBytes = [System.IO.File]::ReadAllBytes($ImagePath)
$imageB64 = [Convert]::ToBase64String($imageBytes)
$payload = @{ base64_image = $imageB64 } | ConvertTo-Json -Depth 4

$start = Get-Date
try {
  $response = Invoke-RestMethod -Method Post -Uri "http://$BindHost`:$Port/parse" -ContentType "application/json" -Body $payload -TimeoutSec 300
} catch {
  $err = $_.Exception
  $body = ""
  if ($err.Response -and $err.Response.GetResponseStream()) {
    $reader = New-Object System.IO.StreamReader($err.Response.GetResponseStream())
    $body = $reader.ReadToEnd()
    $reader.Close()
  }
  $fail = @{
    timestamp = (Get-Date).ToString("o")
    ok = $false
    error = "parse_failed"
    http_error = $err.Message
    response_body = $body
    probe = $probe
    image = $ImagePath
  }
  $fail | ConvertTo-Json -Depth 10 | Set-Content -Path $OutPath -Encoding UTF8
  Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
  Write-Host "parse_result_written=$OutPath"
  exit 6
}
$elapsedMs = [math]::Round(((Get-Date) - $start).TotalMilliseconds, 2)

$elements = 0
$hasText = $false
$hasIcon = $false
if ($response.parsed_content_list) {
  $elements = $response.parsed_content_list.Count
  foreach ($item in $response.parsed_content_list) {
    if ($item -is [string]) {
      if ($item -match "^Text Box") { $hasText = $true }
      if ($item -match "^Icon Box") { $hasIcon = $true }
    } elseif ($item.type) {
      if ($item.type -eq "text") { $hasText = $true }
      if ($item.type -eq "icon") { $hasIcon = $true }
    }
  }
}

$result = @{
  timestamp = (Get-Date).ToString("o")
  ok = $true
  image = $ImagePath
  latency_ms = $response.latency_ms
  latency_ms_wall = $elapsedMs
  element_count = $elements
  has_text = $hasText
  has_icon = $hasIcon
  probe = $probe
  response = $response
}

$result | ConvertTo-Json -Depth 10 | Set-Content -Path $OutPath -Encoding UTF8
Write-Host "parse_result_written=$OutPath"

Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
