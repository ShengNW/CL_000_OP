$Python = "D:\exe\environment\anaconda\envs\Aliyun39\python.exe"
$Script = "F:\aw-omni\src\sidecar\omni_sidecar_entry.py"
$Mode = "mock"
$Host = "127.0.0.1"
$Port = "8000"
$RealRepo = "F:\aw-omni\src\third_party\OmniParser"
$WeightsRoot = "F:\aw-omni\models\omniparser"

if ($args.Count -ge 1) { $Mode = $args[0] }
if ($args.Count -ge 2) { $Host = $args[1] }
if ($args.Count -ge 3) { $Port = $args[2] }
if ($args.Count -ge 4) { $RealRepo = $args[3] }
if ($args.Count -ge 5) { $WeightsRoot = $args[4] }

if (-not (Test-Path $Python)) {
  Write-Error "Python not found: $Python"
  exit 2
}
if (-not (Test-Path $Script)) {
  Write-Error "Entry script not found: $Script"
  exit 2
}

& $Python $Script --mode $Mode --host $Host --port $Port --real-repo $RealRepo --weights-root $WeightsRoot
