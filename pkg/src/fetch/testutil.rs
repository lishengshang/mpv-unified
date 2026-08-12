//! Offline test fixtures: hermetic zip/tar.gz builders (no external tooling),
//! a self-cleaning temp directory, and a lock for environment-var tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// A unique temp directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    #[must_use]
    pub fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("mpvcfg-test-{tag}-{}-{n}", std::process::id()));
        fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Build a valid ZIP archive (stored, no compression) from `(name, content)`
/// pairs. Hand-rolled so tests never depend on a `zip` binary or crate.
#[must_use]
pub fn zip_bytes(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, data) in files {
        let name_bytes = name.as_bytes();
        let crc = crc32(data);
        let offset = u32::try_from(out.len()).expect("small fixture");
        out.extend_from_slice(b"PK\x03\x04");
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
        out.extend_from_slice(&0u16.to_le_bytes()); // mod time
        out.extend_from_slice(&0x21u16.to_le_bytes()); // mod date 1980-01-01
        out.extend_from_slice(&crc.to_le_bytes());
        out.extend_from_slice(
            &u32::try_from(data.len())
                .expect("small fixture")
                .to_le_bytes(),
        );
        out.extend_from_slice(
            &u32::try_from(data.len())
                .expect("small fixture")
                .to_le_bytes(),
        );
        out.extend_from_slice(
            &u16::try_from(name_bytes.len())
                .expect("small fixture")
                .to_le_bytes(),
        );
        out.extend_from_slice(&0u16.to_le_bytes()); // extra len
        out.extend_from_slice(name_bytes);
        out.extend_from_slice(data);
        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&20u16.to_le_bytes()); // version made by
        central.extend_from_slice(&20u16.to_le_bytes()); // version needed
        central.extend_from_slice(&0u16.to_le_bytes()); // flags
        central.extend_from_slice(&0u16.to_le_bytes()); // method
        central.extend_from_slice(&0u16.to_le_bytes()); // time
        central.extend_from_slice(&0x21u16.to_le_bytes()); // date
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(
            &u32::try_from(data.len())
                .expect("small fixture")
                .to_le_bytes(),
        );
        central.extend_from_slice(
            &u32::try_from(data.len())
                .expect("small fixture")
                .to_le_bytes(),
        );
        central.extend_from_slice(
            &u16::try_from(name_bytes.len())
                .expect("small fixture")
                .to_le_bytes(),
        );
        central.extend_from_slice(&0u16.to_le_bytes()); // extra len
        central.extend_from_slice(&0u16.to_le_bytes()); // comment len
        central.extend_from_slice(&0u16.to_le_bytes()); // disk start
        central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
        central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name_bytes);
    }
    let cd_offset = u32::try_from(out.len()).expect("small fixture");
    out.extend_from_slice(&central);
    out.extend_from_slice(b"PK\x05\x06");
    out.extend_from_slice(&0u16.to_le_bytes()); // disk number
    out.extend_from_slice(&0u16.to_le_bytes()); // cd disk
    out.extend_from_slice(
        &u16::try_from(files.len())
            .expect("small fixture")
            .to_le_bytes(),
    );
    out.extend_from_slice(
        &u16::try_from(files.len())
            .expect("small fixture")
            .to_le_bytes(),
    );
    out.extend_from_slice(
        &u32::try_from(central.len())
            .expect("small fixture")
            .to_le_bytes(),
    );
    out.extend_from_slice(&cd_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // comment len
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, entry) in table.iter_mut().enumerate() {
        let mut crc = i as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                0xEDB8_8320 ^ (crc >> 1)
            } else {
                crc >> 1
            };
        }
        *entry = crc;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc = table[((crc ^ u32::from(*byte)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Build a `.tar.gz` archive by shelling out to the system `tar` (present on
/// Linux/macOS and Windows 10+). Returns `None` when `tar` is unavailable.
pub fn tar_gz_bytes(files: &[(&str, &[u8])]) -> Option<Vec<u8>> {
    let dir = TempDir::new("targz");
    for (name, content) in files {
        let path = dir.path().join(name);
        fs::create_dir_all(path.parent().expect("fixture has a parent")).ok()?;
        fs::write(path, content).ok()?;
    }
    let out = Command::new("tar")
        .args(["-czf", "-", "-C"])
        .arg(dir.path())
        .arg(".")
        .output()
        .ok()?;
    out.status.success().then_some(out.stdout)
}

/// A minimal `package.yaml` valid under the manifest schema.
#[must_use]
pub fn manifest_yaml(name: &str, version: &str) -> String {
    format!(
        "name: {name}\nversion: {version}\ndescription: fixture package\nfiles:\n  - src: file.lua\n    dest: ~~/scripts\n"
    )
}
