#!/usr/bin/env python3
import json
import sys
from pathlib import Path


def is_protected(name: str) -> bool:
    n = name.lower()
    if n in {
        "torch",
        "torchvision",
        "torchaudio",
        "opencv-python",
        "opencv-python-headless",
    }:
        return True
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
    pre_path = root / "docs" / "protect_pre.json"
    out_path = root / "docs" / "protected_constraints.txt"

    if not pre_path.exists():
        print(f"missing snapshot: {pre_path}", file=sys.stderr)
        return 2

    data = json.loads(pre_path.read_text(encoding="utf-8-sig"))
    freeze = data.get("protected_freeze", [])

    constraints = []
    version_map = {
        "torch": data.get("torch_version"),
        "torchvision": data.get("torchvision_version"),
        "torchaudio": data.get("torchaudio_version"),
    }
    for name, version in version_map.items():
        if version:
            constraints.append(f"{name}=={version}")

    for line in freeze:
        name = ""
        if "==" in line:
            name = line.split("==", 1)[0].strip()
        elif " @" in line:
            name = line.split("@", 1)[0].strip()
        else:
            continue
        if name.lower() in version_map:
            continue
        if is_protected(name):
            constraints.append(line.strip())

    if not constraints:
        print("no protected packages found in snapshot", file=sys.stderr)
        return 3

    out_path.write_text("\n".join(constraints) + "\n", encoding="utf-8")
    print(f"constraints_written={out_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
