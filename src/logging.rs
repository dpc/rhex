use fern;
use log;
use chrono;

pub fn init() {
    let logger_config = fern::DispatchConfig {
        format: Box::new(|msg: &str, level: &log::LogLevel, _location: &log::LogLocation| {
            // This is a fairly simple format, though it's possible to do more complicated ones.
            // This closure can contain any code, as long as it produces a String message.
            format!("{} {:<6} {}", chrono::Local::now().to_rfc3339(), level, msg)
        }),
        output: vec![fern::OutputConfig::file("debug.log")],
        level: log::LogLevelFilter::Debug,
    };

    if let Err(e) = fern::init_global_logger(logger_config, log::LogLevelFilter::Trace) {
        panic!("Failed to initialize global logger: {}", e);
    }
}
