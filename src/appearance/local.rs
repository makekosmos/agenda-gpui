//! Agenda-owned appearance choices, separate from Engine's shared config.
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use super::Mode;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LocalMaterial {
    Opaque,
    Acrylic,
    Mica,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LocalAppearance {
    schema_version: u32,
    mode: Mode,
    theme: String,
    material: LocalMaterial,
}

impl LocalAppearance {
    pub fn from_selection(mode: u8, theme: usize, material: u8) -> Self {
        Self {
            schema_version: 1,
            mode: match mode {
                0 => Mode::Light,
                2 => Mode::System,
                _ => Mode::Dark,
            },
            theme: crate::palettes::THEMES
                .get(theme)
                .unwrap_or(&crate::palettes::THEMES[0])
                .key
                .into(),
            material: match material {
                1 => LocalMaterial::Acrylic,
                2 => LocalMaterial::Mica,
                _ => LocalMaterial::Opaque,
            },
        }
    }

    pub fn selection(&self) -> (u8, usize, u8) {
        let mode = match self.mode {
            Mode::Light => 0,
            Mode::Dark => 1,
            Mode::System => 2,
        };
        let theme = crate::palettes::THEMES
            .iter()
            .position(|def| def.key == self.theme)
            .unwrap_or(0);
        let material = match self.material {
            LocalMaterial::Opaque => 0,
            LocalMaterial::Acrylic => 1,
            LocalMaterial::Mica => 2,
        };
        (mode, theme, material)
    }

    fn valid(&self) -> bool {
        self.schema_version == 1
            && crate::palettes::THEMES
                .iter()
                .any(|def| def.key == self.theme)
    }

    pub fn load(path: &Path) -> Option<Self> {
        if std::fs::metadata(path).ok()?.len() > 4096 {
            return None;
        }
        let parsed: Self = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        parsed.valid().then_some(parsed)
    }
}

pub fn path() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("APPDATA").map(PathBuf::from)?;
    #[cfg(target_os = "macos")]
    let base = PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support");
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("Agenda").join("appearance.json"))
}

enum Command {
    Save(LocalAppearance),
    Finish,
}

pub struct LocalWriter {
    sender: Sender<Command>,
    thread: Option<JoinHandle<()>>,
}

impl LocalWriter {
    pub fn start(path: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            while let Ok(Command::Save(mut latest)) = receiver.recv() {
                loop {
                    match receiver.recv_timeout(Duration::from_millis(250)) {
                        Ok(Command::Save(next)) => latest = next,
                        Ok(Command::Finish) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                            if let Err(error) = write_atomic(&path, &latest) {
                                eprintln!("[appearance] local save failed: {error}");
                            }
                            return;
                        }
                        Err(mpsc::RecvTimeoutError::Timeout) => break,
                    }
                }
                if let Err(error) = write_atomic(&path, &latest) {
                    eprintln!("[appearance] local save failed: {error}");
                }
            }
        });
        Self {
            sender,
            thread: Some(thread),
        }
    }

    pub fn save(&self, selection: (u8, usize, u8)) {
        let value = LocalAppearance::from_selection(selection.0, selection.1, selection.2);
        let _ = self.sender.send(Command::Save(value));
    }
}

impl Drop for LocalWriter {
    fn drop(&mut self) {
        let _ = self.sender.send(Command::Finish);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn write_atomic(path: &Path, value: &LocalAppearance) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("appearance path has no parent"))?;
    std::fs::create_dir_all(parent)?;
    let temp = parent.join(format!(".appearance-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        let bytes = serde_json::to_vec(value).map_err(std::io::Error::other)?;
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests;
