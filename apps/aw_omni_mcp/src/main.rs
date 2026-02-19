use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use aw_client::AwClient;
use base64::engine::general_purpose::STANDARD as BASE64_ENGINE;
use base64::Engine;
use chrono::Utc;
use clap::Parser;
use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_rect_mut};
use imageproc::rect::Rect;
use nowframe_core::NowFrame;
use omni_client::OmniClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
#[cfg(windows)]
use xcap::{Monitor, Window};

static FRAME_COUNTER: AtomicU64 = AtomicU64::new(0);
static LATEST_BUNDLE: OnceLock<Mutex<Option<LatestBundle>>> = OnceLock::new();
const MAX_IMAGE_BYTES: u64 = 6 * 1024 * 1024;
static LOG_FILE: OnceLock<Mutex<Option<fs::File>>> = OnceLock::new();

#[derive(Copy, Clone, Debug)]
enum WireMode {
    ContentLength,
    LineJson,
}

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

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LatestBundle {
    frame_id: String,
    ts: String,
    raw_path: String,
    annotated_path: Option<String>,
    mask_path: Option<String>,
    json_path: Option<String>,
    som_path: Option<String>,
}

#[derive(Debug)]
struct CaptureMeta {
    frame_id: String,
    ts: String,
    raw_path: PathBuf,
    width: u32,
    height: u32,
    cursor_included: bool,
}

#[derive(Debug)]
struct ParseMeta {
    frame_id: String,
    raw_path: PathBuf,
    elements: Vec<Value>,
    latency_ms: i64,
    has_text: bool,
    has_icon: bool,
    som_path: Option<PathBuf>,
    response: Value,
}

fn main() -> Result<()> {
    init_log();
    log_line(&format!("startup_ts={}", Utc::now().to_rfc3339()));
    log_line(&format!("argv={:?}", env::args().collect::<Vec<_>>()));

    let cli = Cli::parse();
    log_line(&format!("config_path={}", cli.config));
    let cfg = match load_config(&cli.config) {
        Ok(cfg) => {
            log_line("config_loaded=ok");
            cfg
        }
        Err(err) => {
            log_line(&format!("config_loaded=err: {}", err));
            return Err(err);
        }
    };

    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin.lock());
    let stdout = io::stdout();
    let mut writer = io::BufWriter::new(stdout.lock());

    let mut shutdown_requested = false;

    loop {
        let (frame, wire_mode) = match read_frame(&mut reader) {
            Ok(Some((bytes, mode))) => (bytes, mode),
            Ok(None) => break,
            Err(err) => {
                log_line(&format!("read_frame_io_error={}", err));
                break;
            }
        };

        if frame.is_empty() {
            continue;
        }

        let parsed: Value = match serde_json::from_slice(&frame) {
            Ok(value) => value,
            Err(err) => {
                log_line(&format!("parse_error={}", err));
                let response = error_response(Value::Null, -32700, &format!("parse error: {}", err));
                write_response(&mut writer, &response, wire_mode)?;
                continue;
            }
        };

        let id = parsed.get("id").cloned().unwrap_or(Value::Null);
        let is_notification = parsed.get("id").is_none();

        if let Err(message) = validate_auth(&parsed) {
            if !is_notification {
                let response = error_response(id, -32001, &message);
                write_response(&mut writer, &response, wire_mode)?;
            }
            continue;
        }

        let outcome = dispatch_request(&cfg, &parsed);
        if let Some(response) = outcome.response {
            write_response(&mut writer, &response, wire_mode)?;
        }
        if outcome.shutdown {
            shutdown_requested = true;
        }
        if outcome.exit {
            break;
        }
        if shutdown_requested {
            // stay alive until explicit exit
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

fn validate_auth(parsed: &Value) -> Result<(), String> {
    let expected = match env::var("MCP_AUTH_TOKEN") {
        Ok(value) if !value.trim().is_empty() => value,
        _ => return Ok(()),
    };

    let provided = parsed
        .get("auth_token")
        .and_then(|v| v.as_str())
        .or_else(|| {
            parsed
                .get("params")
                .and_then(|p| p.get("auth_token"))
                .and_then(|v| v.as_str())
        });

    if provided == Some(expected.as_str()) {
        Ok(())
    } else {
        Err("unauthorized".to_string())
    }
}

fn tool_definition() -> Value {
    json!({
        "name": "screen.bundle",
        "description": "Capture, parse, and annotate the screen",
        "inputSchema": {
            "type": "object",
            "properties": {
                "mode": { "type": "string" },
                "format": { "type": "string", "enum": ["png"] },
                "with_cursor": { "type": "boolean" },
                "include_b64": { "type": "boolean" }
            },
            "required": []
        }
    })
}

fn init_log() {
    let path = env::var("MCP_LOG_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "F:\\aw-omni\\cache\\logs\\aw_omni_mcp.log".to_string());
    if let Some(parent) = Path::new(&path).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .ok();
    let lock = LOG_FILE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = lock.lock() {
        *guard = file;
    }
}

fn log_line(message: &str) {
    let lock = match LOG_FILE.get() {
        Some(lock) => lock,
        None => return,
    };
    if let Ok(mut guard) = lock.lock() {
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "[{}] {}", Utc::now().to_rfc3339(), message);
            let _ = file.flush();
        }
    }
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
}

fn result_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

struct DispatchOutcome {
    response: Option<Value>,
    shutdown: bool,
    exit: bool,
}

fn dispatch_request(cfg: &Config, parsed: &Value) -> DispatchOutcome {
    let id_opt = parsed.get("id").cloned();
    let id = id_opt.clone().unwrap_or(Value::Null);
    let is_notification = id_opt.is_none();
    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let params = parsed.get("params").cloned().unwrap_or(Value::Null);
    let params_size = serde_json::to_vec(&params).map(|v| v.len()).unwrap_or(0);
    log_line(&format!(
        "dispatch method={} id_present={} params_size={}",
        method,
        !is_notification,
        params_size
    ));

    match method {
        "notifications/initialized" => DispatchOutcome {
            response: None,
            shutdown: false,
            exit: false,
        },
        "ping" => DispatchOutcome {
            response: if is_notification {
                None
            } else {
                Some(result_response(id, json!({})))
            },
            shutdown: false,
            exit: false,
        },
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": { "name": "aw_omni_mcp", "version": "0.1.0" },
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "listChanged": false }
                }
            });
            DispatchOutcome {
                response: if is_notification {
                    None
                } else {
                    Some(result_response(id, result))
                },
                shutdown: false,
                exit: false,
            }
        }
        "shutdown" => DispatchOutcome {
            response: if is_notification {
                None
            } else {
                Some(result_response(id, Value::Null))
            },
            shutdown: true,
            exit: false,
        },
        "exit" => DispatchOutcome {
            response: if is_notification {
                None
            } else {
                Some(result_response(id, Value::Null))
            },
            shutdown: false,
            exit: true,
        },
        "tools/list" => {
            let result = json!({ "tools": [tool_definition()] });
            DispatchOutcome {
                response: if is_notification {
                    None
                } else {
                    Some(result_response(id, result))
                },
                shutdown: false,
                exit: false,
            }
        }
        "tools/call" => {
            if is_notification {
                return DispatchOutcome {
                    response: None,
                    shutdown: false,
                    exit: false,
                };
            }
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(Value::Null);
            if name != "screen.bundle" {
                return DispatchOutcome {
                    response: Some(error_response(id, -32601, "unknown tool")),
                    shutdown: false,
                    exit: false,
                };
            }
            let result = match screen_bundle(cfg, args) {
                Ok(value) => {
                    let text = serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string());
                    Ok(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": false
                    }))
                }
                Err(err) => Err(err),
            };
            DispatchOutcome {
                response: Some(match result {
                    Ok(value) => result_response(id, value),
                    Err(err) => error_response(id, -32000, &err),
                }),
                shutdown: false,
                exit: false,
            }
        }
        "resources/read" | "resource.read" => {
            if is_notification {
                return DispatchOutcome {
                    response: None,
                    shutdown: false,
                    exit: false,
                };
            }
            let result = match resource_read(cfg, params) {
                Ok(value) => result_response(id, value),
                Err(err) => error_response(id, -32000, &err),
            };
            DispatchOutcome {
                response: Some(result),
                shutdown: false,
                exit: false,
            }
        }
        // legacy JSON-RPC methods
        "aw.get_state" => wrap_legacy_result(id, is_notification, aw_get_state(cfg)),
        "nowframe.build" => wrap_legacy_result(id, is_notification, nowframe_build(cfg, params)),
        "system.health" => wrap_legacy_result(id, is_notification, system_health(cfg)),
        "screen.capture" => wrap_legacy_result(id, is_notification, screen_capture(cfg, params)),
        "screen.parse" => wrap_legacy_result(id, is_notification, screen_parse(cfg, params)),
        "screen.bundle" => wrap_legacy_result(id, is_notification, screen_bundle(cfg, params)),
        _ => DispatchOutcome {
            response: if is_notification {
                None
            } else {
                Some(error_response(id, -32601, &format!("unknown method: {}", method)))
            },
            shutdown: false,
            exit: false,
        },
    }
}

fn wrap_legacy_result(
    id: Value,
    is_notification: bool,
    result: Result<Value, String>,
) -> DispatchOutcome {
    if is_notification {
        return DispatchOutcome {
            response: None,
            shutdown: false,
            exit: false,
        };
    }
    let response = match result {
        Ok(value) => result_response(id, value),
        Err(err) => error_response(id, -32000, &err),
    };
    DispatchOutcome {
        response: Some(response),
        shutdown: false,
        exit: false,
    }
}

fn read_frame<R: BufRead>(reader: &mut R) -> io::Result<Option<(Vec<u8>, WireMode)>> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = match reader.read_line(&mut line) {
            Ok(n) => n,
            Err(err) => {
                log_line(&format!("read_frame read_line_err={}", err));
                return Err(err);
            }
        };
        if n == 0 {
            return Ok(None);
        }
        if line.trim().is_empty() {
            continue;
        }
        let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
        let lower = trimmed.to_ascii_lowercase();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            log_line(&format!("read_frame line_json len={}", trimmed.len()));
            return Ok(Some((trimmed.as_bytes().to_vec(), WireMode::LineJson)));
        }
        if lower.starts_with("content-length:") || lower.starts_with("content-type:") {
            log_line(&format!("read_frame header: {}", trimmed));
            let mut headers = vec![trimmed.to_string()];
            loop {
                line.clear();
                let n = match reader.read_line(&mut line) {
                    Ok(n) => n,
                    Err(err) => {
                        log_line(&format!("read_frame header_read_err={}", err));
                        return Err(err);
                    }
                };
                if n == 0 {
                    log_line("read_frame eof_in_headers");
                    return Ok(None);
                }
                if line.trim().is_empty() {
                    break;
                }
                let header_line = line.trim_end().to_string();
                log_line(&format!("read_frame header: {}", header_line));
                headers.push(header_line);
            }
            let mut content_length: Option<usize> = None;
            for header in headers {
                let header_lower = header.to_ascii_lowercase();
                if header_lower.starts_with("content-length:") {
                    let value = header
                        .splitn(2, ':')
                        .nth(1)
                        .unwrap_or("")
                        .trim();
                    match value.parse::<usize>() {
                        Ok(len) => content_length = Some(len),
                        Err(err) => {
                            log_line(&format!(
                                "read_frame content_length_parse_err={} value={}",
                                err, value
                            ));
                        }
                    }
                }
            }
            if let Some(len) = content_length {
                log_line(&format!("read_frame content_length={}", len));
                if len == 0 {
                    log_line("read_frame content_length=0");
                    continue;
                }
                let mut buf = vec![0u8; len];
                if let Err(err) = reader.read_exact(&mut buf) {
                    log_line(&format!("read_frame read_exact_err={}", err));
                    continue;
                }
                log_line(&format!("read_frame bytes_read={}", buf.len()));
                return Ok(Some((buf, WireMode::ContentLength)));
            } else {
                log_line("read_frame error=missing_content_length");
                continue;
            }
        }
        // ignore unknown header line
    }
}

fn write_response<W: Write>(writer: &mut W, response: &Value, mode: WireMode) -> io::Result<()> {
    let data = serde_json::to_vec(response).unwrap_or_else(|_| b"{}".to_vec());
    log_line(&format!("write_response mode={:?} bytes={}", mode, data.len()));
    match mode {
        WireMode::ContentLength => {
            write!(writer, "Content-Length: {}\r\n\r\n", data.len())?;
            writer.write_all(&data)?;
            writer.flush()?;
        }
        WireMode::LineJson => {
            writer.write_all(&data)?;
            writer.write_all(b"\n")?;
            writer.flush()?;
        }
    }
    Ok(())
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

fn screen_capture(cfg: &Config, params: Value) -> Result<Value, String> {
    let mode = params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("full");
    let format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("png");
    let with_cursor = params
        .get("with_cursor")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let capture = capture_screen_internal(cfg, mode, format, with_cursor)?;

    Ok(json!({
        "frame_id": capture.frame_id,
        "ts": capture.ts,
        "raw_path": path_to_string(&capture.raw_path),
        "width": capture.width,
        "height": capture.height,
        "cursor_included": capture.cursor_included,
    }))
}

fn screen_parse(cfg: &Config, params: Value) -> Result<Value, String> {
    let parse_options = params.get("parse_options").cloned();
    let (frame_id, raw_path) = resolve_frame_input(cfg, &params)?;

    let parse = parse_screen_internal(cfg, frame_id, &raw_path, parse_options)?;
    let json_path = write_parse_json(cfg, &parse)?;

    Ok(json!({
        "frame_id": parse.frame_id,
        "raw_path": path_to_string(&parse.raw_path),
        "elements": parse.elements,
        "latency_ms": parse.latency_ms,
        "has_text": parse.has_text,
        "has_icon": parse.has_icon,
        "som_path": parse.som_path.as_ref().map(|p| path_to_string(p)),
        "json_path": path_to_string(&json_path),
    }))
}

fn screen_bundle(cfg: &Config, params: Value) -> Result<Value, String> {
    let mode = params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("full");
    let format = params
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("png");
    let with_cursor = params
        .get("with_cursor")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let include_b64 = params
        .get("include_b64")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let parse_options = params.get("parse_options").cloned();

    let capture = capture_screen_internal(cfg, mode, format, with_cursor)?;
    let parse = parse_screen_internal(cfg, Some(capture.frame_id.clone()), &capture.raw_path, parse_options)?;

    let (annotated_path, mask_path) = build_annotations(cfg, &capture.raw_path, capture.width, capture.height, &parse.elements, &capture.frame_id)?;

    let aw_context = aw_context_json(cfg);
    let bundle_json = json!({
        "frame_id": capture.frame_id.clone(),
        "ts": capture.ts.clone(),
        "raw_path": path_to_string(&capture.raw_path),
        "annotated_path": path_to_string(&annotated_path),
        "mask_path": path_to_string(&mask_path),
        "elements": parse.elements.clone(),
        "latency_ms": parse.latency_ms,
        "has_text": parse.has_text,
        "has_icon": parse.has_icon,
        "som_path": parse.som_path.as_ref().map(|p| path_to_string(p)),
        "aw_context": aw_context,
    });

    let json_path = write_bundle_json(cfg, &bundle_json, &capture.frame_id)?;

    update_latest_bundle(
        cfg,
        LatestBundle {
            frame_id: capture.frame_id.clone(),
            ts: capture.ts.clone(),
            raw_path: path_to_string(&capture.raw_path),
            annotated_path: Some(path_to_string(&annotated_path)),
            mask_path: Some(path_to_string(&mask_path)),
            json_path: Some(path_to_string(&json_path)),
            som_path: parse.som_path.as_ref().map(|p| path_to_string(p)),
        },
    )?;

    if include_b64 {
        let raw_b64 = match encode_base64_with_limit(&capture.raw_path) {
            Ok(value) => value,
            Err(err) => return Err(format!("missing_b64: raw_b64 ({})", err)),
        };
        let annotated_b64 = match encode_base64_with_limit(&annotated_path) {
            Ok(value) => value,
            Err(err) => return Err(format!("missing_b64: annotated_b64 ({})", err)),
        };
        let mask_b64 = match encode_base64_with_limit(&mask_path) {
            Ok(value) => value,
            Err(err) => return Err(format!("missing_b64: mask_b64 ({})", err)),
        };
        let mut response_json = bundle_json.clone();
        if let Value::Object(map) = &mut response_json {
            map.insert("raw_b64".to_string(), Value::String(raw_b64));
            map.insert("annotated_b64".to_string(), Value::String(annotated_b64));
            map.insert("mask_b64".to_string(), Value::String(mask_b64));
            map.insert(
                "raw_b64_len".to_string(),
                Value::Number(serde_json::Number::from(
                    map.get("raw_b64")
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0) as u64,
                )),
            );
            map.insert(
                "annotated_b64_len".to_string(),
                Value::Number(serde_json::Number::from(
                    map.get("annotated_b64")
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0) as u64,
                )),
            );
            map.insert(
                "mask_b64_len".to_string(),
                Value::Number(serde_json::Number::from(
                    map.get("mask_b64")
                        .and_then(|v| v.as_str())
                        .map(|s| s.len())
                        .unwrap_or(0) as u64,
                )),
            );
        }
        Ok(response_json)
    } else {
        Ok(bundle_json)
    }
}

fn resource_read(cfg: &Config, params: Value) -> Result<Value, String> {
    let uri = params
        .get("uri")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing uri".to_string())?;

    let latest = load_latest_bundle(cfg).ok_or_else(|| "latest bundle not found".to_string())?;

    let path = match uri {
        "screen://latest/raw" => latest.raw_path,
        "screen://latest/annotated" => latest
            .annotated_path
            .ok_or_else(|| "annotated image not available".to_string())?,
        "screen://latest/mask" => latest
            .mask_path
            .ok_or_else(|| "mask image not available".to_string())?,
        _ => return Err(format!("unknown resource uri: {}", uri)),
    };

    let blob = encode_base64_with_limit(Path::new(&path))?;
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "image/png",
            "blob": blob
        }]
    }))
}

fn capture_screen_internal(
    cfg: &Config,
    mode: &str,
    format: &str,
    with_cursor: bool,
) -> Result<CaptureMeta, String> {
    if format != "png" {
        return Err("format_not_supported".to_string());
    }

    let cache_dir = PathBuf::from(&cfg.paths.cache_screens);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("create cache dir failed: {}", e))?;

    let frame_id = new_frame_id();
    let ts = Utc::now().to_rfc3339();
    let raw_path = cache_dir.join(format!("{}_raw.png", frame_id));

    let (image, cursor_included) = capture_image(mode, with_cursor)?;

    let width = image.width();
    let height = image.height();

    save_rgba_image(&image, &raw_path)?;

    Ok(CaptureMeta {
        frame_id,
        ts,
        raw_path,
        width,
        height,
        cursor_included,
    })
}

fn parse_screen_internal(
    cfg: &Config,
    frame_id: Option<String>,
    raw_path: &Path,
    parse_options: Option<Value>,
) -> Result<ParseMeta, String> {
    let cache_dir = PathBuf::from(&cfg.paths.cache_screens);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("create cache dir failed: {}", e))?;

    let frame_id = frame_id.unwrap_or_else(new_frame_id);
    let bytes = fs::read(raw_path).map_err(|e| format!("read image failed: {}", e))?;
    let encoded = BASE64_ENGINE.encode(bytes);

    let omni_client = OmniClient::new(cfg.omni.base_url.clone());
    let response = omni_client
        .parse(&encoded, parse_options.as_ref())
        .map_err(|e| e.to_string())?;

    let latency_ms = response
        .get("latency_ms")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let elements_value = response
        .get("parsed_content_list")
        .cloned()
        .unwrap_or_else(|| Value::Array(vec![]));
    let elements = elements_value.as_array().cloned().unwrap_or_default();

    let has_text = elements.iter().any(|el| element_has_kind(el, "text"));
    let has_icon = elements.iter().any(|el| element_has_kind(el, "icon"));

    let som_path = response
        .get("som_image_base64")
        .and_then(|v| v.as_str())
        .and_then(|s| decode_base64_image(s).ok())
        .and_then(|bytes| {
            let som_path = cache_dir.join(format!("{}_som.png", frame_id));
            if fs::write(&som_path, bytes).is_ok() {
                Some(som_path)
            } else {
                None
            }
        });

    Ok(ParseMeta {
        frame_id,
        raw_path: raw_path.to_path_buf(),
        elements,
        latency_ms,
        has_text,
        has_icon,
        som_path,
        response,
    })
}

fn build_annotations(
    cfg: &Config,
    raw_path: &Path,
    width: u32,
    height: u32,
    elements: &[Value],
    frame_id: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let cache_dir = PathBuf::from(&cfg.paths.cache_screens);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("create cache dir failed: {}", e))?;

    let mut image = image::open(raw_path)
        .map_err(|e| format!("open image failed: {}", e))?
        .to_rgba8();
    let mut mask = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));

    for el in elements {
        let rect = match extract_bbox(el, width, height) {
            Some(rect) => rect,
            None => continue,
        };
        let color = element_color(el);
        draw_hollow_rect_mut(&mut image, rect, color);
        draw_filled_rect_mut(&mut mask, rect, Rgba([255, 255, 255, 255]));
    }

    let annotated_path = cache_dir.join(format!("{}_annotated.png", frame_id));
    let mask_path = cache_dir.join(format!("{}_mask.png", frame_id));

    save_rgba_image(&image, &annotated_path)?;
    save_rgba_image(&mask, &mask_path)?;

    Ok((annotated_path, mask_path))
}

fn write_parse_json(cfg: &Config, parse: &ParseMeta) -> Result<PathBuf, String> {
    let cache_dir = PathBuf::from(&cfg.paths.cache_screens);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("create cache dir failed: {}", e))?;

    let path = cache_dir.join(format!("{}_parse.json", parse.frame_id));
    let payload = json!({
        "frame_id": parse.frame_id,
        "raw_path": path_to_string(&parse.raw_path),
        "latency_ms": parse.latency_ms,
        "has_text": parse.has_text,
        "has_icon": parse.has_icon,
        "elements": parse.elements,
        "som_path": parse.som_path.as_ref().map(|p| path_to_string(p)),
        "response": parse.response,
    });

    let text = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("serialize parse json failed: {}", e))?;
    fs::write(&path, text).map_err(|e| format!("write parse json failed: {}", e))?;
    Ok(path)
}

fn write_bundle_json(cfg: &Config, payload: &Value, frame_id: &str) -> Result<PathBuf, String> {
    let cache_dir = PathBuf::from(&cfg.paths.cache_screens);
    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("create cache dir failed: {}", e))?;

    let path = cache_dir.join(format!("{}_bundle.json", frame_id));
    let text = serde_json::to_string_pretty(payload)
        .map_err(|e| format!("serialize bundle json failed: {}", e))?;
    fs::write(&path, text).map_err(|e| format!("write bundle json failed: {}", e))?;
    Ok(path)
}

fn update_latest_bundle(cfg: &Config, latest: LatestBundle) -> Result<(), String> {
    let path = latest_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create cache dir failed: {}", e))?;
    }

    let text = serde_json::to_string_pretty(&latest)
        .map_err(|e| format!("serialize latest json failed: {}", e))?;
    fs::write(&path, text).map_err(|e| format!("write latest json failed: {}", e))?;

    let lock = LATEST_BUNDLE.get_or_init(|| Mutex::new(None));
    let mut guard = lock.lock().map_err(|_| "latest lock poisoned".to_string())?;
    *guard = Some(latest);
    Ok(())
}

fn load_latest_bundle(cfg: &Config) -> Option<LatestBundle> {
    let lock = LATEST_BUNDLE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = lock.lock() {
        if let Some(value) = guard.clone() {
            return Some(value);
        }
    }

    let path = latest_path(cfg);
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn latest_path(cfg: &Config) -> PathBuf {
    PathBuf::from(&cfg.paths.cache_screens).join("latest.json")
}

fn resolve_frame_input(cfg: &Config, params: &Value) -> Result<(Option<String>, PathBuf), String> {
    if let Some(raw_path) = params.get("raw_path").and_then(|v| v.as_str()) {
        return Ok((None, PathBuf::from(raw_path)));
    }

    if let Some(frame_id) = params.get("frame_id").and_then(|v| v.as_str()) {
        let raw_path = PathBuf::from(&cfg.paths.cache_screens)
            .join(format!("{}_raw.png", frame_id));
        return Ok((Some(frame_id.to_string()), raw_path));
    }

    Err("missing frame_id or raw_path".to_string())
}

fn new_frame_id() -> String {
    let counter = FRAME_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("frame_{}_{}", Utc::now().format("%Y%m%d_%H%M%S"), counter)
}

fn decode_base64_image(data: &str) -> Result<Vec<u8>, String> {
    let trimmed = match data.split_once(',') {
        Some((_, tail)) => tail,
        None => data,
    };
    BASE64_ENGINE
        .decode(trimmed)
        .map_err(|e| format!("decode base64 failed: {}", e))
}

fn encode_base64_with_limit(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("read image failed: {}", e))?;
    let max_raw = (MAX_IMAGE_BYTES / 4) * 3;
    if metadata.len() > max_raw {
        return Err(format!(
            "image too large for base64 limit (> {} raw bytes)",
            max_raw
        ));
    }
    let bytes = fs::read(path).map_err(|e| format!("read image failed: {}", e))?;
    let b64 = BASE64_ENGINE.encode(bytes);
    if b64.len() > MAX_IMAGE_BYTES as usize {
        return Err(format!(
            "base64 too large (> {} bytes)",
            MAX_IMAGE_BYTES
        ));
    }
    Ok(b64)
}

fn save_rgba_image(image: &RgbaImage, path: &Path) -> Result<(), String> {
    let dyn_img = DynamicImage::ImageRgba8(image.clone());
    dyn_img
        .save(path)
        .map_err(|e| format!("save image failed: {}", e))
}

fn element_has_kind(el: &Value, kind: &str) -> bool {
    if let Some(value) = el.get("type").and_then(|v| v.as_str()) {
        if value.eq_ignore_ascii_case(kind) {
            return true;
        }
    }
    if let Some(value) = el.get("category").and_then(|v| v.as_str()) {
        if value.to_lowercase().contains(kind) {
            return true;
        }
    }
    if kind == "text" {
        if let Some(value) = el.get("content").and_then(|v| v.as_str()) {
            if !value.trim().is_empty() {
                return true;
            }
        }
    }
    false
}

fn element_color(el: &Value) -> Rgba<u8> {
    if element_has_kind(el, "text") {
        Rgba([0, 200, 0, 255])
    } else if element_has_kind(el, "icon") {
        Rgba([200, 0, 0, 255])
    } else {
        Rgba([200, 200, 0, 255])
    }
}

fn extract_bbox(el: &Value, width: u32, height: u32) -> Option<Rect> {
    let bbox = el.get("bbox")?.as_array()?;
    if bbox.len() < 4 {
        return None;
    }

    let mut vals = [0.0f32; 4];
    for (idx, val) in bbox.iter().take(4).enumerate() {
        vals[idx] = val.as_f64().unwrap_or(0.0) as f32;
    }

    let max_val = vals.iter().cloned().fold(0.0_f32, f32::max);
    let normalized = max_val <= 1.5;

    let mut x1 = vals[0];
    let mut y1 = vals[1];
    let mut x2 = vals[2];
    let mut y2 = vals[3];

    if normalized {
        x1 *= width as f32;
        x2 *= width as f32;
        y1 *= height as f32;
        y2 *= height as f32;
    }

    let x1 = x1.round().clamp(0.0, width.saturating_sub(1) as f32) as i32;
    let y1 = y1.round().clamp(0.0, height.saturating_sub(1) as f32) as i32;
    let x2 = x2.round().clamp(0.0, width as f32) as i32;
    let y2 = y2.round().clamp(0.0, height as f32) as i32;

    let w = (x2 - x1).max(1) as u32;
    let h = (y2 - y1).max(1) as u32;

    Some(Rect::at(x1, y1).of_size(w, h))
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn aw_context_json(cfg: &Config) -> Value {
    let client = AwClient::new(cfg.aw.base_url.clone());
    json!({
        "info": client.get_info().ok(),
        "buckets": client.get_buckets().ok(),
    })
}

fn capture_image(mode: &str, _with_cursor: bool) -> Result<(RgbaImage, bool), String> {
    capture_image_platform(mode)
}

#[cfg(windows)]
fn capture_image_platform(mode: &str) -> Result<(RgbaImage, bool), String> {
    match mode {
        "full" => {
            let monitors = Monitor::all().map_err(|e| e.to_string())?;
            let monitor = monitors
                .iter()
                .find(|m| m.is_primary().unwrap_or(false))
                .or_else(|| monitors.first())
                .ok_or_else(|| "no monitor found".to_string())?;
            let image = monitor.capture_image().map_err(|e| e.to_string())?;
            Ok((image, false))
        }
        "active" => {
            let windows = Window::all().map_err(|e| e.to_string())?;
            let window = windows
                .into_iter()
                .find(|w| w.is_focused().unwrap_or(false) && !w.is_minimized().unwrap_or(true))
                .ok_or_else(|| "active window not found".to_string())?;
            let image = window.capture_image().map_err(|e| e.to_string())?;
            Ok((image, false))
        }
        _ => Err("invalid mode".to_string()),
    }
}

#[cfg(not(windows))]
fn capture_image_platform(_mode: &str) -> Result<(RgbaImage, bool), String> {
    Err("screen capture only supported on Windows".to_string())
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
