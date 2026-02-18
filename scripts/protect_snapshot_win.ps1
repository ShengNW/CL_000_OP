param(
  [string]$Stage = "pre",
  [string]$Python = "D:\exe\environment\anaconda\envs\Aliyun39\python.exe",
  [string]$OutPath = ""
)

$Root = Resolve-Path "$PSScriptRoot\.."
if (-not $OutPath) {
  $OutPath = Join-Path $Root ("docs\protect_{0}.json" -f $Stage)
}

if (-not (Test-Path $Python)) {
  Write-Error "Python not found: $Python"
  exit 2
}

$py = @'
import json
import subprocess
import sys
import io
import contextlib
from datetime import datetime

def safe_import(name):
    try:
        buf_out = io.StringIO()
        buf_err = io.StringIO()
        with contextlib.redirect_stdout(buf_out), contextlib.redirect_stderr(buf_err):
            mod = __import__(name)
        return mod, None
    except Exception as exc:
        return None, f"{type(exc).__name__}: {exc}"

now = datetime.utcnow().isoformat() + "Z"

torch_mod, torch_err = safe_import("torch")
cv2_mod, cv2_err = safe_import("cv2")
numpy_mod, numpy_err = safe_import("numpy")
torchvision_mod, torchvision_err = safe_import("torchvision")
torchaudio_mod, torchaudio_err = safe_import("torchaudio")

try:
    freeze = subprocess.check_output([sys.executable, "-m", "pip", "freeze"], text=True)
except Exception as exc:
    freeze = ""

protected = []
for line in freeze.splitlines():
    l = line.lower()
    if l.startswith(("torch==", "torchvision==", "torchaudio==", "opencv-python==", "opencv-python-headless==")):
        protected.append(line)
        continue
    if any(x in l for x in ("nvidia", "cuda", "cudnn", "tensorrt")):
        protected.append(line)

payload = {
    "timestamp": now,
    "python_executable": sys.executable,
    "torch_version": getattr(torch_mod, "__version__", None) if torch_mod else None,
    "torch_version_cuda": getattr(getattr(torch_mod, "version", None), "cuda", None) if torch_mod else None,
    "torch_cuda_available": getattr(getattr(torch_mod, "cuda", None), "is_available", lambda: None)() if torch_mod else None,
    "torch_file": getattr(torch_mod, "__file__", None) if torch_mod else None,
    "torchvision_version": getattr(torchvision_mod, "__version__", None) if torchvision_mod else None,
    "torchaudio_version": getattr(torchaudio_mod, "__version__", None) if torchaudio_mod else None,
    "numpy_version": getattr(numpy_mod, "__version__", None) if numpy_mod else None,
    "numpy_file": getattr(numpy_mod, "__file__", None) if numpy_mod else None,
    "cv2_file": getattr(cv2_mod, "__file__", None) if cv2_mod else None,
    "protected_freeze": protected,
    "import_errors": {
        "torch": torch_err,
        "torchvision": torchvision_err,
        "torchaudio": torchaudio_err,
        "cv2": cv2_err,
        "numpy": numpy_err,
    },
}

print(json.dumps(payload))
'@

$py | & $Python - | Set-Content -Path $OutPath -Encoding UTF8
Write-Host "snapshot_written=$OutPath"
