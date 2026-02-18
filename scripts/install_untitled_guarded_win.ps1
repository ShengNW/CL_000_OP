param(
  [string]$Python = "D:\exe\environment\anaconda\envs\untitled\python.exe",
  [string]$Constraints = "",
  [string]$ReportPath = ""
)

$Root = Resolve-Path "$PSScriptRoot\.."
$Docs = Join-Path $Root "docs"
$SnapshotScript = Join-Path $Root "scripts\protect_snapshot_win.ps1"

if (-not $Constraints) {
  $Constraints = Join-Path $Docs "step7_untitled_protected_constraints.txt"
}
if (-not $ReportPath) {
  $ReportPath = Join-Path $Docs "step7_untitled_install_report.md"
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
"# Step7 Untitled Install Batches Report" | Set-Content -Path $ReportPath -Encoding UTF8
"Generated: $timestamp" | Add-Content -Path $ReportPath
"" | Add-Content -Path $ReportPath

$comparePy = @'
import json
import sys

def filter_freeze(lines):
    if not lines:
        return []
    out = []
    for line in lines:
        l = line.lower()
        if l.startswith("nvidia-") or l.startswith("cuda") or "cudnn" in l or "tensorrt" in l:
            out.append(line)
    return out

def main():
    pre = json.load(open(sys.argv[1], "r", encoding="utf-8-sig"))
    post = json.load(open(sys.argv[2], "r", encoding="utf-8-sig"))
    keys = [
        "numpy_version",
        "numpy_file",
        "torch_version",
        "torch_version_cuda",
        "torch_file",
        "torchvision_version",
        "torchaudio_version",
    ]
    diff = 0
    for k in keys:
        if pre.get(k) != post.get(k):
            diff += 1
    if filter_freeze(pre.get("protected_freeze")) != filter_freeze(post.get("protected_freeze")):
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
    [switch]$BlockOnFail
  )

  "## $Name" | Add-Content -Path $ReportPath
  "Packages: $($Packages -join ' ')" | Add-Content -Path $ReportPath

  $prePath = Join-Path $Docs "step7_untitled_pre_batch.json"
  $postPath = Join-Path $Docs "step7_untitled_post_batch.json"

  & $SnapshotScript -Stage pre -Python $Python -OutPath $prePath | Out-Null

  $env:PIP_DISABLE_PIP_VERSION_CHECK = "1"
  $env:PIP_NO_CACHE_DIR = "1"
  $env:HF_HOME = "F:\aw-omni\cache\hf"
  $env:HUGGINGFACE_HUB_CACHE = "F:\aw-omni\cache\hf"
  $env:TRANSFORMERS_CACHE = "F:\aw-omni\cache\hf"
  $env:EASYOCR_HOME = "F:\aw-omni\cache\easyocr"
  $env:TMP = "F:\aw-omni\cache\tmp"
  $env:TEMP = "F:\aw-omni\cache\tmp"

  foreach ($pkg in $Packages) {
    & $Python -m pip install --no-cache-dir -c $Constraints --upgrade-strategy only-if-needed $pkg
    if ($LASTEXITCODE -ne 0) {
      if ($BlockOnFail) {
        "Result: BLOCK (pip install error for $pkg)" | Add-Content -Path $ReportPath
        "" | Add-Content -Path $ReportPath
        exit 12
      }
      "Result: FAIL (pip install error for $pkg)" | Add-Content -Path $ReportPath
      "" | Add-Content -Path $ReportPath
      exit 10
    }
  }

  & $SnapshotScript -Stage post -Python $Python -OutPath $postPath | Out-Null

  $diffCount = $comparePy | & $Python - $prePath $postPath
  if ($LASTEXITCODE -ne 0) {
    "Result: FAIL (protected diff detected, count=$diffCount)" | Add-Content -Path $ReportPath
    "" | Add-Content -Path $ReportPath
    exit 11
  }

  "Result: OK (protected diff count=$diffCount)" | Add-Content -Path $ReportPath
  "" | Add-Content -Path $ReportPath
}

Invoke-Batch -Name "batchU1-runtime" -Packages @(
  "fastapi",
  "uvicorn",
  "pydantic",
  "pillow"
)

Invoke-Batch -Name "batchU2-inference" -Packages @(
  "transformers",
  "accelerate",
  "supervision",
  "timm",
  "einops==0.8.0",
  "ultralytics==8.3.70"
)

Invoke-Batch -Name "batchU3-ocr" -Packages @(
  "easyocr",
  "defusedxml",
  "thop"
)

Invoke-Batch -Name "batchU4-paddle" -Packages @(
  "paddlepaddle",
  "paddleocr"
) -BlockOnFail
