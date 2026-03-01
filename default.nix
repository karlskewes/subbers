{ lib, rustPlatform }:
rustPlatform.buildRustPackage {
  name = "subbers";
  cargoLock.lockFile = ./Cargo.lock;
  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./src
      ./build.rs
      ./Cargo.toml
      ./Cargo.lock
    ];
  };

}
