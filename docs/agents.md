# Agent Guide: Running & Testing the Layerrd UI

Instructions for AI agents (Claude Code, etc.) to build, run, screenshot, and visually inspect the Layerrd application in a headless environment.

## Prerequisites

Install these packages once per session:

```bash
apt-get install -y xvfb scrot xdotool libxkbcommon-x11-0 mesa-vulkan-drivers
```

## Quick Start

### 1. Start a virtual display

```bash
Xvfb :99 -screen 0 1920x1080x24 &>/dev/null &
```

### 2. Build and launch the app

The app requires Vulkan with the lavapipe software renderer (llvmpipe). The GL backend does not support storage textures needed by the compute shaders.

```bash
DISPLAY=:99 \
XDG_RUNTIME_DIR=/tmp/xdg-runtime \
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
WGPU_BACKEND=vulkan \
cargo run &>/tmp/layerrd.log &
```

Wait ~5 seconds for the window to appear, then check logs for errors:

```bash
sleep 5 && head -20 /tmp/layerrd.log
```

If the log only shows the `Finished`/`Running` lines with no panics, the app is up.

### 3. Take a screenshot

```bash
DISPLAY=:99 scrot /tmp/layerrd_screenshot.png
```

Then read the screenshot with the `Read` tool to view it visually.

### 4. Interact with the UI

Use `xdotool` to simulate mouse clicks and keyboard input:

```bash
# Click at coordinates (x, y)
DISPLAY=:99 xdotool mousemove 500 300 click 1

# Type text
DISPLAY=:99 xdotool type "hello"

# Press a key
DISPLAY=:99 xdotool key ctrl+z
```

### 5. Clean up

```bash
# Kill the app and Xvfb
kill %2 2>/dev/null   # app
kill %1 2>/dev/null   # Xvfb
```

Or by PID if backgrounding differently.

## Environment Variables Reference

| Variable | Value | Why |
|----------|-------|-----|
| `DISPLAY` | `:99` | Points to the Xvfb virtual display |
| `XDG_RUNTIME_DIR` | `/tmp/xdg-runtime` | Required by Wayland/X11 session code |
| `VK_ICD_FILENAMES` | `/usr/share/vulkan/icd.d/lvp_icd.json` | Forces lavapipe (Vulkan software renderer) |
| `WGPU_BACKEND` | `vulkan` | Selects the Vulkan backend; GL lacks storage texture support |

## Troubleshooting

### `Library libxkbcommon-x11.so could not be loaded`
Install: `apt-get install -y libxkbcommon-x11-0`

### `Too many bindings of type StorageTextures` (GL backend)
The OpenGL backend doesn't support storage textures. Use Vulkan with lavapipe instead (`WGPU_BACKEND=vulkan` + `VK_ICD_FILENAMES=.../lvp_icd.json`).

### `XDG_RUNTIME_DIR is invalid or not set`
Set it: `export XDG_RUNTIME_DIR=/tmp/xdg-runtime && mkdir -p $XDG_RUNTIME_DIR`

### App crashes with no useful error
Run with logging: `RUST_LOG=info,wgpu=warn` added to the environment to get more context.

## Testing Workflow

A typical test cycle for verifying UI changes:

```bash
# 1. Ensure Xvfb is running
Xvfb :99 -screen 0 1920x1080x24 &>/dev/null &

# 2. Build and run
DISPLAY=:99 XDG_RUNTIME_DIR=/tmp/xdg-runtime \
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
WGPU_BACKEND=vulkan cargo run &>/tmp/layerrd.log &

# 3. Wait for startup
sleep 5

# 4. Verify no errors
head -20 /tmp/layerrd.log

# 5. Screenshot and inspect
DISPLAY=:99 scrot /tmp/layerrd_screenshot.png
# Use the Read tool to view /tmp/layerrd_screenshot.png

# 6. Interact (e.g., click a menu)
DISPLAY=:99 xdotool mousemove 15 8 click 1
sleep 1
DISPLAY=:99 scrot /tmp/layerrd_after_click.png

# 7. Kill when done
pkill -f 'target/debug/layerrd'
pkill Xvfb
```
