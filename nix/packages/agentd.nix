# agentd — guest init/agent daemon for microsandbox microVMs.
#
# Built as a static musl binary from this flake's filtered workspace source.
# agentd runs INSIDE the guest microVM, not on the host. A static musl binary
# has no dynamic-library dependencies, but retains third-party licensing duties.
# It works with musl or glibc guest root filesystems.
#
# Uses pkgsStatic (nixpkgs' static musl build infrastructure) so the musl
# toolchain is handled automatically. nixpkgs' default rustc is used (not the
# fenix pin) — agentd is a simple guest binary and doesn't need to match the
# host toolchain. If nixpkgs' rustc is too old for edition 2024 (requires
# rustc >= 1.85), the host build will fail and we must inject the fenix
# toolchain via fenix.combine with the musl target.

{
  pkgs,
  src,
  cargoLock,
  version,
}:
let
  legal = import ./license-hooks.nix {
    inherit pkgs src;
    pname = "microsandbox-agentd";
    manifest = "crates/agentd/Cargo.toml";
    target = pkgs.pkgsStatic.stdenv.hostPlatform.rust.rustcTarget;
  };
in
pkgs.pkgsStatic.rustPlatform.buildRustPackage rec {
  pname = "microsandbox-agentd";
  inherit version;

  inherit src;

  inherit cargoLock;
  inherit (legal) nativeBuildInputs postBuild postInstall;

  # Only build the agentd crate, not the whole workspace.
  cargoBuildFlags = [
    "-p"
    "microsandbox-agentd"
  ];

  doCheck = false;

  installPhase = ''
    runHook preInstall
    install -Dm755 target/${pkgs.pkgsStatic.stdenv.hostPlatform.rust.rustcTarget}/release/agentd \
      $out/libexec/agentd
    runHook postInstall
  '';

  doInstallCheck = true;
  installCheckPhase = ''
    runHook preInstallCheck
    ${pkgs.buildPackages.binutils}/bin/readelf --program-headers $out/libexec/agentd > headers
    ${pkgs.buildPackages.binutils}/bin/readelf --dynamic $out/libexec/agentd > dynamic
    if grep -q INTERP headers || grep -q NEEDED dynamic; then
      echo "error: guest agentd must be statically linked" >&2
      exit 1
    fi
    runHook postInstallCheck
  '';

  meta.license = pkgs.lib.licenses.asl20;
}
