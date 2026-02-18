param(
  [string]$Python = "D:\exe\environment\anaconda\envs\Aliyun39\python.exe",
  [string]$OutPath = ""
)

$Root = Resolve-Path "$PSScriptRoot\.."
if (-not $OutPath) {
  $OutPath = Join-Path $Root "docs\step5_import_matrix.json"
}

if (-not (Test-Path $Python)) {
  Write-Error "Python not found: $Python"
  exit 2
}

$py = @'
import json
import importlib
import sys

modules = [
    "numpy",
    "cv2",
    "torch",
    "torchvision",
    "torchaudio",
    "transformers",
    "ultralytics",
    "supervision",
    "easyocr",
]

result = {"python_executable": sys.executable, "modules": {}}
for name in modules:
    try:
        mod = importlib.import_module(name)
        result["modules"][name] = {
            "ok": True,
            "version": getattr(mod, "__version__", None),
        }
    except Exception as exc:
        result["modules"][name] = {
            "ok": False,
            "error": f"{type(exc).__name__}: {exc}",
        }

print(json.dumps(result))
'@

$py | & $Python - | Set-Content -Path $OutPath -Encoding UTF8
Write-Host "import_matrix_written=$OutPath"
