use cosmic::{
    app,
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{row, text, column},
        Alignment, Subscription,
    },
    widget::{autosize, button},
    Element,
};

use sysinfo::{System, SystemExt, CpuExt, ComponentExt};
use std::fs;
use std::path::Path;
use crate::config;
use std::process::Command;
use std::thread;

pub struct Window {
    core: cosmic::app::Core,
    sys: System,
    cpu_usage: f32,    // CPU usage in percent
    avg_freq: u64,     // Average CPU frequency in MHz
    cpu_temp: f32,     // CPU temperature in °C
    ram_percent: f32,  // RAM usage in percent
    show_sensor_menu: bool,
    selected_sensor: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    ToggleMenu,
    SelectSensor(String),
}

impl Window {
    fn classify_label(label: &str) -> &'static str {
        Self::classify_label_static(label)
    }

    fn classify_label_static(label: &str) -> &'static str {
        let l = label.to_lowercase();

        // More extensive classification by keyword matching (ordered by priority)
        let cpu_keys = ["cpu", "package", "pkg", "k10temp", "coretemp", "tctl", "tdie", "core", "cros_ec cpu"];
        if cpu_keys.iter().any(|k| l.contains(k)) {
            return "CPU";
        }

        if l.contains("gpu") || l.contains("amdgpu") || l.contains("nvidia") || l.contains("radeon") {
            return "GPU";
        }

        if l.contains("nvme") || l.contains("ssd") || l.contains("disk") || l.contains("hdd") {
            return "SSD";
        }

        if l.contains("battery") || l.contains("charge") {
            return "Battery";
        }

        if l.contains("acpitz") || l.contains("ambient") || l.contains("temp") && (l.contains("ambient") || l.contains("zone")) {
            return "Ambient";
        }

        if l.contains("pch") || l.contains("motherboard") || l.contains("board") || l.contains("ec") {
            // 'cros_ec' often appears for Chromebook EC sensors
            if l.contains("cros_ec") { return "EC"; }
            return "Motherboard";
        }

        if l.contains("spd") || l.contains("dimm") || l.contains("memory") || l.contains("dram") {
            return "Memory";
        }

        if l.contains("iwl") || l.contains("wifi") || l.contains("wlan") || l.contains("phy") || l.contains("mt7") || l.contains("mt79") {
            return "Wireless";
        }

        if l.contains("fan") || l.contains("tach") {
            return "Fan";
        }

        if l.contains("psu") || l.contains("ac") || l.contains("power") {
            return "Power";
        }

        if l.contains("raid") || l.contains("md") || l.contains("controller") {
            return "Controller";
        }

        if l.contains("spd5118") {
            return "Memory";
        }

        // fallback
        "Other"
    }
    fn format_percent(value: f32) -> String {
        format!("{:.1}%", value)
    }

    fn format_freq(mhz: u64) -> String {
        format!("{} MHz", mhz)
    }

    fn format_temp(temp: f32) -> String {
        format!("{:.1} °C", temp)
    }

    fn update_metrics(&mut self) {
        self.sys.refresh_cpu();
        self.sys.refresh_memory();
        self.sys.refresh_components();

        let cpus = self.sys.cpus();
        if !cpus.is_empty() {
            let total_usage: f32 = cpus.iter().map(|c| c.cpu_usage()).sum::<f32>();
            self.cpu_usage = total_usage / (cpus.len() as f32);

            let total_freq: u64 = cpus.iter().map(|c| c.frequency() as u64).sum::<u64>();
            self.avg_freq = total_freq / (cpus.len() as u64);
        } else {
            self.cpu_usage = 0.0;
            self.avg_freq = 0;
        }

        // per-CPU frequency (kept for potential future use)
        let _list: Vec<u64> = cpus.iter().map(|c| c.frequency() as u64).collect();

        // Try to read current frequency from sysfs (scaling_cur_freq) as a more
        // reliable, up-to-date fallback on Linux systems. Values are in kHz
        // there so convert to MHz. If sysfs is not available, try /proc/cpuinfo.
        if let Some(mhz) = Self::read_freq_sysfs() {
            self.avg_freq = mhz;
        } else if let Some(mhz_proc) = Self::read_freq_proc_cpuinfo() {
            self.avg_freq = mhz_proc;
        }

        let components = self.sys.components();

        // Reload selection from disk in case external picker updated it
        if let Some(s) = config::load_selected_sensor() {
            self.selected_sensor = Some(s);
        }

        // Prefer a user-selected hwmon sensor when available, otherwise fallback to k10temp-pci-00c3
        let preferred_label = self.selected_sensor.clone().unwrap_or_else(|| "k10temp-pci-00c3".to_string());
        if let Some(pref) = components.iter().find(|c| c.label().to_lowercase() == preferred_label.to_lowercase()) {
            self.cpu_temp = pref.temperature();
        } else {
            let temps: Vec<f32> = components
                .iter()
                .filter(|c| {
                    let l = c.label().to_lowercase();
                    l.contains("cpu") || l.contains("package")
                })
                .map(|c| c.temperature())
                .collect();

            if !temps.is_empty() {
                let sum: f32 = temps.iter().copied().sum();
                self.cpu_temp = sum / (temps.len() as f32);
            } else if !components.is_empty() {
                self.cpu_temp = components.iter().map(|c| c.temperature()).fold(0.0_f32, |a, b| a.max(b));
            } else {
                self.cpu_temp = 0.0;
            }
        }

        let total_ram = self.sys.total_memory() as f32;
        let used_ram = self.sys.used_memory() as f32;
        if total_ram > 0.0 {
            self.ram_percent = (used_ram / total_ram) * 100.0;
        } else {
            self.ram_percent = 0.0;
        }
    }

    fn read_freq_sysfs() -> Option<u64> {
        let cpu_dir = Path::new("/sys/devices/system/cpu");
        let mut freqs_khz: Vec<u64> = Vec::new();

        let entries = fs::read_dir(cpu_dir).ok()?;
        for entry in entries.flatten() {
            let name = entry.file_name().into_string().ok()?;
            if !name.starts_with("cpu") {
                continue;
            }
            // ensure the suffix is a CPU number (cpu0, cpu1, ...)
            let suffix = &name[3..];
            if suffix.is_empty() || !suffix.chars().all(|c| c.is_ascii_digit()) {
                continue;
            }

            let path = cpu_dir.join(name).join("cpufreq").join("scaling_cur_freq");
            if let Ok(s) = fs::read_to_string(&path) {
                if let Ok(khz) = s.trim().parse::<u64>() {
                    freqs_khz.push(khz);
                }
            }
        }

        if freqs_khz.is_empty() {
            None
        } else {
            let sum: u64 = freqs_khz.iter().sum();
            let avg_khz = sum / (freqs_khz.len() as u64);
            Some(avg_khz / 1000) // convert kHz -> MHz
        }
    }

    fn read_freq_proc_cpuinfo() -> Option<u64> {
        let s = fs::read_to_string("/proc/cpuinfo").ok()?;
        let mut mhz_vals: Vec<f32> = Vec::new();
        for line in s.lines() {
            if line.starts_with("cpu MHz") {
                if let Some(pos) = line.find(':') {
                    let v = line[pos + 1..].trim();
                    if let Ok(f) = v.parse::<f32>() {
                        mhz_vals.push(f);
                    }
                }
            }
        }
        if mhz_vals.is_empty() {
            None
        } else {
            let sum: f32 = mhz_vals.iter().copied().sum();
            let avg = sum / (mhz_vals.len() as f32);
            Some(avg.round() as u64)
        }
    }
}

impl cosmic::Application for Window {
    type Message = Message;
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    const APP_ID: &'static str = "io.github.khanra17.ressources-monitor";

    fn init(
        core: app::Core,
        _flags: Self::Flags,
    ) -> (Self, cosmic::iced::Task<app::Message<Self::Message>>) {
        let mut sys = System::new_all();
        sys.refresh_cpu();
        sys.refresh_memory();
        sys.refresh_components();

        let mut window = Self {
            core,
            sys,
            cpu_usage: 0.0,
            avg_freq: 0,
            cpu_temp: 0.0,
            ram_percent: 0.0,
            show_sensor_menu: false,
            selected_sensor: config::load_selected_sensor(),
        };

        window.update_metrics();

        (window, cosmic::iced::Task::none())
    }

    fn core(&self) -> &cosmic::app::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::app::Core {
        &mut self.core
    }

    fn style(&self) -> Option<cosmic::iced_runtime::Appearance> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Message> {
        cosmic::iced::time::every(std::time::Duration::from_secs(1)).map(|_| Message::Tick)
    }

    fn update(&mut self, _message: Message) -> cosmic::iced::Task<app::Message<Self::Message>> {
        match _message {
            Message::Tick => {
                self.update_metrics();
            }
            Message::ToggleMenu => {
                // Collect labels + temps so we can show temps and types next to labels in the picker
                let components: Vec<(String, f32)> = self
                    .sys
                    .components()
                    .iter()
                    .map(|c| (c.label().to_string(), c.temperature()))
                    .collect();
                thread::spawn(move || {
                    // Build entries as tuples (kind, label, temp), sort by kind then label,
                    // then render as "TYPE — label — 42.3 °C"
                    let mut entries: Vec<(String, String, f32)> = components
                        .into_iter()
                        .map(|(label, temp)| {
                            let kind = Window::classify_label_static(&label).to_string();
                            (kind, label, temp)
                        })
                        .collect();

                    entries.sort_by(|a, b| {
                        let cmp_kind = a.0.cmp(&b.0);
                        if cmp_kind == std::cmp::Ordering::Equal {
                            a.1.cmp(&b.1)
                        } else {
                            cmp_kind
                        }
                    });

                    let display_entries: Vec<String> = entries
                        .iter()
                        .map(|(kind, label, temp)| format!("{} — {} — {:.1} °C", kind, label, temp))
                        .collect();

                    // Try rofi -dmenu
                    let input = display_entries.join("\n");
                    if let Ok(mut child) = Command::new("rofi")
                        .arg("-dmenu")
                        .arg("-p")
                        .arg("Select sensor")
                        .stdin(std::process::Stdio::piped())
                        .stdout(std::process::Stdio::piped())
                        .spawn()
                    {
                        if let Some(mut stdin) = child.stdin.take() {
                            use std::io::Write;
                            let _ = stdin.write_all(input.as_bytes());
                        }
                        if let Ok(output) = child.wait_with_output() {
                            if output.status.success() {
                                if let Ok(s) = String::from_utf8(output.stdout) {
                                    let sel = s.trim().to_string();
                                    if !sel.is_empty() {
                                        // extract label part between the separators
                                        // format: "TYPE — label — 42.3 °C"
                                        let parts: Vec<&str> = sel.split(" — ").collect();
                                        let label = if parts.len() >= 2 { parts[1].trim().to_string() } else { sel.clone() };
                                        let _ = config::save_selected_sensor(&label);
                                    }
                                }
                            }
                        }
                    } else if let Ok(output) = Command::new("zenity")
                        .arg("--list")
                        .arg("--column=Sensor")
                        .args(display_entries.iter())
                        .output()
                    {
                        if output.status.success() {
                            if let Ok(s) = String::from_utf8(output.stdout) {
                                let sel = s.trim().to_string();
                                if !sel.is_empty() {
                                    let parts: Vec<&str> = sel.split(" — ").collect();
                                    let label = if parts.len() >= 2 { parts[1].trim().to_string() } else { sel.clone() };
                                    let _ = config::save_selected_sensor(&label);
                                }
                            }
                        }
                    }

                });

                // keep the in-applet menu state for backward compatibility
                self.show_sensor_menu = !self.show_sensor_menu;
                self.sys.refresh_components();
            }
            Message::SelectSensor(label) => {
                self.selected_sensor = Some(label.clone());
                let _ = config::save_selected_sensor(&label);
                self.show_sensor_menu = false;
                self.sys.refresh_components();
            }
        }
        cosmic::iced::Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let horizontal = matches!(
            self.core.applet.anchor,
            PanelAnchor::Top | PanelAnchor::Bottom
        );

        let content = if horizontal {
            row![
                text(format!("CPU {}", Self::format_percent(self.cpu_usage))),
                text(format!("{}", Self::format_freq(self.avg_freq))),
                text(format!("{}", Self::format_temp(self.cpu_temp))),
                text(format!("RAM {}", Self::format_percent(self.ram_percent))),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
        } else {
            row![
                text(format!("CPU {}", Self::format_percent(self.cpu_usage))),
                text(format!("{}", Self::format_freq(self.avg_freq))),
                text(format!("{}", Self::format_temp(self.cpu_temp))),
                text(format!("RAM {}", Self::format_percent(self.ram_percent))),
            ]
            .spacing(4)
            .align_y(Alignment::Center)
        };

        let main_button = button::custom(content)
            .on_press(Message::ToggleMenu)
            .padding([
                self.core.applet.suggested_padding(horizontal),
                self.core.applet.suggested_padding(!horizontal),
            ])
            .class(cosmic::theme::Button::AppletIcon);

        // If menu is open, build a column with all temperature sensors
        if self.show_sensor_menu {
            // build vector of (kind,label,temp) and sort
            let mut items: Vec<(String, String, f32)> = self
                .sys
                .components()
                .iter()
                .map(|c| {
                    let label = c.label().to_string();
                    let kind = Self::classify_label(&label).to_string();
                    (kind, label, c.temperature())
                })
                .collect();

            items.sort_by(|a, b| {
                let cmp_kind = a.0.cmp(&b.0);
                if cmp_kind == std::cmp::Ordering::Equal {
                    a.1.cmp(&b.1)
                } else {
                    cmp_kind
                }
            });

            let mut menu = column![];
            for (kind, label, temp) in items {
                let label_clone = label.clone();
                let display = text(format!("{} — {} — {:.1} °C", kind, label, temp));
                let btn = button::custom(display).on_press(Message::SelectSensor(label_clone));
                menu = menu.push(btn.padding(4));
            }

            let layout = column![main_button, menu].spacing(4);
            autosize::autosize(layout, cosmic::widget::Id::unique()).into()
        } else {
            autosize::autosize(main_button, cosmic::widget::Id::unique()).into()
        }
    }

    fn on_close_requested(&self, _id: cosmic::iced::window::Id) -> Option<Message> {
        None
    }
}
