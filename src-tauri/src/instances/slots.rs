//! Where each running game's copy of Mocktail is started from.
//!
//! Mocktail allows one process per executable path. Its Flatpak ships the binary at two
//! paths, which covers the first two games. Every further game gets its own copy of the
//! binary inside the Flatpak's data folder, laid out the way Mocktail expects
//! (`bin/mocktail`, with `lib/mocktail` and `share` pointing back at the real install).

use std::io;
use std::path::{Path, PathBuf};

/// More than this is refused: every game needs about 1.5 GB of memory.
pub const MAX_INSTANCES: usize = 10;
const BUILT_IN: [Option<&str>; 2] = [None, Some("/app/lib/mocktail/mocktail")];

pub fn slot_dir(flatpak_data: &Path, slot: usize) -> PathBuf {
    flatpak_data.join("vodka/slots").join(slot.to_string())
}

/// The `--command` to start this slot with. `None` means the Flatpak's default command.
pub fn command(flatpak_data: &Path, slot: usize) -> Option<String> {
    match BUILT_IN.get(slot) {
        Some(built_in) => built_in.map(str::to_owned),
        None => Some(slot_dir(flatpak_data, slot).join("bin/mocktail").to_string_lossy().into_owned()),
    }
}

pub fn needs_copy(slot: usize) -> bool {
    slot >= BUILT_IN.len()
}

fn link(target: &str, at: &Path) -> io::Result<()> {
    match std::fs::read_link(at) {
        Ok(existing) if existing == Path::new(target) => return Ok(()),
        Ok(_) => std::fs::remove_file(at)?,
        Err(_) => {}
    }
    std::os::unix::fs::symlink(target, at)
}

/// Makes sure a copied slot exists and matches the installed Mocktail (it is copied again after an update).
pub fn prepare(installed_binary: &Path, dir: &Path) -> io::Result<()> {
    std::fs::create_dir_all(dir.join("bin"))?;
    std::fs::create_dir_all(dir.join("lib"))?;
    let copy = dir.join("bin/mocktail");
    let source = std::fs::metadata(installed_binary)?;
    let fresh = std::fs::metadata(&copy).is_ok_and(|c| c.len() == source.len() && c.modified().ok() >= source.modified().ok());
    if !fresh {
        // Copy to a temporary name first so a half-written binary is never started.
        let tmp = dir.join("bin/mocktail.tmp");
        std::fs::copy(installed_binary, &tmp)?;
        std::fs::rename(tmp, &copy)?;
    }
    // These paths exist inside the Flatpak sandbox, where the copy runs.
    link("/app/lib/mocktail", &dir.join("lib/mocktail"))?;
    link("/app/share", &dir.join("share"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_slot_starts_from_a_different_path() {
        let data = Path::new("/d");
        let commands: Vec<Option<String>> = (0..MAX_INSTANCES).map(|s| command(data, s)).collect();
        for (i, a) in commands.iter().enumerate() {
            for b in &commands[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert_eq!(commands[0], None);
        assert_eq!(commands[2].as_deref(), Some("/d/vodka/slots/2/bin/mocktail"));
        assert!(!needs_copy(1) && needs_copy(2));
    }

    #[test]
    fn prepares_the_layout_mocktail_expects_and_refreshes_after_an_update() {
        let dir = std::env::temp_dir().join(format!("vodka-slots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let installed = dir.join("installed");
        std::fs::write(&installed, b"v1").unwrap();
        let slot = dir.join("slot");

        prepare(&installed, &slot).unwrap();
        assert_eq!(std::fs::read(slot.join("bin/mocktail")).unwrap(), b"v1");
        assert_eq!(std::fs::read_link(slot.join("lib/mocktail")).unwrap(), Path::new("/app/lib/mocktail"));
        assert_eq!(std::fs::read_link(slot.join("share")).unwrap(), Path::new("/app/share"));

        // Running it again changes nothing and doesn't fail on the existing links.
        prepare(&installed, &slot).unwrap();
        // A Mocktail update (different size) is picked up.
        std::fs::write(&installed, b"version two").unwrap();
        prepare(&installed, &slot).unwrap();
        assert_eq!(std::fs::read(slot.join("bin/mocktail")).unwrap(), b"version two");
        assert!(prepare(&dir.join("missing"), &slot).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
