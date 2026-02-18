# Data Governance

This project processes user activity context. The policy below is designed for minimal retention, privacy-by-default, and predictable disk usage.

```mermaid
flowchart TB
  AW[AW Buckets (raw events)] --> RAW[raw events snapshot]
  RAW --> NOW[NowFrame]
  NOW --> EP[Episode]
  EP --> ARCH[Optional Export]
```

## Data Layers

1) **Raw events** (read-only from AW)
- Source: ActivityWatch buckets.
- Stored only if explicitly enabled.

2) **NowFrame** (short-lived, structured snapshot)
- Aggregates AW state + sidecar probe + parsed UI elements.
- Used for immediate reasoning and system health.

3) **Episode** (session-scale aggregation)
- Derived from multiple NowFrames.
- Should be opt-in with clear user consent.

## Privacy & Minimization

- Treat all UI content as **sensitive**.
- Store only what is required for short-term reasoning.
- Prefer hashing/redaction of titles, text, and URLs.
- Provide a clear opt-out for any long-term storage.

## Retention & Deletion

- Raw events: default retention **0** (not stored).
- NowFrames: keep a rolling window (e.g., last 24 hours).
- Episodes: opt-in, time-bounded, and user-controlled.

## Disk Strategy (F:\ preferred)

- Cache and runtime output live under `F:\aw-omni`.
- Suggested limits:
  - `cache/`: 5–8 GB soft cap
  - `runtime/logs`: 250 MB rolling
  - `data/`: project-specific cap; default 2 GB
- Prefer LRU or time-based eviction.

## Compliance Notes

- This repo does **not** collect or transmit data by itself.
- Any deployment should document data handling and local retention policy.
