#!/bin/bash
# sync-frontend.sh — Sync web/dist → desktop/frontend with path corrections
#
# Trunk builds with public_url="/ui/" (for the web server).
# Tauri serves from root "/" (no prefix).
# This script copies assets and generates a desktop index.html with relative paths.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
WEB_DIST="${SCRIPT_DIR}/../web/dist"
DESKTOP_FE="${SCRIPT_DIR}/frontend"

if [ ! -d "$WEB_DIST" ]; then
    echo "ERROR: $WEB_DIST not found. Run trunk build in crates/web first."
    exit 1
fi

# Copy WASM, JS, CSS assets
mkdir -p "$DESKTOP_FE"
# Remove old WASM/JS files to prevent stale references
rm -f "$DESKTOP_FE"/web-*.wasm "$DESKTOP_FE"/web-*.js
cp "$WEB_DIST"/*.wasm "$DESKTOP_FE/" 2>/dev/null || true
cp "$WEB_DIST"/*.js "$DESKTOP_FE/" 2>/dev/null || true
cp "$WEB_DIST"/*.css "$DESKTOP_FE/" 2>/dev/null || true

# Find the WASM background JS file (the main entry) from DIST, not frontend
WASM_JS=$(ls "$WEB_DIST"/web-*.js 2>/dev/null | head -1 | xargs -r basename)
WASM_BG=$(ls "$WEB_DIST"/web-*_bg.wasm 2>/dev/null | head -1 | xargs -r basename)
STYLE_CSS=$(ls "$DESKTOP_FE"/style.css 2>/dev/null | head -1 | xargs -r basename)

if [ -z "$WASM_JS" ] || [ -z "$WASM_BG" ]; then
    echo "ERROR: No WASM/JS files found in $WEB_DIST"
    exit 1
fi

echo "Syncing: $WASM_JS, $WASM_BG, $STYLE_CSS"

# Generate desktop index.html with relative paths (no /ui/ prefix)
cat > "$DESKTOP_FE/index.html" << 'HTMLEOF'
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8"/>
    <meta name="viewport" content="width=device-width, initial-scale=1"/>
    <meta name="description" content="Ferro - Self-hosted file storage platform"/>
    <title>FERRO</title>
    <link rel="preconnect" href="https://fonts.googleapis.com"/>
    <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin/>
    <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600;700;800;900&family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet"/>
    <link rel="stylesheet" href="style.css"/>
    <link data-trunk-rel="rust" data-wasm-opt="O" data-bincode/>
HTMLEOF

# Inject dynamic references
cat >> "$DESKTOP_FE/index.html" << EOF
    <link rel="modulepreload" href="${WASM_JS}" crossorigin="anonymous"/>
    <link rel="preload" href="${WASM_BG}" crossorigin="anonymous" as="fetch" type="application/wasm"/>
EOF

cat >> "$DESKTOP_FE/index.html" << 'HTMLEOF'
</head>
<body>
    <div id="app"></div>
    <noscript>
        <div style="padding: 3rem; text-align: center; font-family: 'IBM Plex Mono', monospace; background: #F5F0EB; height: 100vh; display: flex; flex-direction: column; align-items: center; justify-content: center; border: 3px solid #2B2B2B;">
            <h1 style="font-size: 4rem; font-weight: 900; letter-spacing: -0.03em; margin-bottom: 1rem;">FERRO</h1>
            <p style="font-family: 'Inter', sans-serif; color: #8B8178; text-transform: uppercase; letter-spacing: 0.08em; font-size: 0.75rem;">WebAssembly is required to run this application.</p>
        </div>
    </noscript>
<script>window.FERRO_SERVER_URL = window.__FERRO_SERVER_URL__ || 'http://127.0.0.1:3000'; console.log('FERRO_SERVER_URL set to', window.FERRO_SERVER_URL);</script>

<script type="module">
HTMLEOF

cat >> "$DESKTOP_FE/index.html" << EOF
import init, * as bindings from './${WASM_JS}';
const wasm = await init({ module_or_path: './${WASM_BG}' });
EOF

cat >> "$DESKTOP_FE/index.html" << 'HTMLEOF'

window.wasmBindings = bindings;

dispatchEvent(new CustomEvent("TrunkApplicationStarted", {detail: {wasm}}));

</script></body>
</html>
HTMLEOF

echo "Done: $DESKTOP_FE/index.html updated"
