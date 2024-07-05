{
  lib,
  rustPlatform,
  ...
}:
rustPlatform.buildRustPackage {
  pname = "nvmetcfg";
  version = "0.1.0";
  src = with lib.strings;
    builtins.filterSource
    (path: type: builtins.any (suf: hasPrefix (toString suf) path) [./src ./Cargo.toml ./Cargo.lock])
    ./.;

  cargoLock = {
    lockFile = ./Cargo.lock;
  };

  meta = with lib; {
    mainProgram = "nvmet";
    platforms = platforms.linux;
    license = licenses.isc;
    maintainers = [maintainers.vifino];
  };
}
