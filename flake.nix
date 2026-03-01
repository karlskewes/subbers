{
  # flake.nix and default.nix thanks to: https://n8henrie.com/2023/09/crosscompile-rust-for-x86-linux-from-m1-mac-with-nix/
  # TODO: dynamically build for host system and cross compile if required. https://nix.dev/tutorials/cross-compilation.html
  description = "Manage subs for your sports game";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;

      pkgsForSystem =
        system:
        import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

      commonFor =
        system:
        let
          pkgs = pkgsForSystem system;
          rustToolchain = pkgs.pkgsBuildHost.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        in
        {
          inherit pkgs rustToolchain;
          buildInputs = [ ];
          nativeBuildInputs = [ ];
          runtimeDependencies = [ ];
        };
    in
    {
      packages = forAllSystems (
        system:
        let
          common = commonFor system;
          rustPlatform = common.pkgs.makeRustPlatform {
            cargo = common.rustToolchain;
            rustc = common.rustToolchain;
          };
        in
        {
          default = self.packages.${system}.subbers;

          subbers = rustPlatform.buildRustPackage {
            pname = "subbers";
            version = "0.1.0";

            src = common.pkgs.lib.fileset.toSource {
              root = ./.;
              fileset = common.pkgs.lib.fileset.unions [
                # .git # for build.rs
                ./src
                ./Cargo.toml
                ./Cargo.lock
                ./build.rs
              ];
            };

            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };

            separateDebugInfo = true;

            inherit (common) buildInputs nativeBuildInputs runtimeDependencies;

            makeFlags = [
              "prefix=${placeholder "out"}"
              "CARGO_TARGET_DIR=target/${common.pkgs.stdenv.hostPlatform.rust.cargoShortTarget}"
            ];

            # dontCargoInstall = true;

            meta = with common.pkgs.lib; {
              description = "Manage subs for your sports game";
              homepage = "https://github.com/karlskewes/subbers";
              license = licenses.mit;
              platforms = platforms.linux;
              mainProgram = "subbers";
            };
          };
        }
      );

      devShells = forAllSystems (
        system:
        let
          common = commonFor system;
        in
        {
          default = common.pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.subbers ];

            packages = with common.pkgs; [
              common.rustToolchain
              rust-analyzer
            ];

            LD_LIBRARY_PATH = common.pkgs.lib.makeLibraryPath common.runtimeDependencies;

            shellHook = ''
              echo "Subbers development environment"
              echo "Run 'cargo build' to build, 'cargo test' to test"
            '';
          };
        }
      );

      # Formatter for 'nix fmt'
      formatter = forAllSystems (system: (pkgsForSystem system).nixfmt-rfc-style);

      # Overlay for use in other flakes
      overlays.default = final: prev: {
        subbers = self.packages.${prev.system}.subbers;
      };
    };
}
