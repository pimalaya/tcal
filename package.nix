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
  emulator = stdenv.hostPlatform.emulator buildPackages;
  exe = stdenv.hostPlatform.extensions.executable;

in
rustPlatform.buildRustPackage {
  inherit buildNoDefaultFeatures;

  pname = "tcal";
  version = "0.0.1";
  cargoHash = "";

  src = fetchFromGitHub {
    owner = "pimalaya";
    repo = "tcal";
    rev = "v0.0.1";
    hash = "";
  };

  nativeBuildInputs = [ installShellFiles ];
  buildFeatures = buildFeatures ++ [ "cli" ];

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
    description = "CLI & lib to edit iCalendars as ergonomic TOML, written in Rust";
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
