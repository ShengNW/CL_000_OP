$RealRepo = "F:\aw-omni\src\third_party\OmniParser"
$WeightsRoot = "F:\aw-omni\models\omniparser"

if ($args.Count -ge 1) { $RealRepo = $args[0] }
if ($args.Count -ge 2) { $WeightsRoot = $args[1] }

$missing = @()
if (-not (Test-Path $RealRepo)) { $missing += "real_repo_missing:$RealRepo" }
if (-not (Test-Path (Join-Path $RealRepo "omnitool\\omniparserserver\\omniparserserver.py"))) {
  $missing += "omniparserserver_missing:$RealRepo\\omnitool\\omniparserserver\\omniparserserver.py"
}
if (-not (Test-Path $WeightsRoot)) { $missing += "weights_root_missing:$WeightsRoot" }

if ($missing.Count -gt 0) {
  $payload = @{ ok = $false; missing = $missing }
  $payload | ConvertTo-Json -Depth 3
  exit 3
}

$payload = @{ ok = $true; missing = @() }
$payload | ConvertTo-Json -Depth 3
