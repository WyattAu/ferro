{
  description = "Ferro: High-performance Rust Storage Orchestrator";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config = {
            allowUnfreePredicate = pkg: builtins.elem (pkgs.lib.getName pkg) [
              "graphite"
            ];
          };
        };

        # Rust toolchain with wasm32 target
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "rustfmt" "clippy" ];
          targets = [ "wasm32-unknown-unknown" ];
        };

        # System libraries needed for Tauri / WebKitGTK / crypto
        # Use .dev for multi-output packages to get pkg-config files
        systemLibraries = with pkgs; [
          webkitgtk_4_1.dev
          gtk3.dev
          gtk4.dev
          cairo.dev
          gdk-pixbuf.dev
          glib.dev
          dbus.dev
          openssl_3.dev
          librsvg.dev
          libsoup_3.dev
          pango.dev
          atk.dev
          at-spi2-atk.dev
          libxkbcommon.dev
          libepoxy.dev
          freetype.dev
          fontconfig.dev
          libGL
          wayland.dev
          libxrandr.dev
          libXi.dev
          libXcursor.dev
          libX11.dev
          libX11.dev
        ];

        # Core development tools (always available)
        coreTools = with pkgs; [
          rustToolchain
          pkg-config
          cargo-watch
          cargo-edit
          cargo-audit
          cargo-deny
          rclone
          sqlite
          sqlx-cli
          protobuf          # for some dependency builds
          cmake
          gcc
          wrapGAppsHook3
          wrapGAppsHook4
        ];

        # WASM / Web frontend tools
        wasmTools = with pkgs; [
          trunk
          binaryen          # wasm-opt
          wasm-bindgen-cli
          wasm-pack
          nodejs            # for some JS tooling
        ];

        # Desktop / Tauri build dependencies
        desktopTools = with pkgs; [
          llvmPackages.bintools
          gnumake
          file
          libappindicator-gtk3
          util-linux
          xdg-utils
          zenity
          shared-mime-info
        ];

        # Database services
        dbServices = with pkgs; [
          postgresql_16
        ];

        # Process composition for running services
        processTools = with pkgs; [
          process-compose
          overmind
        ];

        # Common shellHook logic
        commonShellHook = ''
          export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath systemLibraries}:$LD_LIBRARY_PATH"
          export PKG_CONFIG_PATH="${pkgs.lib.makeSearchPath "lib/pkgconfig" systemLibraries}:$PKG_CONFIG_PATH"
          export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules"
          export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:$XDG_DATA_DIRS"

          # Cargo paths
          export PATH="$HOME/.cargo/bin:$PATH"

          # Project environment
          export FERRO_LOG_LEVEL=debug
          export RUST_BACKTRACE=1
          export RUST_LOG=ferro=debug,ferro_server=debug,ferro_core=debug

          echo "🛡️  Ferro Development Environment"
          echo "────────────────────────────────────"
        '';

        # PostgreSQL data directory
        pgDataDir = "./.pgdata";

        # PostgreSQL helper script
        pgStartScript = pkgs.writeShellScriptBin "ferro-pg-start" ''
          export PGDATA="${pgDataDir}"
          export PGPORT=5432
          export PGHOST="127.0.0.1"

          if [ ! -d "$PGDATA" ]; then
            echo "📦 Initializing PostgreSQL database..."
            initdb --auth=trust --no-locale --encoding=UTF8
            # Configure for development
            cat >> "$PGDATA/postgresql.conf" <<EOF
          listen_addresses = '127.0.0.1'
          port = 5432
          log_statement = 'all'
          log_duration = on
          max_connections = 100
          EOF
            cat >> "$PGDATA/pg_hba.conf" <<EOF
          local   all   all   trust
          host    all   all   127.0.0.1/32   trust
          EOF
          fi

          # Start PostgreSQL (or restart if already running)
          if pg_isready -h "$PGHOST" -p "$PGPORT" >/dev/null 2>&1; then
            echo "✅ PostgreSQL already running on port $PGPORT"
          else
            echo "🚀 Starting PostgreSQL on port $PGPORT..."
            pg_ctl start -l "$PGDATA/pg.log" -w
            echo "✅ PostgreSQL started"
          fi

          echo "   Host: $PGHOST:$PGPORT"
          echo "   Database URL: postgres://localhost:5432/ferro"
          echo ""
          echo "   To create the ferro database:"
          echo "   $ createdb ferro"
          echo ""
          echo "   To run SQLx migrations:"
          echo "   $ sqlx migrate run"
        '';

        pgStopScript = pkgs.writeShellScriptBin "ferro-pg-stop" ''
          export PGDATA="${pgDataDir}"
          export PGPORT=5432
          if pg_isready -h 127.0.0.1 -p "$PGPORT" >/dev/null 2>&1; then
            echo "🛑 Stopping PostgreSQL..."
            pg_ctl stop -m fast
            echo "✅ PostgreSQL stopped"
          else
            echo "ℹ️  PostgreSQL is not running"
          fi
        '';

        pgResetScript = pkgs.writeShellScriptBin "ferro-pg-reset" ''
          export PGDATA="${pgDataDir}"
          export PGPORT=5432
          if pg_isready -h 127.0.0.1 -p "$PGPORT" >/dev/null 2>&1; then
            pg_ctl stop -m fast 2>/dev/null
          fi
          rm -rf "$PGDATA"
          echo "🗑️  PostgreSQL data directory removed"
          echo "   Run ferro-pg-start to reinitialize"
        '';

        # Integration test runner
        testRunnerScript = pkgs.writeShellScriptBin "ferro-test-integration" ''
          set -e

          echo "🧪 Ferro Integration Test Runner"
          echo "══════════════════════════════════"

          # Ensure PostgreSQL is available
          export PGDATA="${pgDataDir}"
          export PGPORT=5432
          export PGHOST="127.0.0.1"
          export DATABASE_URL="postgres://localhost:5432/ferro_test"

          if ! command -v pg_isready &>/dev/null; then
            echo "⚠️  PostgreSQL not available, skipping DB-dependent tests"
            cargo test --workspace -- --skip sqlite_metadata --skip pg_metadata
            exit $?
          fi

          # Start PostgreSQL
          if ! pg_isready -h "$PGHOST" -p "$PGPORT" >/dev/null 2>&1; then
            echo "📦 Starting PostgreSQL for integration tests..."
            if [ ! -d "$PGDATA" ]; then
              initdb --auth=trust --no-locale --encoding=UTF8
              cat >> "$PGDATA/postgresql.conf" <<EOF
          listen_addresses = '127.0.0.1'
          port = 5432
          log_statement = 'all'
          EOF
            fi
            pg_ctl start -l "$PGDATA/pg.log" -w
          fi

          # Create test database
          createdb ferro_test 2>/dev/null || true

          echo "✅ PostgreSQL ready"
          echo ""
          echo "Running full test suite..."
          echo ""

          # Run all tests
          cargo test --workspace "$@"

          echo ""
          echo "✅ Tests complete"
        '';

        # WASM build helper
        wasmBuildScript = pkgs.writeShellScriptBin "ferro-wasm-build" ''
          set -e
          echo "🔧 Building Ferro Web Frontend (WASM)..."
          echo ""

          # Build the WASM target
          echo "📦 Compiling ferro-web for wasm32-unknown-unknown..."
          cargo build -p web --target wasm32-unknown-unknown --release

          # Build with trunk
          echo "🏗️  Bundling with trunk..."
          cd crates/web
          trunk build --release

          echo ""
          echo "✅ Web frontend built to crates/web/dist/"
          echo "   Serve with: trunk serve --open"
        '';

        # Process compose config for all services
        processComposeConfig = pkgs.writeText "process-compose.yaml" ''
          version: "0.5"

          processes:
            postgres:
              command: |
                export PGDATA="${pgDataDir}"
                if [ ! -d "$PGDATA" ]; then
                  initdb --auth=trust --no-locale --encoding=UTF8
                  cat >> "$PGDATA/postgresql.conf" <<CONF
          listen_addresses = '127.0.0.1'
          port = 5432
          log_statement = 'all'
          CONF
                fi
                postgres -k "$PGDATA" -l "$PGDATA/pg.log"
              availability:
                restart: on_failure
                max_restarts: 3
              readiness_probe:
                exec:
                  command: pg_isready -h 127.0.0.1 -p 5432
                initial_delay: 2s
                period: 1s
        '';

      in
      {
        # ─────────────────────────────────────────────
        # Default dev shell: Rust + WASM + SQLite + CLI
        # ─────────────────────────────────────────────
        devShells.default = pkgs.mkShell {
          name = "ferro-dev";
          buildInputs = coreTools ++ wasmTools ++ dbServices;
          nativeBuildInputs = [ pkgs.pkg-config ];
          packages = [
            pgStartScript
            pgStopScript
            pgResetScript
            testRunnerScript
            wasmBuildScript
          ];

          shellHook = commonShellHook + ''
            echo "   Tools: cargo, trunk, sqlx-cli, rclone, sqlite3"
            echo "   WASM:  wasm32-unknown-unknown target ready"
            echo "   DB:    PostgreSQL 16 available (ferro-pg-start)"
            echo ""
            echo "   Quick start:"
            echo "     cargo build --workspace          # Build everything"
            echo "     cargo test --workspace           # Run all tests"
            echo "     cargo run -p ferro-server         # Start server"
            echo "     ferro-pg-start                   # Start PostgreSQL"
            echo "     ferro-wasm-build                 # Build web frontend"
            echo "     ferro-test-integration           # Run tests with DB"
            echo ""
          '';
        };

        # ─────────────────────────────────────────────
        # Minimal shell: just Rust + system libs
        # ─────────────────────────────────────────────
        devShells.minimal = pkgs.mkShell {
          name = "ferro-minimal";
          buildInputs = [ rustToolchain pkgs.pkg-config pkgs.openssl_3 ];
          nativeBuildInputs = [ pkgs.pkg-config ];

          shellHook = ''
            export LD_LIBRARY_PATH="${pkgs.lib.makeLibraryPath (with pkgs; [ openssl_3 ])}:$LD_LIBRARY_PATH"
            export RUST_BACKTRACE=1
            echo "🛡️  Ferro Minimal Shell (Rust only)"
          '';
        };

        # ─────────────────────────────────────────────
        # Desktop shell: includes Tauri system deps
        # ─────────────────────────────────────────────
        devShells.desktop = pkgs.mkShell {
          name = "ferro-desktop";
          buildInputs = coreTools ++ systemLibraries ++ desktopTools;
          nativeBuildInputs = [ pkgs.pkg-config pkgs.wrapGAppsHook4 ];

          shellHook = commonShellHook + ''
            echo "   Desktop: Tauri system libraries loaded"
            echo "   GTK4 + WebKitGTK4.1 + libadwaita"
            echo ""
            echo "   To build the desktop app:"
            echo "     cargo install tauri-cli --version '^2'"
            echo "     cargo tauri dev"
            echo ""
          '';
        };

        # ─────────────────────────────────────────────
        # Web shell: WASM + trunk + binaryen
        # ─────────────────────────────────────────────
        devShells.web = pkgs.mkShell {
          name = "ferro-web";
          buildInputs = coreTools ++ wasmTools;
          nativeBuildInputs = [ pkgs.pkg-config ];
          packages = [ wasmBuildScript ];

          shellHook = commonShellHook + ''
            echo "   WASM:  wasm32-unknown-unknown target ready"
            echo "   Tools: trunk ${pkgs.trunk.version}, wasm-bindgen-cli ${pkgs.wasm-bindgen-cli.version}"
            echo ""
            echo "   Quick start:"
            echo "     ferro-wasm-build     # Production build"
            echo "     cd crates/web && trunk serve  # Dev server"
            echo ""
          '';
        };

        # ─────────────────────────────────────────────
        # Services shell: PostgreSQL + process-compose
        # ─────────────────────────────────────────────
        devShells.services = pkgs.mkShell {
          name = "ferro-services";
          buildInputs = coreTools ++ dbServices ++ processTools;
          nativeBuildInputs = [ pkgs.pkg-config ];
          packages = [
            pgStartScript
            pgStopScript
            pgResetScript
            testRunnerScript
          ];

          shellHook = commonShellHook + ''
            echo "   Services: PostgreSQL 16"
            echo "   Process manager: process-compose, overmind"
            echo ""
            echo "   Quick start:"
            echo "     ferro-pg-start          # Start PostgreSQL"
            echo "     ferro-pg-stop           # Stop PostgreSQL"
            echo "     ferro-pg-reset          # Reset PostgreSQL data"
            echo "     ferro-test-integration  # Run tests with DB"
            echo "     process-compose up      # Start all services"
            echo ""
          '';
        };

        # ─────────────────────────────────────────────
        # CI shell: minimal + test tools
        # ─────────────────────────────────────────────
        devShells.ci = pkgs.mkShell {
          name = "ferro-ci";
          buildInputs = coreTools ++ dbServices;
          nativeBuildInputs = [ pkgs.pkg-config ];
          packages = [ testRunnerScript ];

          shellHook = ''
            export RUST_BACKTRACE=1
            export RUST_LOG=error
            echo "🛡️  Ferro CI Shell"
          '';
        };

        # ─────────────────────────────────────────────
        # Formatter (nix fmt)
        # ─────────────────────────────────────────────
        formatter = pkgs.nixfmt;
      }
    );
}
