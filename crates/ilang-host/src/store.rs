//! The global, per-user, content-addressed cache store.
//!
//! One directory per user holds every cache of the toolchain, so that caches
//! never grow inside projects and worktrees share them: `$I_CACHE_DIR`, else
//! `$XDG_CACHE_HOME/i`, else `~/.cache/i` (`%LocalAppData%\i` on Windows).
//! `I_CACHE=off` turns every cache off.
//!
//! An entry is named by its kind and a key. The caller makes the key from
//! everything the value depends on (the compiler build, the target, the flags
//! and the hashes of every input); the store only keeps bytes. Entries are
//! written to a temporary file and renamed into place, so readers never see a
//! partial entry and need no lock. Every entry carries a digest of its
//! content; an entry that does not match it is removed and reported as a miss.
//!
//! The store is bounded: by default 5 GiB (`I_CACHE_MAX_SIZE`, in bytes or
//! with a K/M/G suffix) and 14 days without use (`I_CACHE_MAX_AGE`, in days).
//! A read marks the entry used by its modification time, at most once an
//! hour. [`Store::maintain`] collects at most once per 24 hours, and at once
//! when the store has grown past 120% of its cap; a collection removes entries
//! unused for longer than the maximum age and then the least recently used
//! ones until the store is at 80% of its cap. Entries used within the last
//! hour are never removed, and a collection runs under an advisory lock.

use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The layout version of the store's directory.
const LAYOUT: &str = "v1";
/// The header of every entry file, followed by the SHA-256 of the content.
const MAGIC: &[u8; 8] = b"ICACHE1\n";
const HEADER: usize = MAGIC.len() + 32;
const HOUR: Duration = Duration::from_secs(60 * 60);
const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// The default cap of the store, 5 GiB.
pub const DEFAULT_MAX_SIZE: u64 = 5 << 30;
/// The default maximum time without use, 14 days.
pub const DEFAULT_MAX_AGE: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// The size and age bounds of a store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_size: u64,
    pub max_age: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            max_age: DEFAULT_MAX_AGE,
        }
    }
}

impl Limits {
    /// The defaults, changed by `I_CACHE_MAX_SIZE` and `I_CACHE_MAX_AGE`
    /// where these are set and valid.
    pub fn from_env() -> Self {
        let mut limits = Self::default();
        if let Some(size) = std::env::var("I_CACHE_MAX_SIZE")
            .ok()
            .and_then(|v| parse_size(&v))
        {
            limits.max_size = size;
        }
        if let Some(days) = std::env::var("I_CACHE_MAX_AGE")
            .ok()
            .and_then(|v| parse_days(&v))
        {
            limits.max_age = days;
        }
        limits
    }
}

/// A size in bytes, with an optional K, M or G suffix (powers of 1024).
pub fn parse_size(text: &str) -> Option<u64> {
    let text = text.trim();
    let (digits, shift) = match text.char_indices().last()? {
        (i, 'k' | 'K') => (&text[..i], 10),
        (i, 'm' | 'M') => (&text[..i], 20),
        (i, 'g' | 'G') => (&text[..i], 30),
        _ => (text, 0),
    };
    digits.trim().parse::<u64>().ok()?.checked_mul(1 << shift)
}

/// A whole number of days.
pub fn parse_days(text: &str) -> Option<Duration> {
    let text = text.trim();
    let text = text.strip_suffix('d').unwrap_or(text);
    let days = text.trim().parse::<u64>().ok()?;
    Some(Duration::from_secs(days.checked_mul(24 * 60 * 60)?))
}

/// Whether `I_CACHE` turns caching off.
pub fn disabled() -> bool {
    std::env::var("I_CACHE").is_ok_and(|v| matches!(v.trim(), "off" | "0" | "false" | "no"))
}

/// The directory of the global store, whether or not caching is on.
pub fn default_dir() -> Option<PathBuf> {
    let nonempty = |name| std::env::var_os(name).filter(|v| !v.is_empty());
    if let Some(dir) = nonempty("I_CACHE_DIR") {
        return Some(dir.into());
    }
    if cfg!(windows)
        && let Some(dir) = nonempty("LOCALAPPDATA")
    {
        return Some(PathBuf::from(dir).join("i"));
    }
    if let Some(dir) = nonempty("XDG_CACHE_HOME") {
        return Some(PathBuf::from(dir).join("i"));
    }
    nonempty("HOME").map(|home| PathBuf::from(home).join(".cache").join("i"))
}

/// What one collection removed and kept.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Collected {
    pub removed_entries: u64,
    pub removed_bytes: u64,
    pub kept_entries: u64,
    pub kept_bytes: u64,
}

/// The entries of one kind.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KindStatus {
    pub kind: String,
    pub entries: u64,
    pub bytes: u64,
}

/// A store and its limits.
#[derive(Clone, Debug)]
pub struct Store {
    root: PathBuf,
    limits: Limits,
}

struct Entry {
    path: PathBuf,
    bytes: u64,
    used: SystemTime,
}

impl Store {
    /// The global store, or `None` when caching is off or no directory is
    /// known. The directory is created when the first entry is written.
    pub fn global() -> Option<Store> {
        if disabled() {
            return None;
        }
        Some(Store::at(default_dir()?, Limits::from_env()))
    }

    /// A store in `dir` with `limits`.
    pub fn at(dir: impl Into<PathBuf>, limits: Limits) -> Store {
        Store {
            root: dir.into(),
            limits,
        }
    }

    /// The directory of the store.
    pub fn dir(&self) -> &Path {
        &self.root
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    fn entries_dir(&self) -> PathBuf {
        self.root.join(LAYOUT)
    }

    fn path(&self, kind: &str, key: &str) -> PathBuf {
        assert!(valid_name(kind), "cache kind {kind:?}");
        let name = if valid_name(key) && key.len() <= 128 {
            key.to_owned()
        } else {
            hex(&Sha256::digest(key.as_bytes()))
        };
        let shard = &name[..name.len().min(2)];
        self.entries_dir().join(kind).join(shard).join(name)
    }

    /// The content stored under `kind` and `key`. A damaged entry is removed
    /// and is a miss. A hit marks the entry used.
    pub fn get(&self, kind: &str, key: &str) -> Option<Vec<u8>> {
        let path = self.path(kind, key);
        let bytes = fs::read(&path).ok()?;
        let content = match verified(&bytes) {
            Some(content) => content,
            None => {
                let _ = fs::remove_file(&path);
                return None;
            }
        };
        touch(&path);
        Some(content.to_vec())
    }

    /// Stores `content` under `kind` and `key`, replacing any earlier entry.
    /// Returns whether it was stored; a failure only loses the entry.
    pub fn put(&self, kind: &str, key: &str, content: &[u8]) -> bool {
        let path = self.path(kind, key);
        let Some(dir) = path.parent() else {
            return false;
        };
        if fs::create_dir_all(dir).is_err() {
            return false;
        }
        self.mark_root();
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let temporary = dir.join(format!(
            ".tmp.{}.{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let written = (|| {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(MAGIC)?;
            file.write_all(&Sha256::digest(content))?;
            file.write_all(content)?;
            file.flush()?;
            drop(file);
            fs::rename(&temporary, &path)
        })();
        if written.is_err() {
            let _ = fs::remove_file(&temporary);
            return false;
        }
        WROTE.store(true, std::sync::atomic::Ordering::Relaxed);
        true
    }

    /// Marks the store's directory as a cache for backup and indexing tools.
    fn mark_root(&self) {
        let tag = self.root.join("CACHEDIR.TAG");
        if !tag.exists() {
            write_cachedir_tag(&self.root);
        }
    }

    fn all_entries(&self) -> (Vec<Entry>, Vec<(PathBuf, SystemTime)>) {
        let mut entries = Vec::new();
        let mut temporaries = Vec::new();
        let Ok(kinds) = fs::read_dir(self.entries_dir()) else {
            return (entries, temporaries);
        };
        for kind in kinds.flatten() {
            let Ok(shards) = fs::read_dir(kind.path()) else {
                continue;
            };
            for shard in shards.flatten() {
                let Ok(files) = fs::read_dir(shard.path()) else {
                    continue;
                };
                for file in files.flatten() {
                    let Ok(meta) = file.metadata() else { continue };
                    if !meta.is_file() {
                        continue;
                    }
                    let used = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    if file.file_name().to_string_lossy().starts_with(".tmp.") {
                        temporaries.push((file.path(), used));
                    } else {
                        entries.push(Entry {
                            path: file.path(),
                            bytes: meta.len(),
                            used,
                        });
                    }
                }
            }
        }
        (entries, temporaries)
    }

    /// The entries and bytes of each kind, by kind name.
    pub fn status(&self) -> Vec<KindStatus> {
        let mut kinds: Vec<KindStatus> = Vec::new();
        for entry in self.all_entries().0 {
            let kind = entry
                .path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            match kinds.iter_mut().find(|k| k.kind == kind) {
                Some(k) => {
                    k.entries += 1;
                    k.bytes += entry.bytes;
                }
                None => kinds.push(KindStatus {
                    kind,
                    entries: 1,
                    bytes: entry.bytes,
                }),
            }
        }
        kinds.sort_by(|a, b| a.kind.cmp(&b.kind));
        kinds
    }

    /// Removes entries unused for longer than the maximum age, then the least
    /// recently used ones until the store is at most 80% of its cap. Entries
    /// used within the last hour stay. Returns `None` when another process is
    /// collecting.
    pub fn gc(&self) -> Option<Collected> {
        let _lock = self.lock(false)?;
        let collected = self.collect(SystemTime::now());
        let _ = fs::write(self.root.join("gc.stamp"), b"");
        Some(collected)
    }

    fn collect(&self, now: SystemTime) -> Collected {
        let (mut entries, temporaries) = self.all_entries();
        let age = |time: SystemTime| now.duration_since(time).unwrap_or_default();
        for (path, time) in temporaries {
            if age(time) > HOUR {
                let _ = fs::remove_file(path);
            }
        }
        let mut collected = Collected::default();
        let remove = |entry: &Entry, collected: &mut Collected| {
            if fs::remove_file(&entry.path).is_ok() {
                collected.removed_entries += 1;
                collected.removed_bytes += entry.bytes;
            }
        };
        entries.retain(|entry| {
            if age(entry.used) > self.limits.max_age.max(HOUR) {
                remove(entry, &mut collected);
                false
            } else {
                true
            }
        });
        let mut total: u64 = entries.iter().map(|e| e.bytes).sum();
        let target = self.limits.max_size / 5 * 4;
        if total > self.limits.max_size {
            entries.sort_by_key(|e| e.used);
            entries.retain(|entry| {
                if total > target && age(entry.used) > HOUR {
                    total -= entry.bytes;
                    remove(entry, &mut collected);
                    false
                } else {
                    true
                }
            });
        }
        collected.kept_entries = entries.len() as u64;
        collected.kept_bytes = entries.iter().map(|e| e.bytes).sum();
        collected
    }

    /// Removes every entry, or only those of the given kinds. Returns the
    /// number of entries and bytes removed; `None` while another process
    /// holds the store's lock.
    pub fn clean(&self, kinds: Option<&[&str]>) -> Option<Collected> {
        let _lock = self.lock(true)?;
        let mut collected = Collected::default();
        let (entries, temporaries) = self.all_entries();
        for entry in entries {
            let kind = entry
                .path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            if kinds.is_some_and(|kinds| !kinds.contains(&kind.as_str())) {
                collected.kept_entries += 1;
                collected.kept_bytes += entry.bytes;
                continue;
            }
            if fs::remove_file(&entry.path).is_ok() {
                collected.removed_entries += 1;
                collected.removed_bytes += entry.bytes;
            }
        }
        if kinds.is_none() {
            for (path, _) in temporaries {
                let _ = fs::remove_file(path);
            }
        }
        Some(collected)
    }

    /// Collects when the last collection is more than 24 hours old, or at
    /// once when this process wrote entries and the store is above 120% of
    /// its cap. Meant to run once, after a command has done its work.
    pub fn maintain(&self) {
        let stamp = self.root.join("gc.stamp");
        let due = match fs::metadata(&stamp).and_then(|m| m.modified()) {
            Ok(time) => SystemTime::now()
                .duration_since(time)
                .is_ok_and(|age| age > DAY),
            // A store that has entries but no stamp has never been collected.
            Err(_) => self.entries_dir().is_dir(),
        };
        let over = || {
            let total: u64 = self.all_entries().0.iter().map(|e| e.bytes).sum();
            total > self.limits.max_size / 5 * 6
        };
        if due || (WROTE.load(std::sync::atomic::Ordering::Relaxed) && over()) {
            self.gc();
        }
    }

    /// The advisory lock that serializes collections. `wait` blocks for it.
    fn lock(&self, wait: bool) -> Option<fs::File> {
        fs::create_dir_all(&self.root).ok()?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join("gc.lock"))
            .ok()?;
        if wait {
            file.lock().ok()?;
        } else {
            file.try_lock().ok()?;
        }
        Some(file)
    }
}

/// Whether this process stored an entry, so that `maintain` checks the size.
static WROTE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Runs [`Store::maintain`] on the global store, if caching is on.
pub fn maintain_global() {
    if let Some(store) = Store::global() {
        store.maintain();
    }
}

/// Writes a `CACHEDIR.TAG` (bford.ca/cachedir) into `dir`, creating `dir`.
/// Tools that honour the tag skip the directory in backups.
pub fn write_cachedir_tag(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    let tag = dir.join("CACHEDIR.TAG");
    if tag.exists() {
        return;
    }
    let _ = fs::write(
        tag,
        "Signature: 8a477f597d28d172789f06886806bc55\n\
         # This file is a cache directory tag created by the i toolchain.\n\
         # For information about cache directory tags see https://bford.ca/cachedir/\n",
    );
}

/// The content of an entry file whose digest matches, if any.
fn verified(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < HEADER || &bytes[..MAGIC.len()] != MAGIC {
        return None;
    }
    let content = &bytes[HEADER..];
    (Sha256::digest(content).as_slice() == &bytes[MAGIC.len()..HEADER]).then_some(content)
}

/// Marks an entry used, at most once an hour (its modification time).
fn touch(path: &Path) {
    let now = SystemTime::now();
    let Ok(meta) = fs::metadata(path) else { return };
    let recent = meta
        .modified()
        .ok()
        .and_then(|t| now.duration_since(t).ok())
        .is_some_and(|age| age < HOUR);
    if !recent && let Ok(file) = fs::File::options().write(true).open(path) {
        let _ = file.set_modified(now);
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Lowercase hexadecimal.
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A key from length-prefixed parts: the SHA-256 of all of them, in hex.
/// Callers pass the compiler build, target, flags and input hashes.
pub fn key(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hex(&hasher.finalize())
}

#[cfg(test)]
mod store_tests {
    use super::*;

    /// A fresh directory, removed when dropped.
    struct TempDir(PathBuf);
    impl TempDir {
        fn path(&self) -> &Path {
            &self.0
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn tempdir() -> TempDir {
        static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "ilang-store-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn store(dir: &Path, max_size: u64) -> Store {
        Store::at(
            dir,
            Limits {
                max_size,
                max_age: DEFAULT_MAX_AGE,
            },
        )
    }

    fn set_age(store: &Store, kind: &str, key: &str, age: Duration) {
        let file = fs::File::options()
            .write(true)
            .open(store.path(kind, key))
            .unwrap();
        file.set_modified(SystemTime::now() - age).unwrap();
    }

    #[test]
    fn put_then_get_round_trips_and_misses_otherwise() {
        let dir = tempdir();
        let s = store(dir.path(), DEFAULT_MAX_SIZE);
        assert_eq!(s.get("k", "a"), None);
        assert!(s.put("k", "a", b"hello"));
        assert_eq!(s.get("k", "a").as_deref(), Some(&b"hello"[..]));
        assert_eq!(s.get("other", "a"), None);
        assert!(s.put("k", "a", b"again"));
        assert_eq!(s.get("k", "a").as_deref(), Some(&b"again"[..]));
        assert!(dir.path().join("CACHEDIR.TAG").is_file());
        // Keys that are not plain names are hashed into one.
        assert!(s.put("k", "../a b", b"x"));
        assert_eq!(s.get("k", "../a b").as_deref(), Some(&b"x"[..]));
    }

    #[test]
    fn damaged_entries_are_removed_and_miss() {
        let dir = tempdir();
        let s = store(dir.path(), DEFAULT_MAX_SIZE);
        assert!(s.put("k", "a", b"hello"));
        let path = s.path("k", "a");
        let mut bytes = fs::read(&path).unwrap();
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&path, &bytes).unwrap();
        assert_eq!(s.get("k", "a"), None);
        assert!(!path.exists());
        fs::write(&path, b"short").unwrap();
        assert_eq!(s.get("k", "a"), None);
        assert!(!path.exists());
    }

    #[test]
    fn gc_removes_old_entries_then_least_recently_used_to_80_percent() {
        let dir = tempdir();
        // Ten entries of 1000 bytes (plus header) against a cap of 6000.
        let s = store(dir.path(), 6000);
        for n in 0..10 {
            assert!(s.put("k", &format!("e{n}"), &[n as u8; 1000]));
            set_age(&s, "k", &format!("e{n}"), HOUR * (20 - n) * 2);
        }
        // One entry is older than the maximum age.
        set_age(&s, "k", "e9", DEFAULT_MAX_AGE + DAY);
        // One entry was used within the hour: it stays even though it is old.
        assert!(s.put("k", "fresh", &[1; 1000]));
        let collected = s.gc().unwrap();
        assert!(s.get("k", "e9").is_none(), "older than the maximum age");
        assert!(collected.kept_bytes <= 4800, "{collected:?}");
        assert!(s.get("k", "fresh").is_some());
        // The least recently used entries went first.
        assert!(s.get("k", "e0").is_none());
        assert!(s.get("k", "e8").is_some());
        assert!(dir.path().join("gc.stamp").is_file());
    }

    #[test]
    fn gc_never_removes_entries_used_within_the_hour() {
        let dir = tempdir();
        let s = store(dir.path(), 100);
        for n in 0..5 {
            assert!(s.put("k", &format!("e{n}"), &[0; 1000]));
        }
        let collected = s.gc().unwrap();
        assert_eq!(collected.removed_entries, 0);
        assert_eq!(collected.kept_entries, 5);
    }

    #[test]
    fn gc_skips_while_another_process_holds_the_lock() {
        let dir = tempdir();
        let s = store(dir.path(), 100);
        let held = s.lock(true).unwrap();
        assert!(s.gc().is_none());
        drop(held);
        // Another test thread may be between fork and exec of a child process at this moment: that
        // child holds a copy of the lock's descriptor until its exec closes it. Wait for it.
        let collected = (0..200).find_map(|_| {
            s.gc().or_else(|| {
                std::thread::sleep(std::time::Duration::from_millis(10));
                None
            })
        });
        assert!(collected.is_some());
    }

    #[test]
    fn clean_removes_all_or_one_kind_and_status_counts() {
        let dir = tempdir();
        let s = store(dir.path(), DEFAULT_MAX_SIZE);
        assert!(s.put("a", "1", b"x"));
        assert!(s.put("a", "2", b"y"));
        assert!(s.put("b", "1", b"z"));
        let status = s.status();
        assert_eq!(
            status
                .iter()
                .map(|k| (k.kind.as_str(), k.entries))
                .collect::<Vec<_>>(),
            [("a", 2), ("b", 1)]
        );
        let removed = s.clean(Some(&["a"])).unwrap();
        assert_eq!(removed.removed_entries, 2);
        assert!(s.get("b", "1").is_some());
        s.clean(None).unwrap();
        assert!(s.status().is_empty());
    }

    #[test]
    fn reads_mark_entries_used_at_most_hourly() {
        let dir = tempdir();
        let s = store(dir.path(), DEFAULT_MAX_SIZE);
        assert!(s.put("k", "a", b"x"));
        let path = s.path("k", "a");
        let modified = || fs::metadata(&path).unwrap().modified().unwrap();
        set_age(&s, "k", "a", HOUR / 2);
        let before = modified();
        s.get("k", "a").unwrap();
        assert_eq!(modified(), before, "used within the hour: not touched");
        set_age(&s, "k", "a", DAY);
        s.get("k", "a").unwrap();
        assert!(SystemTime::now().duration_since(modified()).unwrap() < HOUR);
    }

    #[test]
    fn limits_parse() {
        assert_eq!(parse_size("5G"), Some(5 << 30));
        assert_eq!(parse_size("512m"), Some(512 << 20));
        assert_eq!(parse_size("1000"), Some(1000));
        assert_eq!(parse_size("x"), None);
        assert_eq!(parse_days("14"), Some(DEFAULT_MAX_AGE));
        assert_eq!(parse_days("14d"), Some(DEFAULT_MAX_AGE));
    }
}
