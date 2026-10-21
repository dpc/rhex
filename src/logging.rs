use chrono;
use fern;
use log;

pub fn init() {
    fern::Dispatch::new()
        .format(|out, msg, record| {
            out.finish(format_args!(
                "{} {:<6} {}",
                chrono::Local::now().to_rfc3339(),
                record.level(),
                msg
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(fern::log_file("debug.log").expect("Failed to open debug.log"))
        .apply()
        .expect("Failed to initialize global logger");
}
