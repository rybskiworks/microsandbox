//! Native disk attachment coverage using an opaque disposable image.
//!
//! Requires KVM and an explicitly isolated test home. Run with
//! `MSB_TEST_ISOLATE_HOME=1 cargo test -p microsandbox --test disk_attach_only -- --ignored`.

#![cfg(target_os = "linux")]

use std::io::Write;

use microsandbox::Sandbox;
use test_utils::msb_test;

#[msb_test]
async fn opaque_readonly_disk_boots_without_being_mounted() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = tempfile::tempdir()?;
    let disk = directory.path().join("opaque.raw");
    let mut file = std::fs::File::create(&disk)?;
    file.write_all(b"attach-only-probe")?;
    file.set_len(16 * 1024 * 1024)?;
    file.sync_all()?;
    drop(file);
    let before = std::fs::read(&disk)?;
    let name = format!("disk-attach-only-{}", std::process::id());
    let sandbox = Sandbox::builder(&name)
        .image("alpine:3.20")
        .cpus(1)
        .memory(256)
        .disable_network()
        .volume("/probe", |mount| mount.disk(&disk).attach_only().readonly())
        .create()
        .await?;
    let guest = sandbox
        .shell(
            r#"set -eu
found=0
for serial_file in /sys/block/*/serial; do
    serial=$(cat "$serial_file")
    case "$serial" in
        probe_*)
            block=${serial_file%/serial}
            block=${block##*/}
            device=/dev/$block
            test -b "$device"
            test "$(dd if="$device" bs=17 count=1 2>/dev/null)" = attach-only-probe
            test "$(cat /sys/block/$block/ro)" = 1
            if grep -q "^$device " /proc/mounts; then exit 1; fi
            if grep -q ' /probe ' /proc/mounts; then exit 1; fi
            if printf forbidden | dd of="$device" bs=1 count=9 2>/dev/null; then exit 1; fi
            found=$((found + 1))
            ;;
    esac
done
test "$found" = 1
printf 'opaque disk attached, unmounted, and read-only\n'
"#,
        )
        .await;
    let stopped = sandbox.stop().await;
    let removed = sandbox.remove_persisted().await;
    let output = guest?;
    stopped?;
    removed?;
    assert!(
        output.status().success,
        "guest verification failed: stdout={:?}, stderr={:?}",
        output.stdout()?,
        output.stderr()?
    );
    assert_eq!(std::fs::read(&disk)?, before);
    Ok(())
}
