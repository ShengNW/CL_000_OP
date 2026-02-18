@echo off
set ROOT=F:\aw-omni\src
if not exist "%ROOT%\Cargo.toml" (
  echo Missing workspace at %ROOT%
  exit /b 1
)
cd /d "%ROOT%"
where cargo >nul 2>nul
if errorlevel 1 (
  echo cargo not found in PATH. Use WSL or install Rust on Windows.
  exit /b 1
)

cargo run -p aw_omni_daemon -- --config config\local.win.toml %*
