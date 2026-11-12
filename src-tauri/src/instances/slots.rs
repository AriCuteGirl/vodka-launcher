//! Where each running game's copy of Mocktail is started from.
//!
//! Mocktail allows one process per executable path. Its Flatpak ships the binary at two
//! paths, which covers the first two games. Every further game gets its own copy of the
//! binary inside the Flatpak's data folder, laid out the way Mocktail expects
//! (`bin/mocktail`, with `lib/mocktail` and `share` pointing back at the real install).

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// More than this is refused: every game needs about 1.5 GB of memory.
pub const MAX_INSTANCES: usize = 10;
const BUILT_IN: [Option<&str>; 2] = [None, Some("/app/lib/mocktail/mocktail")];

/// A running game's hold on its slot and its account. Other Vodka windows see both as taken
/// until this is dropped (or this Vodka exits: the operating system releases file locks itself).
#[derive(Debug)]
pub struct Claim {
    pub slot: usize,
    _slot_lock: File,
    _account_lock: File,
}

#[derive(Debug)]
pub enum ClaimError {
    /// That account already has a game, in this window or another.
    AccountBusy,
    /// Every slot is in use.
    Full,
    Io(io::Error),
}

/// Where the lock files live: the user's runtime folder, which is cleared at logout.
pub fn lock_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("vodka-launcher")
}

fn try_lock(path: &Path) -> io::Result<Option<File>> {
    let file = OpenOptions::new().create(true).truncate(false).write(true).open(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(e)) => Err(e),
    }
}

/// Reserves the account and the lowest free slot, across every Vodka window of this user.
pub fn claim(dir: &Path, account_id: u64) -> Result<Claim, ClaimError> {
    std::fs::create_dir_all(dir).map_err(ClaimError::Io)?;
    let account = try_lock(&dir.join(format!("account-{account_id}.lock"))).map_err(ClaimError::Io)?.ok_or(ClaimError::AccountBusy)?;
    for slot in 0..MAX_INSTANCES {
        if let Some(lock) = try_lock(&dir.join(format!("slot-{slot}.lock"))).map_err(ClaimError::Io)? {
            return Ok(Claim { slot, _slot_lock: lock, _account_lock: account });
        }
    }
    Err(ClaimError::Full)
}

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
    fn claims_are_exclusive_and_freed_when_a_game_ends() {
        let dir = std::env::temp_dir().join(format!("vodka-claims-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // Each claim opens its own file handles, exactly as a second Vodka window would.
        let first = claim(&dir, 1).unwrap();
        assert_eq!(first.slot, 0);
        assert!(matches!(claim(&dir, 1), Err(ClaimError::AccountBusy)));
        let second = claim(&dir, 2).unwrap();
        assert_eq!(second.slot, 1);
        // The first game closes: its slot and account are free again, the other is untouched.
        drop(first);
        let third = claim(&dir, 3).unwrap();
        assert_eq!(third.slot, 0);
        assert!(claim(&dir, 1).is_ok());
        assert!(matches!(claim(&dir, 2), Err(ClaimError::AccountBusy)));
        drop((second, third));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn there_are_only_so_many_slots() {
        let dir = std::env::temp_dir().join(format!("vodka-claims-full-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let held: Vec<Claim> = (0..MAX_INSTANCES as u64).map(|id| claim(&dir, id + 1).unwrap()).collect();
        assert_eq!(held.iter().map(|c| c.slot).collect::<Vec<_>>(), (0..MAX_INSTANCES).collect::<Vec<_>>());
        assert!(matches!(claim(&dir, 99), Err(ClaimError::Full)));
        drop(held);
        std::fs::remove_dir_all(dir).unwrap();
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
