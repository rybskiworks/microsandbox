# Distribution licensing contract

This fork retains Microsandbox's Apache-2.0 license and all applicable upstream
ownership and notices. Georg Rybski's added licensing helpers are Apache-2.0;
this is not a reassignment of Super Rad Company, libkrun, rust-vmm, Firecracker,
Linux, or other contributors' work. Runtime metadata listing Apache, GPL and LGPL
identifies separately licensed components, not alternative choices for one work.

## Rust executables

`nix/packages/license-hooks.nix` generates notices separately for the CLI
(x86_64 GNU, no defaults, net+ssh), agentd (static musl, default features), and
brokerd (x86_64 GNU, default features). Target strings come from their actual Nix
platforms. Cargo and cargo-about use the existing pinned package/toolchain set.
The helper derives accepted licenses from deny.toml, includes local/private
selected crates and transitive/build dependencies, and runs locked/offline.

Each output installs its root license and `share/licenses/<component>/rust/`:
original legal files, HTML, normalized JSON inventory/report and checksums. A
missing selected crate or missing original legal evidence is a build error.
The report is not an exhaustive source-header audit or legal certification.
Unusual upstream notices and missing legal files need reviewed pinned evidence,
not an ignore switch. If upstream adds legal paths outside the filtered workspace,
its source filter must retain them before that input is promoted.

The notice helper/tests mirror the same contract in Workestrate. They are a
bootstrap copy, not mutable scripts fetched at build time. Promote a shared,
versioned supplier only with matching tests and immutable consumer pins.

## Runtime assembly and embedded payloads

The CLI embeds agentd. Its Rust dependency graph therefore is not the complete
license inventory of all bytes in the executable. Runtime assembly preserves
both the CLI and the separately generated agentd notice bundles.

`share/libkrunfw/compliance/` is copied in full from the firmware producer. Its
manifest is verified against the copied versioned firmware binary, not the SONAME
symlink. The bundle retains GPL-2.0-only kernel/patch source and LGPL-2.1-only
library source, the exact upstream kernel tarball, actual configuration and
build/install scripts. A hash alone is not corresponding source.

The old firmware input does not provide this contract. Runtime assembly fails
with an explicit promotion error until the validated producer is pinned and the
lockfile is regenerated. This is an intentional draft integration blocker, not
an optional bypass. Keep ordinary evaluation/formatting separate from this build
requirement. The producer is https://github.com/rybskiworks/libkrunfw/pull/5.
The coordinating consumer is https://github.com/rybskiworks/workestrate/pull/43.

## What remains outside a Cargo report

Native libc/musl, libcap-ng, compiler runtime libraries, other linked code, and
all software in a complete NixOS/agent image must be reviewed at the actual
redistribution boundary. Static linking removes dynamic dependencies, not their
copyright obligations. Native LGPL source/relinking duties are not fulfilled
merely by printing a license name. The generic firmware contract does not certify
TEE/qboot/initrd, EFI, macOS, Windows, benchmark corpora or separate agent forks.

For a public cache or portable distribution: inventory what is actually served;
retain notices and exact corresponding source through that delivery channel;
check symlinks/store references are resolvable; verify permissions for every
bundled binary; and test the assembled product. Do not label a Cargo-generated
report as a complete image SBOM or complete corresponding-source package.

## Release and historical qualification

Before promotion: run formatting, the real offline Cargo/cargo-about checks for
each binary, native linking checks, source reconstruction, and final runtime
assembly verification. Resolve missing evidence without disabling the checks.
Then promote firmware -> Microsandbox -> Workestrate with Nix/Lix-generated
lockfiles. No source hashes are invented, and no runtime ABI is changed here.

Inventory prior release assets, CI artifacts, caches, and shared images separately.
Supply notices/source matching an older distributed binary's actual revision.
A new package does not automatically cure earlier distribution. No old artifact
is deleted, no Git history rewritten, and no existing grant revoked here. The
operator's planned Lix migration is independent of this patch.

References:
- https://www.apache.org/licenses/LICENSE-2.0
- https://embarkstudios.github.io/cargo-about/cli/generate/index.html
- https://embarkstudios.github.io/cargo-about/cli/generate/config.html
- https://docs.rs/crate/ring/0.17.14/source/Cargo.toml.orig
- https://www.gnu.org/licenses/old-licenses/gpl-2.0.html
- https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html
