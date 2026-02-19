param(
  [string]$Root = "F:\aw-omni\src",
  [string]$Config = "F:\aw-omni\src\config\local.win.toml",
  [ValidateSet("full","active")] [string]$Mode = "full"
)

if (-not (Test-Path "$Root\Cargo.toml")) {
  Write-Error "Missing workspace at $Root"
  exit 1
}
Set-Location $Root

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "cargo not found in PATH. Use WSL or install Rust on Windows."
  exit 1
}

$params = @{
  mode = $Mode
  format = "png"
  with_cursor = $false
}

if ($env:MCP_AUTH_TOKEN) {
  $params["auth_token"] = $env:MCP_AUTH_TOKEN
}

$payload = @{
  jsonrpc = "2.0"
  id = 1
  method = "screen.bundle"
  params = $params
} | ConvertTo-Json -Compress

$response = $payload | cargo run -p aw_omni_mcp -- --config $Config

if (-not $response) {
  Write-Error "No response from MCP"
  exit 1
}

$obj = $response | ConvertFrom-Json

if ($obj.error) {
  Write-Error "MCP error: $($obj.error.message)"
  exit 1
}

$result = $obj.result

$paths = @($result.raw_path, $result.annotated_path, $result.mask_path)
foreach ($p in $paths) {
  if (-not $p -or -not (Test-Path $p)) {
    Write-Error "Missing output: $p"
    exit 1
  }
}

$elements = $result.elements
if (-not $elements -or $elements.Count -eq 0) {
  Write-Error "Elements empty; parse likely failed"
  exit 1
}

Write-Host "PASS: screen.bundle produced images and elements."
Write-Host "raw_path=$($result.raw_path)"
Write-Host "annotated_path=$($result.annotated_path)"
Write-Host "mask_path=$($result.mask_path)"
