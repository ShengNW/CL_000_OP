use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use aw_client::AwClient;
use chrono::Utc;
use clap::{Parser, Subcommand};
use nowframe_core::NowFrame;
use omni_client::OmniClient;
use serde::Deserialize;
use serde_json::{json, Value};
use trigger_engine::{score, TriggerInput};

#[derive(Parser, Debug)]
#[command(name = "aw_omni_daemon")]
struct Cli {
    #[arg(long, default_value = "config/local.wsl.toml")]
    config: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Health,
    BuildNowframe {
        #[arg(long)]
        reason: String,
    },
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

    match cli.command {
        Command::Health => {
            let report = health_report(&cfg);
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Command::BuildNowframe { reason } => {
            let nowframe = build_nowframe(&cfg, &reason);
            println!("{}", serde_json::to_string_pretty(&nowframe)?);
        }
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

fn health_report(cfg: &Config) -> Value {
    let aw_client = AwClient::new(cfg.aw.base_url.clone());
    let omni_client = OmniClient::new(cfg.omni.base_url.clone());

    let aw_info = match aw_client.get_info() {
        Ok(value) => json!({ "ok": true, "info": value }),
        Err(err) => json!({ "ok": false, "error": err.to_string() }),
    };

    let omni_probe = match omni_client.probe() {
        Ok(value) => json!({ "ok": true, "probe": value }),
        Err(err) => json!({ "ok": false, "error": err.to_string() }),
    };

    let sidecar_ready = omni_probe
        .get("ok")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let preflight_ok = omni_probe
        .get("probe")
        .and_then(|v| v.get("preflight_ok"))
        .and_then(|v| v.as_bool())
        .unwrap_or(sidecar_ready);

    let (protected_env_ok, protected_diff_count) = protected_env_status(cfg);

    json!({
        "aw": aw_info,
        "omni": omni_probe,
        "sidecar_probe_status": sidecar_ready,
        "preflight_ok": preflight_ok,
        "protected_env_ok": protected_env_ok,
        "protected_diff_count": protected_diff_count,
    })
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

fn build_nowframe(cfg: &Config, reason: &str) -> NowFrame {
    let aw_client = AwClient::new(cfg.aw.base_url.clone());
    let omni_client = OmniClient::new(cfg.omni.base_url.clone());

    let aw_info = aw_client.get_info().ok();
    let aw_buckets = aw_client.get_buckets().ok();
    let omni_probe = omni_client.probe().ok();

    let trigger_input = TriggerInput {
        reason: reason.to_string(),
        hint: Some(0.6),
    };
    let _score = score(&trigger_input);

    NowFrame {
        timestamp: Utc::now().to_rfc3339(),
        reason: reason.to_string(),
        aw_info,
        aw_buckets,
        omni_probe,
    }
}
