$Root = "F:\aw-omni\src"
if (-not (Test-Path "$Root\Cargo.toml")) {
  Write-Error "Missing workspace at $Root"
  exit 1
}
Set-Location $Root
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
  Write-Error "cargo not found in PATH. Use WSL or install Rust on Windows."
  exit 1
}

cargo run -p aw_omni_daemon -- --config config\local.win.toml @args
