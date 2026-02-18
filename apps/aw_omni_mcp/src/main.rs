use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result};
use aw_client::AwClient;
use chrono::Utc;
use clap::Parser;
use nowframe_core::NowFrame;
use omni_client::OmniClient;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Parser, Debug)]
#[command(name = "aw_omni_mcp")]
struct Cli {
    #[arg(long, default_value = "config/local.wsl.toml")]
    config: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    aw: EndpointConfig,
    omni: EndpointConfig,
    paths: PathsConfig,
    sidecar: Option<SidecarConfig>,
}

#[derive(Debug, Deserialize)]
struct EndpointConfig {
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct SidecarConfig {
    python: Option<String>,
    script: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PathsConfig {
    root: String,
    runtime_logs: String,
    runtime_pid: String,
    data_aw_raw: String,
    data_episodes: String,
    data_nowframes: String,
    cache_screens: String,
    cache_thumbs: String,
    cache_tmp: String,
    models: String,
    release: String,
    src: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = load_config(&cli.config)?;

    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());

    for line in stdin.lock().lines() {
        let line = line.unwrap_or_default();
        if line.trim().is_empty() {
            continue;
        }

        let response = handle_request(&cfg, &line);
        writeln!(stdout, "{}", response)?;
        stdout.flush()?;
    }

    Ok(())
}

fn load_config(path: &str) -> Result<Config> {
    let text = fs::read_to_string(Path::new(path))
        .with_context(|| format!("read config failed: {}", path))?;
    let cfg: Config =
        toml::from_str(&text).with_context(|| format!("parse toml failed: {}", path))?;
    Ok(cfg)
}

fn handle_request(cfg: &Config, line: &str) -> String {
    let parsed: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(err) => {
            return serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": Value::Null,
                "error": { "code": -32700, "message": format!("parse error: {}", err) }
            }))
            .unwrap_or_else(|_| "{\"error\":\"serialize_failed\"}".to_string());
        }
    };

    let id = parsed.get("id").cloned().unwrap_or(Value::Null);
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = parsed.get("params").cloned().unwrap_or(Value::Null);

    let result = match method {
        "aw.get_state" => aw_get_state(cfg),
        "nowframe.build" => nowframe_build(cfg, params),
        "system.health" => system_health(cfg),
        _ => Err(format!("unknown method: {}", method)),
    };

    let response = match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": value }),
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": { "code": -32601, "message": err }
        }),
    };

    serde_json::to_string(&response)
        .unwrap_or_else(|_| "{\"error\":\"serialize_failed\"}".to_string())
}

fn aw_get_state(cfg: &Config) -> Result<Value, String> {
    let client = AwClient::new(cfg.aw.base_url.clone());
    let info = client.get_info().map_err(|e| e.to_string())?;
    let buckets = client.get_buckets().map_err(|e| e.to_string())?;
    Ok(json!({
        "info": info,
        "buckets": buckets,
    }))
}

fn nowframe_build(cfg: &Config, params: Value) -> Result<Value, String> {
    let reason = params
        .get("reason")
        .and_then(|v| v.as_str())
        .unwrap_or("mcp")
        .to_string();

    let aw_client = AwClient::new(cfg.aw.base_url.clone());
    let omni_client = OmniClient::new(cfg.omni.base_url.clone());

    let nowframe = NowFrame {
        timestamp: Utc::now().to_rfc3339(),
        reason,
        aw_info: aw_client.get_info().ok(),
        aw_buckets: aw_client.get_buckets().ok(),
        omni_probe: omni_client.probe().ok(),
    };

    serde_json::to_value(&nowframe).map_err(|e| e.to_string())
}

fn system_health(cfg: &Config) -> Result<Value, String> {
    let aw_client = AwClient::new(cfg.aw.base_url.clone());
    let omni_client = OmniClient::new(cfg.omni.base_url.clone());

    let aw_ok = aw_client.get_info().is_ok();
    let omni_probe = omni_client.probe().ok();
    let sidecar_ready = omni_probe
        .as_ref()
        .and_then(|v| v.get("ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let preflight_ok = omni_probe
        .as_ref()
        .and_then(|v| v.get("preflight_ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(sidecar_ready);

    let (protected_env_ok, protected_diff_count) = protected_env_status(cfg);

    Ok(json!({
        "aw_ok": aw_ok,
        "sidecar_probe_status": sidecar_ready,
        "preflight_ok": preflight_ok,
        "protected_env_ok": protected_env_ok,
        "protected_diff_count": protected_diff_count,
        "omni_probe": omni_probe,
    }))
}

fn protected_env_status(cfg: &Config) -> (bool, i64) {
    let pre_path = format!("{}/docs/protect_pre.json", cfg.paths.src);
    let post_path = format!("{}/docs/protect_post.json", cfg.paths.src);
    let pre = load_json(&pre_path);
    let post = load_json(&post_path);
    if pre.is_none() || post.is_none() {
        return (false, -1);
    }
    let diff = compare_protect_snapshots(pre.as_ref().unwrap(), post.as_ref().unwrap());
    (diff == 0, diff)
}

fn load_json(path: &str) -> Option<Value> {
    let text = fs::read_to_string(Path::new(path)).ok()?;
    serde_json::from_str(&text).ok()
}

fn compare_protect_snapshots(pre: &Value, post: &Value) -> i64 {
    let keys = [
        "torch_version_cuda",
        "torch_file",
        "cv2_file",
        "protected_freeze",
    ];
    let mut diff = 0;
    for key in keys.iter() {
        if pre.get(key) != post.get(key) {
            diff += 1;
        }
    }
    diff
}
