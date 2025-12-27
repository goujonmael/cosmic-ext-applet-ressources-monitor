use cosmic::{
    app,
    applet::cosmic_panel_config::PanelAnchor,
    iced::{
        widget::{row, text},
        Alignment, Subscription,
    },
    widget::{autosize, button},
    Element,
};

use sysinfo::{System, SystemExt, CpuExt, ComponentExt};

pub struct Window {
    core: cosmic::app::Core,
    sys: System,
    cpu_usage: f32,    // CPU usage in percent
    avg_freq: u64,     // Average CPU frequency in MHz
    cpu_temp: f32,     // CPU temperature in °C
    ram_percent: f32,  // RAM usage in percent
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
}

impl Window {
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

        let components = self.sys.components();
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

        let total_ram = self.sys.total_memory() as f32;
        let used_ram = self.sys.used_memory() as f32;
        if total_ram > 0.0 {
            self.ram_percent = (used_ram / total_ram) * 100.0;
        } else {
            self.ram_percent = 0.0;
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
        // Only Tick exists, on every tick refresh metrics
        self.update_metrics();
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

        let button = button::custom(content)
            .padding([
                self.core.applet.suggested_padding(horizontal),
                self.core.applet.suggested_padding(!horizontal),
            ])
            .class(cosmic::theme::Button::AppletIcon);

        autosize::autosize(button, cosmic::widget::Id::unique()).into()
    }

    fn on_close_requested(&self, _id: cosmic::iced::window::Id) -> Option<Message> {
        None
    }
}
