use cosmic_ext_applet_ressources_monitor::config::CONFIG_VERSION;

fn main() -> cosmic::iced::Result {
    tracing_subscriber::fmt::init();
    let _ = tracing_log::LogTracer::init();

    tracing::info!("Starting Ressources Monitor applet with version {CONFIG_VERSION}");

    cosmic_ext_applet_ressources_monitor::run()
}
