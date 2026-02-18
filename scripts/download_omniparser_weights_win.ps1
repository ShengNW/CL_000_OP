param(
  [string]$Python = "D:\\exe\\environment\\anaconda\\envs\\Aliyun39\\python.exe",
  [string]$Repo = "microsoft/OmniParser-v2.0",
  [string]$WeightsRoot = "F:\\aw-omni\\models\\omniparser",
  [string]$DocsRoot = "F:\\aw-omni\\src\\docs"
)

$cacheRoot = "F:\\aw-omni\\cache"
$hfCache = Join-Path $cacheRoot "hf"
$tmpCache = Join-Path $cacheRoot "tmp"

New-Item -ItemType Directory -Force -Path $WeightsRoot | Out-Null
New-Item -ItemType Directory -Force -Path $DocsRoot | Out-Null
New-Item -ItemType Directory -Force -Path $hfCache | Out-Null
New-Item -ItemType Directory -Force -Path $tmpCache | Out-Null

$env:HF_HOME = $hfCache
$env:HUGGINGFACE_HUB_CACHE = $hfCache
$env:TRANSFORMERS_CACHE = $hfCache
$env:TMP = $tmpCache
$env:TEMP = $tmpCache

$files = @(
  "icon_detect/train_args.yaml",
  "icon_detect/model.pt",
  "icon_caption/config.json",
  "icon_caption/generation_config.json",
  "icon_caption/model.safetensors"
)

$useHfCli = $false
if (Test-Path $Python) {
  & $Python -c "import huggingface_hub.cli" *> $null
  if ($LASTEXITCODE -eq 0) { $useHfCli = $true }
}

foreach ($file in $files) {
  $skip = $false
  if ($file -like "icon_caption/*") {
    $captionRel = $file -replace "^icon_caption/", "icon_caption_florence/"
    $captionPath = Join-Path $WeightsRoot $captionRel
    if (Test-Path $captionPath) {
      $skip = $true
    }
  }
  if ($skip) {
    continue
  }
  $outPath = Join-Path $WeightsRoot $file
  if (Test-Path $outPath) {
    $info = Get-Item $outPath
    if ($info.Length -gt 0) {
      continue
    }
  }
  $outDir = Split-Path -Parent $outPath
  if (-not (Test-Path $outDir)) {
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
  }
  if ($useHfCli) {
    & $Python -m huggingface_hub.cli download $Repo $file --local-dir $WeightsRoot --local-dir-use-symlinks False
    if ($LASTEXITCODE -ne 0) {
      $useHfCli = $false
    }
  }
  if (-not $useHfCli) {
    $url = "https://huggingface.co/$Repo/resolve/main/$file"
    Invoke-WebRequest -Uri $url -OutFile $outPath
  }
}

$captionDir = Join-Path $WeightsRoot "icon_caption"
$captionFlorence = Join-Path $WeightsRoot "icon_caption_florence"
if (Test-Path $captionDir) {
  if (-not (Test-Path $captionFlorence)) {
    New-Item -ItemType Directory -Force -Path $captionFlorence | Out-Null
  }
  Get-ChildItem -Path $captionDir -File | ForEach-Object {
    Move-Item -Path $_.FullName -Destination $captionFlorence -Force
  }
  Remove-Item -Path $captionDir -Force
}

$manifestPath = Join-Path $DocsRoot "step6_weights_manifest.json"
$manifest = @{
  timestamp = (Get-Date).ToString("o")
  repo = $Repo
  weights_root = $WeightsRoot
  files = @()
  total_bytes = 0
}

$expected = @(
  "icon_detect\\train_args.yaml",
  "icon_detect\\model.pt",
  "icon_caption_florence\\config.json",
  "icon_caption_florence\\generation_config.json",
  "icon_caption_florence\\model.safetensors"
)

foreach ($rel in $expected) {
  $path = Join-Path $WeightsRoot $rel
  if (Test-Path $path) {
    $info = Get-Item $path
    $hash = Get-FileHash -Algorithm SHA256 -Path $path
    $manifest.files += @{
      path = $path
      size_bytes = [int64]$info.Length
      sha256 = $hash.Hash
      ok = $true
    }
    $manifest.total_bytes += [int64]$info.Length
  } else {
    $manifest.files += @{
      path = $path
      size_bytes = 0
      sha256 = ""
      ok = $false
    }
  }
}

$manifest | ConvertTo-Json -Depth 6 | Set-Content -Path $manifestPath -Encoding UTF8
Write-Host "weights_manifest_written=$manifestPath"
