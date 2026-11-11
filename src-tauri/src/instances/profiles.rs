//! One isolated Mocktail profile per account: its own data, cache and state
//! folders, so two accounts never share app data or a saved session.
//!
//! The Roblox client files are hard-linked from Mocktail's own profile, so a
//! new profile costs almost no disk space.

use crate::mocktail::detect::APP_ID;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct ProfilePaths {
    pub root: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
    pub state: PathBuf,
}

/// The Flatpak's private data folder; profiles must live inside it to be visible in the sandbox.
pub fn flatpak_data(home: &Path) -> PathBuf {
    home.join(".var/app").join(APP_ID).join("data")
}

pub fn profile_paths(flatpak_data: &Path, account_id: u64) -> ProfilePaths {
    let root = flatpak_data.join("vodka/profiles").join(account_id.to_string());
    ProfilePaths { data: root.join("data"), cache: root.join("cache"), state: root.join("state"), root }
}

/// Hard-links every file under `from` into `to` (copying if linking isn't possible),
/// keeping directory permissions, which Mocktail sets read-only on client files.
fn link_tree(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for item in std::fs::read_dir(from)? {
        let item = item?;
        let target = to.join(item.file_name());
        let kind = item.file_type()?;
        if kind.is_dir() {
            link_tree(&item.path(), &target)?;
        } else if kind.is_file() && std::fs::hard_link(item.path(), &target).is_err() {
            std::fs::copy(item.path(), &target)?;
        }
    }
    std::fs::set_permissions(to, std::fs::metadata(from)?.permissions())
}

fn copy_tree(from: &Path, to: &Path) -> io::Result<()> {
    std::fs::create_dir_all(to)?;
    for item in std::fs::read_dir(from)? {
        let item = item?;
        let target = to.join(item.file_name());
        if item.file_type()?.is_dir() {
            copy_tree(&item.path(), &target)?;
        } else {
            std::fs::copy(item.path(), &target)?;
        }
    }
    Ok(())
}

/// Deletes a folder even if it contains read-only directories.
/// Only directories are made writable: files may be hard links shared with Mocktail's own profile.
pub fn remove_tree(path: &Path) -> io::Result<()> {
    fn unlock(dir: &Path) -> io::Result<()> {
        let mut perms = std::fs::metadata(dir)?.permissions();
        perms.set_mode(perms.mode() | 0o700);
        std::fs::set_permissions(dir, perms)?;
        for item in std::fs::read_dir(dir)? {
            let item = item?;
            if item.file_type()?.is_dir() {
                unlock(&item.path())?;
            }
        }
        Ok(())
    }
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
        Ok(_) => {
            unlock(path)?;
            std::fs::remove_dir_all(path)
        }
    }
}

/// Makes sure the profile exists and has a Roblox client. Existing profiles are left alone:
/// Mocktail keeps each one up to date by itself.
pub fn prepare(main_data: &Path, profile: &ProfilePaths) -> io::Result<()> {
    for dir in [&profile.data, &profile.cache, &profile.state] {
        std::fs::create_dir_all(dir)?;
    }
    if profile.data.join("current.json").is_file() {
        return Ok(());
    }
    if !main_data.join("current.json").is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Mocktail hasn't downloaded Roblox yet. Start Mocktail once on its own, then try again.",
        ));
    }
    remove_tree(&profile.data.join("payloads"))?;
    link_tree(&main_data.join("payloads"), &profile.data.join("payloads"))?;
    for dir in ["approvals", "compatibility_profiles", "host_abi_profiles"] {
        if main_data.join(dir).is_dir() {
            copy_tree(&main_data.join(dir), &profile.data.join(dir))?;
        }
    }
    for file in ["previous_good.json", "update.json"] {
        if main_data.join(file).is_file() {
            std::fs::copy(main_data.join(file), profile.data.join(file))?;
        }
    }
    // Written last: its presence marks the profile as complete.
    std::fs::copy(main_data.join("current.json"), profile.data.join("current.json")).map(|_| ())
}

/// Mocktail writes the session it was given into the profile. Vodka removes that copy
/// when the game closes, so the keyring stays the only place it is kept.
pub fn clear_session_copy(profile: &ProfilePaths) {
    let _ = std::fs::remove_file(profile.data.join("auth/roblox.cookie"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::MetadataExt;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("vodka-profiles-{name}-{}", std::process::id()));
        let _ = remove_tree(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn fake_main(dir: &Path) -> PathBuf {
        let main = dir.join("mocktail");
        let payload = main.join("payloads/1-abc/assets");
        std::fs::create_dir_all(&payload).unwrap();
        std::fs::write(payload.join("big.bin"), b"client").unwrap();
        std::fs::create_dir_all(main.join("approvals")).unwrap();
        std::fs::write(main.join("approvals/a.json"), b"{}").unwrap();
        std::fs::write(main.join("current.json"), br#"{"version_name":"1"}"#).unwrap();
        // Mocktail makes payload folders read-only.
        std::fs::set_permissions(&payload, std::fs::Permissions::from_mode(0o555)).unwrap();
        main
    }

    #[test]
    fn profiles_of_two_accounts_share_nothing() {
        let base = Path::new("/x/data");
        let (a, b) = (profile_paths(base, 1), profile_paths(base, 2));
        for (pa, pb) in [(&a.data, &b.data), (&a.cache, &b.cache), (&a.state, &b.state)] {
            assert_ne!(pa, pb);
            assert!(!pa.starts_with(&b.root) && !pb.starts_with(&a.root));
        }
        assert!(a.root.starts_with(base));
    }

    #[test]
    fn prepare_links_the_client_instead_of_copying_it() {
        let dir = scratch("link");
        let main = fake_main(&dir);
        let profile = profile_paths(&dir, 7);
        prepare(&main, &profile).unwrap();
        let original = std::fs::metadata(main.join("payloads/1-abc/assets/big.bin")).unwrap();
        let linked = std::fs::metadata(profile.data.join("payloads/1-abc/assets/big.bin")).unwrap();
        assert_eq!(original.ino(), linked.ino());
        assert_eq!(std::fs::metadata(profile.data.join("payloads/1-abc/assets")).unwrap().mode() & 0o777, 0o555);
        assert!(profile.data.join("approvals/a.json").is_file());
        assert!(profile.cache.is_dir() && profile.state.is_dir());
        // The session folder is never copied from the main profile.
        assert!(!profile.data.join("auth").exists());
        remove_tree(&dir).unwrap();
    }

    #[test]
    fn prepare_leaves_an_existing_profile_alone() {
        let dir = scratch("keep");
        let main = fake_main(&dir);
        let profile = profile_paths(&dir, 7);
        prepare(&main, &profile).unwrap();
        std::fs::write(profile.data.join("current.json"), b"updated by mocktail").unwrap();
        prepare(&main, &profile).unwrap();
        assert_eq!(std::fs::read(profile.data.join("current.json")).unwrap(), b"updated by mocktail");
        remove_tree(&dir).unwrap();
    }

    #[test]
    fn prepare_explains_a_missing_client() {
        let dir = scratch("missing");
        let err = prepare(&dir.join("nope"), &profile_paths(&dir, 7)).unwrap_err();
        assert!(err.to_string().contains("Start Mocktail once"));
        remove_tree(&dir).unwrap();
    }

    #[test]
    fn removing_a_profile_keeps_the_original_client() {
        let dir = scratch("remove");
        let main = fake_main(&dir);
        let profile = profile_paths(&dir, 7);
        prepare(&main, &profile).unwrap();
        std::fs::create_dir_all(profile.data.join("auth")).unwrap();
        std::fs::write(profile.data.join("auth/roblox.cookie"), b"secret").unwrap();
        clear_session_copy(&profile);
        assert!(!profile.data.join("auth/roblox.cookie").exists());
        remove_tree(&profile.root).unwrap();
        assert!(!profile.root.exists());
        let original = main.join("payloads/1-abc/assets/big.bin");
        assert_eq!(std::fs::read(&original).unwrap(), b"client");
        assert_eq!(std::fs::metadata(original.parent().unwrap()).unwrap().mode() & 0o777, 0o555);
        remove_tree(&dir).unwrap();
        // Removing something that isn't there is fine.
        remove_tree(&dir).unwrap();
    }
}
