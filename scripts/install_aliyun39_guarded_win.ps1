param(
  [string]$Python = "D:\exe\environment\anaconda\envs\Aliyun39\python.exe",
  [string]$Constraints = "",
  [string]$ReportPath = ""
)

$Root = Resolve-Path "$PSScriptRoot\.."
$Docs = Join-Path $Root "docs"
$SnapshotScript = Join-Path $Root "scripts\protect_snapshot_win.ps1"

if (-not $Constraints) {
  $Constraints = Join-Path $Docs "protected_constraints.txt"
}
if (-not $ReportPath) {
  $ReportPath = Join-Path $Docs "step5_install_batches_report.md"
}

if (-not (Test-Path $Python)) {
  Write-Error "Python not found: $Python"
  exit 2
}
if (-not (Test-Path $Constraints)) {
  Write-Error "Constraints not found: $Constraints"
  exit 2
}
if (-not (Test-Path $SnapshotScript)) {
  Write-Error "Snapshot script not found: $SnapshotScript"
  exit 2
}

$timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
"# Step5 Install Batches Report" | Set-Content -Path $ReportPath -Encoding UTF8
"Generated: $timestamp" | Add-Content -Path $ReportPath
"" | Add-Content -Path $ReportPath

$comparePy = @'
import json
import sys

def main():
    pre = json.load(open(sys.argv[1], "r", encoding="utf-8-sig"))
    post = json.load(open(sys.argv[2], "r", encoding="utf-8-sig"))
    keys = ["torch_version_cuda", "torch_file", "cv2_file", "protected_freeze"]
    diff = 0
    for k in keys:
        if pre.get(k) != post.get(k):
            diff += 1
    print(diff)
    return 0 if diff == 0 else 5

if __name__ == "__main__":
    raise SystemExit(main())
'@

function Get-PackageName {
  param([string]$Spec)
  if ($Spec -match "==") { return $Spec.Split("==")[0] }
  if ($Spec -match "@") { return $Spec.Split("@")[0].Trim() }
  return $Spec
}

function Invoke-Batch {
  param(
    [string]$Name,
    [string[]]$Packages,
    [string[]]$NoDepsPackages
  )

  "## $Name" | Add-Content -Path $ReportPath
  "Packages: $($Packages -join ' ')" | Add-Content -Path $ReportPath
  if ($NoDepsPackages -and $NoDepsPackages.Count -gt 0) {
    "NoDeps: $($NoDepsPackages -join ' ')" | Add-Content -Path $ReportPath
  }

  & $SnapshotScript -Stage pre -Python $Python | Out-Null
  $prePath = Join-Path $Docs "protect_pre.json"
  $postPath = Join-Path $Docs "protect_post.json"

  $env:PIP_DISABLE_PIP_VERSION_CHECK = "1"
  $env:PIP_NO_CACHE_DIR = "1"

  foreach ($pkg in $Packages) {
    $pkgName = Get-PackageName -Spec $pkg
    $useNoDeps = $false
    if ($NoDepsPackages) {
      $useNoDeps = $NoDepsPackages -contains $pkgName
    }

    if ($useNoDeps) {
      & $Python -m pip install --no-cache-dir --no-deps -c $Constraints --upgrade-strategy only-if-needed $pkg
    } else {
      & $Python -m pip install --no-cache-dir -c $Constraints --upgrade-strategy only-if-needed $pkg
    }

    if ($LASTEXITCODE -ne 0) {
      "Result: FAIL (pip install error for $pkg)" | Add-Content -Path $ReportPath
      "" | Add-Content -Path $ReportPath
      exit 10
    }
  }

  & $SnapshotScript -Stage post -Python $Python | Out-Null

  $diffCount = $comparePy | & $Python - $prePath $postPath
  if ($LASTEXITCODE -ne 0) {
    "Result: FAIL (protected diff detected, count=$diffCount)" | Add-Content -Path $ReportPath
    "" | Add-Content -Path $ReportPath
    exit 11
  }

  "Result: OK (protected diff count=$diffCount)" | Add-Content -Path $ReportPath
  "" | Add-Content -Path $ReportPath
}

Invoke-Batch -Name "batchA-inference" -Packages @(
  "transformers",
  "accelerate",
  "timm",
  "einops==0.8.0",
  "ultralytics==8.3.70"
) -NoDepsPackages @(
  "timm",
  "ultralytics"
)

Invoke-Batch -Name "batchB-supervision" -Packages @(
  "supervision==0.18.0"
) -NoDepsPackages @(
  "supervision"
)

Invoke-Batch -Name "batchC-ocr-min" -Packages @(
  "easyocr"
) -NoDepsPackages @(
  "easyocr"
)

Invoke-Batch -Name "batchD-support" -Packages @(
  "thop",
  "defusedxml",
  "python-bidi"
)
