# CL_000_OP — AW + OmniParser MCP

A Rust-first control plane that combines **ActivityWatch (AW)** context with **OmniParser** UI parsing. The system is designed to be safe-by-default, read-only against AW, and friendly to constrained Windows environments.

```mermaid
flowchart LR
  AW[ActivityWatch Server] -->|/api/0/info, /api/0/buckets| Daemon
  Sidecar[OmniParser Sidecar] -->|/probe, /parse| Daemon
  Daemon --> NowFrame[NowFrame JSON]
  MCP[MCP Stdio Stub] -->|aw.get_state / nowframe.build / system.health| Clients
```

## Project Positioning

- **Rust control plane**: health, triggers, NowFrame assembly, MCP stub.
- **Python sidecar**: mock (stdlib) or real OmniParser (optional).
- **No automatic CUDA/Torch changes**: all sensitive packages are protected.

## Quick Start (Windows)

### 1) Mock Sidecar (safe, no ML deps)

```powershell
# Start mock sidecar
F:\aw-omni\src\scripts\run_sidecar_entry_win.ps1 -Mode mock

# Start daemon
F:\aw-omni\src\scripts\run_daemon_win.ps1

# Start MCP (stdio JSON-RPC stub)
F:\aw-omni\src\scripts\run_mcp_win.ps1
```

### 2) Real Sidecar (OmniParser required)

- Add OmniParser as a submodule or external clone under `third_party/OmniParser`.
- Apply the local patch: `docs/patches/omniparser-local.patch`.
- Place weights under `F:\aw-omni\models\omniparser`.

```powershell
# Start real sidecar (untitled env example)
D:\exe\environment\anaconda\envs\untitled\python.exe F:\aw-omni\src\sidecar\omni_sidecar_entry.py `
  --mode real_local_untitled `
  --host 127.0.0.1 --port 8000 `
  --real-repo F:\aw-omni\src\third_party\OmniParser `
  --weights-root F:\aw-omni\models\omniparser
```

## Configuration

- `config/local.win.toml`
- `config/local.wsl.toml`

All paths are expected to be on **F:** to keep large artifacts out of system disks. The system will **not** modify Torch/CUDA/OpenCV stacks automatically.

## MCP Local Architecture (OP + Lively)

- **OP** (this repo): develop + build `aw_omni_mcp.exe`, register MCP locally.
- **Lively**: consume the MCP tool from Codex and call `screen.bundle`.

The MCP server supports both **Content-Length framing** and **line-json** inputs for compatibility. Output framing follows the input mode.

## Build aw_omni_mcp (Windows/WSL Hybrid)

```powershell
# Build on Windows toolchain from WSL
powershell.exe -NoProfile -Command "Set-Item Env:CARGO_TARGET_DIR 'F:\aw-omni\cache\aw_omni_target'; Set-Location 'F:\aw-omni\src'; & 'C:\Users\surface\.rustup\toolchains\stable-x86_64-pc-windows-msvc\bin\cargo.exe' build -p aw_omni_mcp"
```

## MCP Registration (Current Working Command)

```bash
codex mcp remove aw_omni_local || true
codex mcp add aw_omni_local -- /mnt/f/aw-omni/cache/aw_omni_target/debug/aw_omni_mcp.exe --config 'F:\aw-omni\src\config\local.win.toml'
```

## Lively Consumption Template

**Always set `format="png"`**.

```text
aw_omni_local.screen.bundle({"mode":"full","format":"png","with_cursor":false,"include_b64":true})
```

## screen.bundle Return Contract (Key Fields)

- `raw_path`, `annotated_path`, `mask_path`
- `raw_b64`, `annotated_b64`, `mask_b64`
- `raw_b64_len`, `annotated_b64_len`, `mask_b64_len`
- `elements`, `aw_context`, `frame_id`, `ts`, `latency_ms`, `has_text`, `has_icon`

## Protocol Documentation

- `docs/protocols/AW_PROTOCOL.md`
- `docs/protocols/OMNIPARSER_SIDECAR_PROTOCOL.md`
- `docs/protocols/MCP_TOOL_CONTRACT.md`
- `DATA_GOVERNANCE.md`

## Troubleshooting

- **NumPy / OpenCV conflicts**: use guarded install scripts in `scripts/`.
- **Weights missing**: check `F:\aw-omni\models\omniparser`.
- **Cache paths**: ensure `F:\aw-omni\cache` is writable.
- **Sidecar not ready**: call `/probe` and inspect `reason` / `missing_*` fields.
- **initialize response closed**: client/server framing mismatch; server accepts both line-json and Content-Length.
- **format_not_supported**: `screen.bundle` only supports `format="png"`.
- **CONTRACT_INCOMPLETE / truncated**: response too large; consumers should not rely on local paths to re-derive images.

## Next Step (OSS Object Storage)

Persist images to object storage and return **URLs + metadata** in MCP responses to avoid large base64 payloads.

## Safety & Stability

- This repo does **not** modify AW configuration.
- Torch/CUDA/OpenCV are treated as **protected** packages.
- Heavy artifacts (models/cache/runtime/logs) are excluded from version control.

## License

- This repository is licensed under MPL-2.0. See `LICENSE`.
- Third-party license references: `THIRD_PARTY_NOTICES.md`.
