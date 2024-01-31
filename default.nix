{
  lib,
  rustPlatform,
  ...
}:
rustPlatform.buildRustPackage {
  pname = "nvmetcfg";
  version = "0.1.0";
  src = ./.;

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
