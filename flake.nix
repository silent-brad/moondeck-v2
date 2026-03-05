{
  description = "Moondeck - Cyberdeck display configured in Lua";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };

        # FHS environment for ESP-IDF compatibility
        fhs = pkgs.buildFHSEnv {
          name = "moondeck-dev";
          targetPkgs = pkgs:
            with pkgs; [
              # Build essentials
              cmake
              ninja
              pkg-config
              gnumake
              gcc
              flex
              bison
              gperf
              gettext
              ccache
              git
              curl
              wget

              # Python for ESP-IDF
              python3
              python3Packages.pip
              python3Packages.virtualenv

              # OpenSSL
              openssl
              openssl.dev

              # ESP tools
              espflash
              espup
              rustup

              # Serial monitor
              picocom

              # Lua for config testing
              lua5_4

              # LLVM for bindgen
              llvmPackages.libclang

              # Libraries needed by ESP-IDF binaries
              zlib
              libxml2_13 # ESP tools need libxml2.so.2 (old ABI)
              ncurses
              stdenv.cc.cc.lib

              # Patching tools (still useful for some edge cases)
              patchelf
              file
            ];

          profile = ''
            # Ensure cargo is available
            export PATH="$HOME/.cargo/bin:$PATH"

            # For bindgen
            export LIBCLANG_PATH="${pkgs.llvmPackages.libclang.lib}/lib"

            # ESP-IDF version
            export ESP_IDF_VERSION="v5.2"

            # Auto-install rustup if not initialized
            if [ ! -d "$HOME/.rustup" ]; then
              echo "📦 Installing rustup..."
              rustup-init -y --no-modify-path
              export PATH="$HOME/.cargo/bin:$PATH"
            fi

            # Auto-install ESP toolchain if not present
            EXPORT_FILE="$HOME/.espup/export-esp.sh"
            if [ ! -f "$EXPORT_FILE" ]; then
              echo "📦 Installing ESP Xtensa toolchain (this may take a few minutes)..."
              espup install --targets esp32s3
            fi

            # Auto-install ldproxy if not present
            if ! command -v ldproxy &> /dev/null; then
              echo "📦 Installing ldproxy..."
              cargo install ldproxy
            fi

            # Source ESP toolchain
            if [ -f "$EXPORT_FILE" ]; then
              source "$EXPORT_FILE"
            fi

            echo ""
            echo "🌙 Moondeck ESP32-S3 Development Environment (FHS)"
            echo ""
            echo "  Build & flash:"
            echo "    cargo build --release -p moondeck-app"
            echo "    espflash flash target/xtensa-esp32s3-espidf/release/moondeck --monitor"
            echo ""
          '';

          runScript = "bash";
        };
      in {
        # Enter FHS shell with: nix develop
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [ fhs ];
          shellHook = ''
            echo "Entering FHS environment..."
            exec ${fhs}/bin/moondeck-dev
          '';
        };

        # For running commands directly: nix run .#build
        packages.build = pkgs.writeShellScriptBin "moondeck-build" ''
          exec ${fhs}/bin/moondeck-dev -c "
            export PATH=\"\$HOME/.cargo/bin:\$PATH\"
            if [ -f \"\$HOME/.espup/export-esp.sh\" ]; then
              source \"\$HOME/.espup/export-esp.sh\"
            fi
            cargo build --release -p moondeck-app
          "
        '';

        # Export the FHS env itself
        packages.default = fhs;
      });
}
