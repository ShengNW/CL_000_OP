# Step3 OmniParser Risk Report (No-Install Gate)

Generated: 2026-02-17
Scope: OmniParser real sidecar integration preflight, **no installation**.

## 1) Sources (Read-Only)
- Repo: `/tmp/omniparser_repo` (cloned from https://github.com/microsoft/OmniParser)
- Requirements: `requirements.txt`
- Server entry: `omnitool/omniparserserver/omniparserserver.py`
- pip: `20.0.2` (no `--dry-run` support)

## 2) OmniParser Server Interface (from repo)
- Base URL: `http://<host>:<port>`
- `GET /probe/` -> `{"message": "Omniparser API ready"}`
- `POST /parse/` body: `{ "base64_image": "..." }`
- `POST /parse/` response: `{ "som_image_base64": "...", "parsed_content_list": [...], "latency": <seconds> }`

Compatibility note: our mock now supports `/probe` and `/probe/`, `/parse` and `/parse/`, and returns `latency` + `parsed_content_list` entries with `content` fields to align with OmniParser client formatting.

## 3) Dependency Risk Matrix

### 3.1 Critical GPU stack (High risk)
- `torch`
- `torchvision`
- (implicit) `torchaudio` in many stacks
- `paddlepaddle`, `paddleocr` (own GPU stack)
- `ultralytics`, `transformers`, `timm`, `accelerate` (may drive CUDA/torch upgrades)

**Risk:** any real install is very likely to resolve or upgrade CUDA/torch-related packages. This conflicts with fixed Aliyun39 stack (torch 1.12.0+cu113, CUDA 11.3) and is considered a BLOCKER for in-place install.

### 3.2 Heavy frameworks / GUI / web (Medium risk)
- `gradio`, `streamlit`, `fastapi`, `uvicorn`
- `opencv-python`, `opencv-python-headless`
- `pyautogui`, `uiautomation`

**Risk:** extra binary wheels and conflicting transitive dependencies. Not safe on the fixed environment.

### 3.3 Model ecosystems (Medium / High)
- `transformers`, `timm`, `easyocr`
- `paddlepaddle`, `paddleocr`

**Risk:** large downloads and GPU requirements. Disk pressure on F:.

## 4) Aliyun39 Known Baseline
- torch: `1.12.0+cu113`
- cuda: `11.3`
- numpy: `1.26.4`
- opencv: `4.11.0`

## 5) Gate Decision
**Result: BLOCK**
Reason: OmniParser requirements include `torch`, `torchvision`, `paddlepaddle`, `paddleocr`, and other ML stacks. Any installation is highly likely to change CUDA/torch dependencies, violating the fixed Aliyun39 constraints.

## 6) Allowed Next Actions (No-Install)
- Implement sidecar entrypoint with mock/real mode switching.
- Add prereq checks and clear failure messages for real mode.
- Build isolated environment plan (separate venv/conda or container) for real installation.

## 7) Recommendation
Proceed only with **isolated environment** installation (separate from Aliyun39), then point sidecar to that environment. Do not install into Aliyun39.
