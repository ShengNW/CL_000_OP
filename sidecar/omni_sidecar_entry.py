#!/usr/bin/env python3
import argparse
import json
import os
import sys
import time
import traceback
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Optional


def json_print(payload):
    sys.stdout.write(json.dumps(payload) + "\n")
    sys.stdout.flush()


def run_mock(args):
    # Defer to the lightweight stdlib mock server.
    if __package__ is None and os.path.dirname(__file__) not in sys.path:
        sys.path.append(os.path.dirname(__file__))
    from omni_sidecar_mock import main as mock_main

    sys.argv = [
        sys.argv[0],
        "--host",
        args.host,
        "--port",
        str(args.port),
    ]
    mock_main()


def check_real_prereq(args):
    missing = []
    repo = Path(args.real_repo) if args.real_repo else None
    if not repo or not repo.exists():
        missing.append(f"real_repo_missing:{args.real_repo}")
    else:
        server = repo / "omnitool" / "omniparserserver" / "omniparserserver.py"
        if not server.exists():
            missing.append(f"omniparserserver_missing:{server}")
        core = repo / "util" / "omniparser.py"
        if not core.exists():
            missing.append(f"omniparser_core_missing:{core}")

    weights = Path(args.weights_root) if args.weights_root else None
    if not weights or not weights.exists():
        missing.append(f"weights_root_missing:{args.weights_root}")

    return missing


def run_real(args):
    missing = check_real_prereq(args)
    if missing:
        json_print(
            {
                "ok": False,
                "mode": "real",
                "error": "missing_prerequisites",
                "details": missing,
            }
        )
        sys.exit(2)

    # Placeholder for real OmniParser server launch (not installed in this step).
    json_print(
        {
            "ok": False,
            "mode": "real",
            "error": "real_sidecar_not_installed",
            "details": [
                "Dependencies not installed. This step only wires entrypoint.",
                "Use isolated environment to install OmniParser deps before enabling real mode.",
            ],
        }
    )
    sys.exit(3)


def preflight_imports():
    required = [
        "torch",
        "torchvision",
        "cv2",
        "numpy",
        "PIL",
        "supervision",
        "transformers",
        "ultralytics",
        "easyocr",
        "matplotlib",
    ]
    optional = [
        "paddle",
        "paddleocr",
    ]

    missing = []
    versions = {}
    for name in required:
        try:
            module = __import__(name)
            versions[name] = getattr(module, "__version__", "")
        except Exception as exc:
            missing.append(f"{name}:{type(exc).__name__}")

    optional_missing = []
    for name in optional:
        try:
            module = __import__(name)
            versions[name] = getattr(module, "__version__", "")
        except Exception as exc:
            optional_missing.append(f"{name}:{type(exc).__name__}")

    return missing, optional_missing, versions


def json_response(handler, status, payload):
    data = json.dumps(payload).encode("utf-8")
    handler.send_response(status)
    handler.send_header("Content-Type", "application/json; charset=utf-8")
    handler.send_header("Content-Length", str(len(data)))
    handler.end_headers()
    handler.wfile.write(data)

def required_weight_files(weights_root: Path):
    return {
        "icon_detect/train_args.yaml": weights_root / "icon_detect" / "train_args.yaml",
        "icon_detect/model.pt": weights_root / "icon_detect" / "model.pt",
        "icon_caption/config.json": weights_root / "icon_caption_florence" / "config.json",
        "icon_caption/generation_config.json": weights_root / "icon_caption_florence" / "generation_config.json",
        "icon_caption/model.safetensors": weights_root / "icon_caption_florence" / "model.safetensors",
    }


def check_weight_files(weights_root: Path):
    required = required_weight_files(weights_root)
    missing = []
    for key, path in required.items():
        if not path.exists():
            missing.append(f"weight_missing:{key}")
    return missing, required


_REAL_STATE = {"omniparser": None, "init_error": None}


def load_omniparser(repo: Path, weights_root: Path):
    if _REAL_STATE["omniparser"] is not None:
        return _REAL_STATE["omniparser"], None

    if str(repo) not in sys.path:
        sys.path.insert(0, str(repo))
    try:
        from util.omniparser import Omniparser
    except Exception as exc:
        log_path = repo.parent.parent / "docs" / "step6_parse_error.log"
        log_parse_error(log_path, exc)
        _REAL_STATE["init_error"] = f"{type(exc).__name__}: {exc}"
        return None, _REAL_STATE["init_error"]

    config = {
        "som_model_path": str(weights_root / "icon_detect" / "model.pt"),
        "caption_model_name": "florence2",
        "caption_model_path": str(weights_root / "icon_caption_florence"),
        "BOX_TRESHOLD": 0.05,
    }
    try:
        _REAL_STATE["omniparser"] = Omniparser(config)
    except Exception as exc:
        log_path = repo.parent.parent / "docs" / "step6_parse_error.log"
        log_parse_error(log_path, exc)
        _REAL_STATE["init_error"] = f"{type(exc).__name__}: {exc}"
        return None, _REAL_STATE["init_error"]

    return _REAL_STATE["omniparser"], None


def apply_cache_env(weights_root: Optional[Path]):
    if not weights_root:
        return
    base_root = weights_root.parent.parent if len(weights_root.parents) >= 2 else None
    if not base_root:
        return
    cache_root = base_root / "cache"
    hf_cache = cache_root / "hf"
    tmp_cache = cache_root / "tmp"
    easyocr_cache = cache_root / "easyocr"
    paddlex_cache = cache_root / "paddlex"
    hf_cache.mkdir(parents=True, exist_ok=True)
    tmp_cache.mkdir(parents=True, exist_ok=True)
    easyocr_cache.mkdir(parents=True, exist_ok=True)
    paddlex_cache.mkdir(parents=True, exist_ok=True)

    os.environ.setdefault("HF_HOME", str(hf_cache))
    os.environ.setdefault("HUGGINGFACE_HUB_CACHE", str(hf_cache))
    os.environ.setdefault("TRANSFORMERS_CACHE", str(hf_cache))
    os.environ.setdefault("TMP", str(tmp_cache))
    os.environ.setdefault("TEMP", str(tmp_cache))
    os.environ.setdefault("EASYOCR_HOME", str(easyocr_cache))
    os.environ.setdefault("PADDLE_PDX_CACHE_HOME", str(paddlex_cache))


def log_parse_error(log_path: Path, exc: Exception):
    try:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with log_path.open("a", encoding="utf-8") as handle:
            handle.write("=" * 60 + "\n")
            handle.write(f"time={time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime())}\n")
            handle.write(f"error={type(exc).__name__}: {exc}\n")
            handle.write(traceback.format_exc())
            handle.write("\n")
    except Exception:
        return


def run_real_local(args, mode_name: str):
    weights_root = Path(args.weights_root) if args.weights_root else None
    apply_cache_env(weights_root)
    missing_imports, optional_missing, versions = preflight_imports()
    missing_files = check_real_prereq(args)
    missing_weights, required = check_weight_files(weights_root) if weights_root else ([], {})
    preflight_ok = len(missing_imports) == 0
    ready = preflight_ok and len(missing_files) == 0 and len(missing_weights) == 0
    reason = "ready"
    if missing_imports:
        reason = "imports_missing"
    elif any("real_repo_missing" in item or "omniparserserver_missing" in item for item in missing_files):
        reason = "repo_missing"
    elif any("weights_root_missing" in item for item in missing_files) or missing_weights:
        reason = "weights_missing"

    status_payload = {
        "ok": ready,
        "mode": mode_name,
        "preflight_ok": preflight_ok,
        "ready": ready,
        "reason": reason,
        "missing_imports": missing_imports,
        "optional_missing": optional_missing,
        "missing_files": missing_files,
        "missing_weights": missing_weights,
        "required_weights": {k: str(v) for k, v in required.items()},
        "versions": versions,
    }

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            if self.path in ("/probe", "/probe/"):
                json_response(self, 200, status_payload)
                return
            json_response(self, 404, {"ok": False, "error": "not found"})

        def do_POST(self):
            if self.path in ("/parse", "/parse/"):
                if not ready:
                    json_response(
                        self,
                        503,
                        {
                            "ok": False,
                            "error": "preflight_only",
                            "preflight_ok": preflight_ok,
                            "reason": reason,
                            "missing_imports": missing_imports,
                            "missing_files": missing_files,
                            "missing_weights": missing_weights,
                        },
                    )
                    return

                length = int(self.headers.get("Content-Length", "0"))
                body = self.rfile.read(length) if length else b"{}"
                try:
                    payload = json.loads(body.decode("utf-8"))
                except Exception:
                    json_response(self, 400, {"ok": False, "error": "invalid_json"})
                    return

                image_b64 = payload.get("base64_image") or payload.get("image_base64")
                if not image_b64:
                    json_response(self, 400, {"ok": False, "error": "missing_base64_image"})
                    return

                repo = Path(args.real_repo)
                omni, err = load_omniparser(repo, weights_root)
                if err:
                    json_response(self, 500, {"ok": False, "error": "init_failed", "details": err})
                    return

                start = time.time()
                try:
                    som_image_base64, parsed_content_list = omni.parse(image_b64)
                except Exception as exc:
                    log_path = repo.parent.parent / "docs" / "step6_parse_error.log"
                    log_parse_error(log_path, exc)
                    json_response(
                        self,
                        500,
                        {"ok": False, "error": "parse_failed", "details": f"{type(exc).__name__}: {exc}"},
                    )
                    return

                latency = time.time() - start
                json_response(
                    self,
                    200,
                    {
                        "ok": True,
                        "latency": latency,
                        "latency_ms": int(latency * 1000),
                        "parsed_content_list": parsed_content_list,
                        "som_image_base64": som_image_base64,
                    },
                )
                return
            json_response(self, 404, {"ok": False, "error": "not found"})

        def log_message(self, format, *args):
            return

    server = ThreadingHTTPServer((args.host, args.port), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


def run_real_local_aliyun39(args):
    run_real_local(args, "real_local_aliyun39")


def run_real_local_untitled(args):
    run_real_local(args, "real_local_untitled")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--mode",
        choices=["mock", "real", "real_local_aliyun39", "real_local_untitled"],
        default="mock",
    )
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8000)
    parser.add_argument("--real-repo", dest="real_repo", default="")
    parser.add_argument("--weights-root", dest="weights_root", default="")
    args = parser.parse_args()

    if args.mode == "mock":
        run_mock(args)
        return

    if args.mode == "real_local_aliyun39":
        run_real_local_aliyun39(args)
        return
    if args.mode == "real_local_untitled":
        run_real_local_untitled(args)
        return

    run_real(args)


if __name__ == "__main__":
    main()
