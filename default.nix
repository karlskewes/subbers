{ rustPlatform }:
rustPlatform.buildRustPackage {
  name = "subbers";
  src = ./.;
  cargoLock.lockFile = ./Cargo.lock;
}
