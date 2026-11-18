use std::path::Path;
use std::process::Command;

pub fn safe_filename(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    s = s.trim().to_string();
    if s.is_empty() {
        s = "Note".into();
    }
    s.chars().take(120).collect()
}

pub fn open_path(path: &Path) {
    open_target(&path.to_string_lossy());
}

fn open_target(target: &str) {
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(target).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(target).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", target])
            .spawn();
    }
}

pub fn play_sound(path: &str) {
    if path.is_empty() || !std::path::Path::new(path).exists() {
        return;
    }
    #[cfg(target_os = "linux")]
    {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let candidates: &[&str] = if ext == "mp3" {
            &["ffplay", "mpv", "paplay"]
        } else {
            &["paplay", "aplay", "ffplay"]
        };
        for c in candidates {
            let mut cmd = Command::new(c);
            if *c == "ffplay" {
                cmd.args(["-nodisp", "-autoexit", "-loglevel", "quiet"]);
            }
            if cmd.arg(path).spawn().is_ok() {
                break;
            }
        }
    }
}

pub fn print_text(title: &str, text: &str) -> bool {
    let dir = std::env::temp_dir().join("sticky-desk-print");
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{}.txt", safe_filename(title)));
    if std::fs::write(&path, text).is_err() {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        return Command::new("lp").arg(&path).spawn().is_ok();
    }
    #[cfg(target_os = "macos")]
    {
        return Command::new("lp").arg(&path).spawn().is_ok();
    }
    #[cfg(target_os = "windows")]
    {
        return Command::new("notepad").arg("/p").arg(&path).spawn().is_ok();
    }
    #[allow(unreachable_code)]
    false
}

pub fn set_autostart(enabled: bool) {
    #[cfg(target_os = "linux")]
    {
        let dir = dirs::config_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("autostart");
        let file = dir.join("sticky-desk.desktop");
        if enabled {
            let _ = std::fs::create_dir_all(&dir);
            let exe =
                std::env::current_exe().unwrap_or_else(|_| std::path::PathBuf::from("ssn-linux"));
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=StickyDesk\nComment=Sticky notes for your desktop\nExec=\"{}\"\nIcon=accessories-text-editor\nTerminal=false\nCategories=Utility;\nX-GNOME-Autostart-enabled=true\n",
                exe.display()
            );
            let _ = std::fs::write(file, entry);
        } else {
            let _ = std::fs::remove_file(file);
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = enabled;
    }
}

pub fn sounds_dir() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("sticky-desk")
        .join("Sounds")
}

pub fn list_sounds() -> Vec<(String, String)> {
    let dir = sounds_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(ext.as_str(), "mp3" | "wav" | "ogg" | "flac") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    out.push((name.to_string(), path.to_string_lossy().to_string()));
                }
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    out
}

pub fn install_sound(source: &std::path::Path) -> anyhow::Result<String> {
    let dir = sounds_dir();
    std::fs::create_dir_all(&dir)?;
    let name = source
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("invalid sound path"))?;
    let target = dir.join(name);
    std::fs::copy(source, &target)?;
    Ok(target.to_string_lossy().to_string())
}
