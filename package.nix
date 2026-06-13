# TODO: move this to nixpkgs
# This file aims to be a replacement for the nixpkgs derivation.

{
  buildFeatures ? [ ],
  buildNoDefaultFeatures ? false,
  buildPackages,
  fetchFromGitHub,
  installManPages ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellCompletions ? stdenv.buildPlatform.canExecute stdenv.hostPlatform,
  installShellFiles,
  lib,
  rustPlatform,
  stdenv,
}:

let
  version = "0.0.1";
  hash = "";
  cargoHash = "";

  emulator = stdenv.hostPlatform.emulator buildPackages;
  exe = stdenv.hostPlatform.extensions.executable;

in
rustPlatform.buildRustPackage {
  inherit cargoHash version buildNoDefaultFeatures;

  pname = "tcal";

  src = fetchFromGitHub {
    inherit hash;
    owner = "pimalaya";
    repo = "tcal";
    rev = "v${version}";
  };

  nativeBuildInputs = lib.optional (installManPages || installShellCompletions) installShellFiles;

  buildFeatures = buildFeatures ++ [ "cli" ];

  cargoTestFlags = [ "--lib" ];

  postInstall =
    lib.optionalString (lib.hasInfix "wine" emulator) ''
      export WINEPREFIX="''${WINEPREFIX:-$(mktemp -d)}"
      mkdir -p $WINEPREFIX
    ''
    + ''
      mkdir -p $out/share/{completions,man}
      ${emulator} "$out"/bin/tcal${exe} manuals "$out"/share/man
      ${emulator} "$out"/bin/tcal${exe} completions -d "$out"/share/completions bash elvish fish powershell zsh
    ''
    + lib.optionalString installManPages ''
      installManPage "$out"/share/man/*
    ''
    + lib.optionalString installShellCompletions ''
      installShellCompletion --cmd tcal \
        --bash "$out"/share/completions/tcal.bash \
        --fish "$out"/share/completions/tcal.fish \
        --zsh "$out"/share/completions/_tcal
    '';

  meta = {
    description = "CLI and lib to edit calendar events as ergonomic TOML, written in Rust";
    mainProgram = "tcal";
    homepage = "https://github.com/pimalaya/tcal";
    changelog = "https://github.com/pimalaya/tcal/blob/master/CHANGELOG.md";
    license = [
      lib.licenses.mit
      lib.licenses.asl20
    ];
    maintainers = with lib.maintainers; [ soywod ];
  };
}
