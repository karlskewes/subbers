{
  # flake.nix and default.nix thanks to: https://n8henrie.com/2023/09/crosscompile-rust-for-x86-linux-from-m1-mac-with-nix/
  # TODO: dynamically build for host system and cross compile if required. https://nix.dev/tutorials/cross-compilation.html
  description = "Manage subs for your sports game";

  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, rust-overlay, }:
    let
      system = "aarch64-linux";
      overlays = [ (import rust-overlay) ];
      makePkgs = targetSystem:
        import nixpkgs {
          inherit overlays system;
          crossSystem = {
            config = targetSystem;
            rustc.config = targetSystem;
            isStatic = true;
          };
        };
    in {
      packages.${system} = {
        default = self.outputs.packages.${system}.subbers-x86_64-linux;
        subbers-aarch64-linux =
          (makePkgs "aarch64-unknown-linux-musl").callPackage ./default.nix { };
        subbers-x86_64-linux =
          (makePkgs "x86_64-unknown-linux-musl").callPackage ./default.nix { };
      };
    };
}

