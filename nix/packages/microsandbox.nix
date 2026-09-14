# Assemble immutable source-built components without rebuilding the CLI.
{
  pkgs,
  cli,
  agentd,
  libkrunfw,
}:
assert cli.agentd.drvPath == agentd.drvPath;
pkgs.runCommand "microsandbox-${cli.version}"
  {
    pname = "microsandbox";
    inherit (cli) version;
    nativeBuildInputs = [ pkgs.buildPackages.python3 ];
    passthru = {
      inherit cli agentd libkrunfw;
      # Consumers use this canonical filtered workspace for matching SDK builds.
      inherit (cli) src;
      licensing = {
        schema = 1;
        firmware = "share/libkrunfw/compliance";
        notices = [
          "share/licenses/microsandbox-cli/rust"
          "share/licenses/microsandbox-agentd/rust"
        ];
      };
    };
    meta = cli.meta // {
      description = "Microsandbox CLI, static guest agent and source-built firmware";
      license = with pkgs.lib.licenses; [
        asl20
        lgpl21Only
        gpl2Only
      ];
    };
  }
  ''
    # Do not silently assemble the old firmware without corresponding source.
    # Producer pin/lock promotion is a required integration step for this PR.
    if ! test -s ${libkrunfw}/share/libkrunfw/compliance/manifest.json; then
      echo "error: promote a validated source-bundle-capable libkrunfw input" >&2
      exit 1
    fi

    # Use a regular copy: current_exe must resolve inside this complete
    # runtime, so sibling lib/ discovery cannot resolve to the CLI-only output.
    install -Dm755 ${cli}/bin/msb $out/bin/msb
    install -Dm755 ${agentd}/libexec/agentd $out/libexec/agentd
    install -Dm755 ${libkrunfw}/lib/libkrunfw.so.5.6.1 $out/lib/libkrunfw.so.5.6.1
    ln -s libkrunfw.so.5.6.1 $out/lib/libkrunfw.so.5
    ln -s libkrunfw.so.5 $out/lib/libkrunfw.so
    mkdir -p $out/share/libkrunfw
    for name in kernel.config kernel.release kernel-source.sha256 kernel-patches.sha256; do
      install -m644 ${libkrunfw}/share/libkrunfw/$name $out/share/libkrunfw/$name
    done
    cp -r ${libkrunfw}/share/libkrunfw/compliance $out/share/libkrunfw/compliance
    python3 ${libkrunfw.src}/scripts/licensing/source_bundle.py verify \
      --bundle $out/share/libkrunfw/compliance \
      --binary $out/lib/libkrunfw.so.5.6.1

    # The CLI embeds agentd; a CLI-only Rust graph does not cover that payload.
    mkdir -p $out/share/licenses
    cp -r ${cli}/share/licenses/microsandbox-cli $out/share/licenses/microsandbox-cli
    cp -r ${agentd}/share/licenses/microsandbox-agentd $out/share/licenses/microsandbox-agentd
    for component in microsandbox-cli microsandbox-agentd; do
      python3 ${../../scripts/licensing/cargo_notices.py} verify $out/share/licenses/$component/rust
    done
    install -Dm644 ${../../LICENSE} $out/share/licenses/microsandbox/LICENSE
    install -Dm644 ${../../DISTRIBUTION.md} $out/share/licenses/microsandbox/DISTRIBUTION.md
  ''
