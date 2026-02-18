# Third-Party Notices

This project integrates with third-party software and APIs. The following notices summarize licenses and the relationship to this repository.

## ActivityWatch

- License: MPL-2.0
- Relationship: external local service accessed over HTTP (`/api/0`).
- Source: ActivityWatch repository license.

## OmniParser

- License: CC-BY-4.0 per LICENSE file. The README currently shows an MIT badge; verify before redistribution if you need a definitive statement.
- Model weights: per OmniParser README, `icon_detect` is AGPL-3.0 and `icon_caption` weights are MIT.
- Relationship: optional external dependency; **not vendored** in this repo. Use `third_party/README.md` to add as submodule or clone.

## Key Rust Dependencies

These crates are pulled via Cargo and are **not vendored** here. Licenses are documented in their upstream repositories.

- `serde` — MIT OR Apache-2.0
- `serde_json` — MIT OR Apache-2.0
- `reqwest` — MIT OR Apache-2.0
- `tokio` — MIT
- `clap` — MIT OR Apache-2.0

## Sources

- ActivityWatch: https://github.com/ActivityWatch/activitywatch
- OmniParser: https://github.com/microsoft/OmniParser
- serde: https://github.com/serde-rs/serde
- serde_json: https://github.com/serde-rs/json
- reqwest: https://github.com/seanmonstar/reqwest
- tokio: https://github.com/tokio-rs/tokio
- clap: https://github.com/clap-rs/clap
