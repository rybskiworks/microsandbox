# SPDX-FileCopyrightText: 2026 Georg Rybski
# SPDX-License-Identifier: Apache-2.0
# Per-binary notices generated inside the already-vendored build environment.
{
  pkgs,
  src,
  pname,
  manifest,
  target,
  features ? "",
  noDefaultFeatures ? false,
}:
let
  # Import the directory, not just the script: reviewed evidence is adjacent.
  helper = "${../../scripts/licensing}/cargo_notices.py";
in
{
  nativeBuildInputs = [
    pkgs.buildPackages.python3
    pkgs.buildPackages.cargo-about
  ];
  postBuild = ''
    python3 ${helper} generate \
      --manifest "$PWD/${manifest}" --policy ${src}/deny.toml \
      --target ${pkgs.lib.escapeShellArg target} --source-root "$PWD" \
      ${pkgs.lib.optionalString noDefaultFeatures "--no-default-features"} \
      ${pkgs.lib.optionalString (features != "") "--features ${pkgs.lib.escapeShellArg features}"} \
      --output "$TMPDIR/${pname}-notices"
  '';
  postInstall = ''
    mkdir -p $out/share/licenses/${pname}
    install -m644 ${src}/LICENSE $out/share/licenses/${pname}/LICENSE
    cp -r "$TMPDIR/${pname}-notices" $out/share/licenses/${pname}/rust
    python3 ${helper} verify $out/share/licenses/${pname}/rust
  '';
}
