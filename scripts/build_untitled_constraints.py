#!/usr/bin/env python3
import json
import sys
from pathlib import Path


def is_cuda_related(name: str) -> bool:
    n = name.lower()
    if n.startswith("nvidia-"):
        return True
    if n.startswith("cuda"):
        return True
    if "cudnn" in n:
        return True
    if "tensorrt" in n:
        return True
    return False


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    in_path = root / "docs" / "step7_untitled_pre.json"
    out_path = root / "docs" / "step7_untitled_protected_constraints.txt"

    if "--in" in sys.argv:
        in_path = Path(sys.argv[sys.argv.index("--in") + 1])
    if "--out" in sys.argv:
        out_path = Path(sys.argv[sys.argv.index("--out") + 1])

    if not in_path.exists():
        print(f"missing snapshot: {in_path}", file=sys.stderr)
        return 2

    data = json.loads(in_path.read_text(encoding="utf-8-sig"))
    freeze = data.get("protected_freeze", [])

    constraints = []

    version_map = {
        "torch": data.get("torch_version"),
        "torchvision": data.get("torchvision_version"),
        "torchaudio": data.get("torchaudio_version"),
        "numpy": data.get("numpy_version"),
    }
    for name, version in version_map.items():
        if version:
            constraints.append(f"{name}=={version}")

    # Preserve OpenCV if present in freeze
    for line in freeze:
        name = ""
        if "==" in line:
            name = line.split("==", 1)[0].strip()
        elif " @" in line:
            name = line.split("@", 1)[0].strip()
        else:
            continue
        lname = name.lower()
        if lname in version_map:
            continue
        if lname in {"opencv-python", "opencv-python-headless"}:
            constraints.append(line.strip())
            continue
        if is_cuda_related(lname):
            constraints.append(line.strip())

    if not constraints:
        print("no protected packages found in snapshot", file=sys.stderr)
        return 3

    out_path.write_text("\n".join(constraints) + "\n", encoding="utf-8")
    print(f"constraints_written={out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
