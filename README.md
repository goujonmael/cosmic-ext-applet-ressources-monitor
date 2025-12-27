# Resources Monitor - COSMIC™ Applet

![screenshot of the applet](res/screenshots/top_panel.png)

Resources Monitor is a panel applet for the COSMIC™ desktop environment that displays key system metrics in real time:

- CPU usage (%),
- Average CPU frequency (MHz),
- CPU temperature (°C),
- RAM usage (%).

**Features**

- Periodic refresh (1s)
- Implemented in Rust and integrated with `libcosmic` / Iced
- Uses the `sysinfo` crate to gather system metrics

**Prerequisites**

Install the required build tools:

```bash
sudo apt install just cargo build-essential
```

For CPU temperature readings, ensure your sensors are available (for example `lm-sensors` and the appropriate kernel drivers). Test with:

```bash
sudo apt install lm-sensors
sudo sensors-detect
sensors
```

**Installation**

1. Clone this fork:

```bash
git clone https://github.com/goujonmael/cosmic-ext-applet-ressources-monitor.git
cd cosmic-ext-applet-ressources-monitor
```

2. Build the release and install:

```bash
just build-release
sudo just install
```

For development (debug build + install):

```bash
just build-debug && sudo just debug=1 install && pkill cosmic-panel
```

**Usage**

After installation the applet should appear in the COSMIC applet list. You can also run the binary directly:

```bash
/usr/bin/cosmic-ext-applet-ressources-monitor
```

**Development**

- Main UI logic lives in `src/window.rs`.
- Metrics are collected via the `sysinfo` crate (see `Cargo.toml`).

If you want to modify or extend the applet, start by exploring `src/window.rs` and the other files in `src/`.

**Contributing**

This repository is a fork of the original netspeed applet by `khanra17`. To contribute:

1. Fork the repository
2. Create a feature branch
3. Make your changes and open a pull request

**License**

This project is licensed under the GPL-3.0 License.
