# MCP Capability Matrix

Scope: `aw_omni_mcp` JSON-RPC over stdio. Status reflects **current code** in this workspace.

## Tools & Resources

| Capability | Type | Status | Notes |
| --- | --- | --- | --- |
| `aw.get_state` | Tool | Implemented | Reads AW `/api/0/info` and `/api/0/buckets`. |
| `nowframe.build` | Tool | Implemented | Aggregates AW info/buckets and sidecar `/probe`. |
| `system.health` | Tool | Implemented | Returns AW/sidecar health + protected env diff status. |
| `screen.capture` | Tool | Implemented | Captures `full` or `active` screen to `cache/screens`. |
| `screen.parse` | Tool | Implemented | Sends screenshot to sidecar `/parse`, stores SOM if provided. |
| `screen.bundle` | Tool | Implemented | Capture + parse + annotated/mask output. |
| `resource.read` | Tool | Implemented | Returns latest screen resources by URI. |
| `screen://latest/raw` | Resource | Implemented | Path to latest raw capture. |
| `screen://latest/annotated` | Resource | Implemented | Path to latest annotated image. |
| `screen://latest/mask` | Resource | Implemented | Path to latest mask image. |
| `screen://latest/json` | Resource | Implemented | Latest bundle JSON content + path. |

## Missing / Suggested Next

- `screen.list_windows` — enumerate windows for more controlled capture.
- `screen.capture_region` — accept explicit bounding box for partial captures.
- `aw.query_events` — time-range event queries for richer context.
- `nowframe.save` — persist NowFrames to disk with retention policy.
