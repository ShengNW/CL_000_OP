# AW + OmniParser MCP (Rust-first skeleton)

## 1. Project定位
Rust 控制面 + Python mock sidecar 的最小可运行骨架。Rust 负责 AW 访问、触发与 nowframe 生成；Python sidecar 仅提供标准库 HTTP mock，后续可替换为真实 OmniParser。

## 2. 目录结构（F 盘布局）
固定根目录：`F:\aw-omni`

- `F:\aw-omni\runtime\logs`
- `F:\aw-omni\runtime\pid`
- `F:\aw-omni\data\aw_raw`
- `F:\aw-omni\data\episodes`
- `F:\aw-omni\data\nowframes`
- `F:\aw-omni\cache\screens`
- `F:\aw-omni\cache\thumbs`
- `F:\aw-omni\cache\tmp`
- `F:\aw-omni\models`
- `F:\aw-omni\release`
- `F:\aw-omni\src` (本仓库源码)

## 3. 启动步骤（Windows 优先）
1. 进入 `F:\aw-omni\src`
2. 启动 Python sidecar entry（默认 mock，不安装任何包）：
   - `D:\exe\environment\anaconda\envs\Aliyun39\python.exe F:\aw-omni\src\sidecar\omni_sidecar_entry.py --mode mock --host 127.0.0.1 --port 8000`
3. 启动 Rust 控制面：
   - `scripts\run_daemon_win.cmd health`
   - `scripts\run_mcp_win.cmd`

## 4. API / MCP 工具示例
- AW 探活：
  - `http://127.0.0.1:5600/api/0/info`
  - `http://127.0.0.1:5600/api/0/buckets`
- Sidecar mock：
  - `GET http://127.0.0.1:8000/probe`
  - `POST http://127.0.0.1:8000/parse`
- MCP stub (stdio JSON-RPC)：
  - `{"id":1,"method":"aw.get_state","params":{}}`
  - `{"id":2,"method":"nowframe.build","params":{"reason":"manual"}}`
  - `{"id":3,"method":"system.health","params":{}}`
  - `{"id":4,"method":"screen.capture","params":{"mode":"full","format":"png","with_cursor":false}}`
  - `{"id":5,"method":"screen.parse","params":{"frame_id":"frame_xxx"}}`
  - `{"id":6,"method":"screen.bundle","params":{"mode":"full","format":"png","with_cursor":false}}`
  - `{"id":7,"method":"resource.read","params":{"uri":"screen://latest/annotated"}}`

## 4.1 安全与网络
- MCP 默认走 stdio，本地仅限 `127.0.0.1` 侧的 AW/sidecar 访问。
- 可选鉴权：设置 `MCP_AUTH_TOKEN`，并在 `params.auth_token` 里携带同值。
- 不要直接公网暴露；如需远程访问，建议走 SSH 双跳隧道（示例，转发 sidecar 8000）：`ssh -J user@bastion user@vps -L 127.0.0.1:8000:127.0.0.1:8000`

## 5. 已知限制与下一步
- 仅为最小骨架，MCP 仅实现 JSON-RPC 风格 stub。
- Sidecar 为 mock，不涉及真实模型、GPU 或 OmniParser。
- 下一步：在不破坏现有 conda/torch 环境的前提下，引入真实 OmniParser sidecar，并评估依赖风险后再接入。
