use std::path::{PathBuf};
use std::fs;
use std::io::{self, Write};

pub const CONFIG_VERSION: u64 = 1;

fn config_dir() -> Option<PathBuf> {
	if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
		return Some(PathBuf::from(xdg).join("cosmic-ext-applet-ressources-monitor"));
	}
	if let Ok(home) = std::env::var("HOME") {
		return Some(PathBuf::from(home).join(".config").join("cosmic-ext-applet-ressources-monitor"));
	}
	None
}

fn selected_sensor_path() -> Option<PathBuf> {
	config_dir().map(|d| d.join("selected_sensor.txt"))
}

pub fn load_selected_sensor() -> Option<String> {
	let path = selected_sensor_path()?;
	if path.exists() {
		if let Ok(s) = fs::read_to_string(path) {
			let t = s.trim().to_string();
			if t.is_empty() { None } else { Some(t) }
		} else {
			None
		}
	} else {
		None
	}
}

pub fn save_selected_sensor(label: &str) -> io::Result<()> {
	if let Some(dir) = config_dir() {
		fs::create_dir_all(&dir)?;
		if let Some(path) = selected_sensor_path() {
			let mut f = fs::File::create(path)?;
			f.write_all(label.as_bytes())?;
		}
	}
	Ok(())
}
