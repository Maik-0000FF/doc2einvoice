{
  # SPDX-FileCopyrightText: Maik-0000FF
  # SPDX-License-Identifier: GPL-3.0-or-later
  #
  # Nix development environment for doc2einvoice.
  #
  # Provides a reproducible development shell only: the Rust toolchain, Node.js
  # and the native libraries the Tauri WebView links against on Linux. It does
  # not package or install the application.
  #
  # Usage:
  #   nix develop        # enter the dev shell (or `direnv allow` once)
  #   npm install
  #   cargo tauri dev    # run the application
  #   npm run verify     # every quality gate
  #
  # The pin tracks nixos-unstable; flake.lock freezes the exact revision.

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      # The shell lists the GTK/WebKitGTK stack, which is the Linux backend of
      # the WebView. Windows and macOS builds use the platform WebView and are
      # produced on their own hosts, so a shell for them would be misleading.
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          inherit (pkgs) lib;

          # Libraries the WebView links at build time and loads at run time.
          webviewLibraries = [
            pkgs.at-spi2-atk
            pkgs.cairo
            pkgs.gdk-pixbuf
            pkgs.glib
            pkgs.gtk3
            pkgs.harfbuzz
            pkgs.librsvg
            pkgs.libsoup_3
            pkgs.openssl
            pkgs.pango
            pkgs.webkitgtk_4_1
          ];

          # GSettings schemas GTK needs for its file dialogs; without them the
          # open/save dialog aborts the process.
          gsettingsSchemaPackages = [
            pkgs.gsettings-desktop-schemas
            pkgs.gtk3
          ];
          gsettingsSchemaDirs = lib.concatMapStringsSep ":" (
            pkg: "${pkg}/share/gsettings-schemas/${pkg.name}"
          ) gsettingsSchemaPackages;

          # The graphics drivers the WebView renders with. A NixOS host offers
          # its own under /run/opengl-driver, built against the C library of
          # the host. If that one is newer than the C library of this shell,
          # they cannot be loaded and the WebView aborts while it creates its
          # EGL display. The shell therefore brings the drivers that belong to
          # its own packages: this file names their EGL library by its full
          # path, and the two variables below name the directories of the
          # drivers it loads in turn.
          eglVendorFile = pkgs.writeText "50_mesa.json" (
            builtins.toJSON {
              file_format_version = "1.0.0";
              ICD.library_path = "${pkgs.mesa}/lib/libEGL_mesa.so.0";
            }
          );
        in
        {
          default = pkgs.mkShell {
            nativeBuildInputs = [
              pkgs.cargo
              pkgs.cargo-about
              pkgs.cargo-deny
              pkgs.cargo-tauri
              pkgs.clippy
              pkgs.gobject-introspection
              pkgs.nodejs
              pkgs.pkg-config
              pkgs.reuse
              pkgs.rust-analyzer
              pkgs.rustc
              pkgs.rustfmt
            ];

            buildInputs = webviewLibraries;

            # rust-analyzer needs the standard library sources.
            RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

            __EGL_VENDOR_LIBRARY_FILENAMES = "${eglVendorFile}";
            LIBGL_DRIVERS_PATH = "${pkgs.mesa}/lib/dri";
            GBM_BACKENDS_PATH = "${pkgs.mesa}/lib/gbm";

            shellHook = ''
              export XDG_DATA_DIRS="${gsettingsSchemaDirs}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
            '';
          };
        }
      );
    };
}
