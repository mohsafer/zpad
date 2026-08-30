//! Plain-file persistence: one UTF-8 `.txt` per note plus a small `state.toml`
//! for window geometry. All writes are atomic (temp file + fsync + rename) so
//! a crash or power loss can never corrupt an existing note.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

const NOTES_DIR: &str = "notes";
const STATE_FILE: &str = "state.toml";
const STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteFile {
    pub id: u64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct StateFile {
    version: u32,
    note: Vec<NoteGeom>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NoteGeom {
    id: u64,
    width: i32,
    height: i32,
}

/// Owns the data directory and all file I/O. Cheap to clone; every method
/// takes `&self` and re-derives paths from the stored base.
#[derive(Debug, Clone)]
pub struct Store {
    base: PathBuf,
}

impl Store {
    pub fn new(base: PathBuf) -> Self {
        Store { base }
    }

    /// `~/.local/share/zpad`, honoring `XDG_DATA_HOME` when it is absolute.
    pub fn default_dir() -> io::Result<PathBuf> {
        let base = match std::env::var_os("XDG_DATA_HOME") {
            Some(v) if !v.is_empty() => {
                let p = PathBuf::from(v);
                if p.is_absolute() {
                    p
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "XDG_DATA_HOME must be an absolute path",
                    ));
                }
            }
            _ => {
                let home = std::env::var_os("HOME").ok_or_else(|| {
                    io::Error::new(io::ErrorKind::NotFound, "HOME is not set")
                })?;
                PathBuf::from(home).join(".local").join("share")
            }
        };
        Ok(base.join("zpad"))
    }

    pub fn ensure_dirs(&self) -> io::Result<()> {
        fs::create_dir_all(self.notes_dir())
    }

    fn notes_dir(&self) -> PathBuf {
        self.base.join(NOTES_DIR)
    }

    fn note_path(&self, id: u64) -> PathBuf {
        self.notes_dir().join(format!("{id}.txt"))
    }

    /// All notes on disk, ordered by id. Unreadable or non-note files are
    /// skipped rather than failing the whole session; a missing directory is
    /// simply "no notes" (fresh install).
    pub fn load_notes(&self) -> io::Result<Vec<NoteFile>> {
        let mut notes = Vec::new();
        let dir = self.notes_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(notes),
            Err(err) => return Err(err),
        };
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                continue;
            }
            let Ok(id) = stem.parse::<u64>() else {
                continue;
            };
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(err) => {
                    eprintln!("zpad: skipping note {id}: {err}");
                    continue;
                }
            };
            notes.push(NoteFile {
                id,
                text: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        notes.sort_by_key(|n| n.id);
        Ok(notes)
    }

    /// Highest existing id + 1, so ids never collide even after manual
    /// tampering with the notes directory.
    pub fn next_id(&self) -> u64 {
        self.load_notes()
            .map(|notes| notes.iter().map(|n| n.id).max().unwrap_or(0) + 1)
            .unwrap_or(1)
    }

    pub fn save_note(&self, id: u64, text: &str) -> io::Result<()> {
        self.ensure_dirs()?;
        atomic_write(&self.note_path(id), text.as_bytes())
    }

    pub fn delete_note(&self, id: u64) -> io::Result<()> {
        match fs::remove_file(self.note_path(id)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// Saved window sizes keyed by note id. A missing or corrupt state file
    /// is not an error: geometry is a nicety, notes themselves are the data.
    pub fn load_sizes(&self) -> HashMap<u64, (i32, i32)> {
        let Ok(raw) = fs::read_to_string(self.base.join(STATE_FILE)) else {
            return HashMap::new();
        };
        let Ok(state) = toml::from_str::<StateFile>(&raw) else {
            eprintln!("zpad: {STATE_FILE} is corrupt, ignoring saved sizes");
            return HashMap::new();
        };
        state
            .note
            .into_iter()
            .map(|g| (g.id, (g.width, g.height)))
            .collect()
    }

    pub fn save_sizes(&self, sizes: &HashMap<u64, (i32, i32)>) -> io::Result<()> {
        self.ensure_dirs()?;
        let state = StateFile {
            version: STATE_VERSION,
            note: sizes
                .iter()
                .map(|(&id, &(width, height))| NoteGeom { id, width, height })
                .collect(),
        };
        let raw = toml::to_string(&state).map_err(|err| {
            io::Error::new(io::ErrorKind::InvalidData, format!("toml: {err}"))
        })?;
        atomic_write(&self.base.join(STATE_FILE), raw.as_bytes())
    }
}

/// Write to a sibling temp file, fsync it, rename over the target, then fsync
/// the directory so the rename itself survives a power loss.
fn atomic_write(path: &std::path::Path, data: &[u8]) -> io::Result<()> {
    let dir = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "path has no parent directory")
    })?;
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");

    let file = File::create(dir.join(&tmp))?;
    {
        let mut writer = io::BufWriter::new(&file);
        writer.write_all(data)?;
        writer.flush()?;
    }
    file.sync_all()?;
    drop(file);

    match fs::rename(dir.join(&tmp), path) {
        Ok(()) => {}
        Err(err) => {
            let _ = fs::remove_file(dir.join(&tmp));
            return Err(err);
        }
    }

    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "zpad-test-{}-{n}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            TempDir(path)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn note_roundtrip() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        store.save_note(1, "hello\nworld\n").unwrap();
        let notes = store.load_notes().unwrap();
        assert_eq!(notes, vec![NoteFile { id: 1, text: "hello\nworld\n".into() }]);
    }

    #[test]
    fn save_overwrites_atomically() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        store.save_note(2, "first").unwrap();
        store.save_note(2, "second").unwrap();
        assert_eq!(store.load_notes().unwrap()[0].text, "second");
        assert!(!tmp.0.join("notes/2.txt.tmp").exists());
    }

    #[test]
    fn delete_removes_note_and_is_idempotent() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        store.save_note(3, "bye").unwrap();
        store.delete_note(3).unwrap();
        assert!(store.load_notes().unwrap().is_empty());
        store.delete_note(3).unwrap(); // second delete must not error
    }

    #[test]
    fn next_id_grows_past_max() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        assert_eq!(store.next_id(), 1);
        store.save_note(7, "seven").unwrap();
        assert_eq!(store.next_id(), 8);
    }

    #[test]
    fn sizes_roundtrip() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        let mut sizes = HashMap::new();
        sizes.insert(1u64, (260, 220));
        sizes.insert(4u64, (400, 300));
        store.save_sizes(&sizes).unwrap();
        assert_eq!(store.load_sizes(), sizes);
    }

    #[test]
    fn corrupt_state_yields_empty_not_error() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        fs::create_dir_all(&tmp.0).unwrap();
        fs::write(tmp.0.join(STATE_FILE), "not [ valid toml {{{{").unwrap();
        assert!(store.load_sizes().is_empty());
    }

    #[test]
    fn missing_everything_is_empty() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        assert!(store.load_notes().unwrap().is_empty());
        assert!(store.load_sizes().is_empty());
    }

    #[test]
    fn non_note_files_are_skipped() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        store.ensure_dirs().unwrap();
        fs::write(tmp.0.join("notes/5.txt"), "real").unwrap();
        fs::write(tmp.0.join("notes/readme.md"), "nope").unwrap();
        fs::write(tmp.0.join("notes/notes.bak"), "nope").unwrap();
        let notes = store.load_notes().unwrap();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].id, 5);
    }

    #[test]
    fn invalid_utf8_loads_lossily() {
        let tmp = TempDir::new();
        let store = Store::new(tmp.0.clone());
        store.ensure_dirs().unwrap();
        fs::write(tmp.0.join("notes/9.txt"), [0x68, 0x69, 0xFF]).unwrap();
        let notes = store.load_notes().unwrap();
        assert_eq!(notes[0].text, "hi\u{FFFD}");
    }

    #[test]
    fn xdg_requires_absolute() {
        // Sanity on the contract used by default_dir(); the happy path needs
        // a HOME env which we don't touch here.
        let prev = std::env::var("XDG_DATA_HOME").ok();
        std::env::set_var("XDG_DATA_HOME", "relative/path");
        assert!(Store::default_dir().is_err());
        match prev {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
    }
}
