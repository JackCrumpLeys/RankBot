use super::Error;
use log::LevelFilter;

pub fn setup_logging() -> Result<(), Error> {
    let formatted_time = chrono::Local::now().format("%d-%m-%Y-%H-%M");
    let info_logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%d-%m-%Y][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Info)
        .chain(fern::log_file("log/info.log")?)
        .chain(fern::log_file(format!("log/info_{}.log", formatted_time))?);
    let debug_logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%d-%m-%Y][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Debug)
        .chain(fern::log_file("log/debug.log")?)
        .chain(fern::log_file(format!("log/debug_{}.log", formatted_time))?);
    let trace_logger = fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%d-%m-%Y][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(LevelFilter::Trace)
        .chain(fern::log_file("log/trace.log")?)
        .chain(fern::log_file(format!("log/trace_{}.log", formatted_time))?)
        .chain(std::io::stdout());
    fern::Dispatch::new()
        // per-module overrides
        .level_for("serenity", LevelFilter::Warn)
        .level_for("hyper", LevelFilter::Warn)
        .level_for("poise", LevelFilter::Warn)
        .level_for("tracing", LevelFilter::Warn)
        .level_for("h2", LevelFilter::Warn)
        .level_for("reqwest", LevelFilter::Warn)
        .level_for("rustls", LevelFilter::Warn)
        .level_for("sqlx", LevelFilter::Warn)
        .level_for("tungstenite", LevelFilter::Warn)
        .level_for("tokio_tungstenite", LevelFilter::Warn)
        .level_for("sea_orm", LevelFilter::Info)
        .level_for("tokio_util", LevelFilter::Info)
        .level_for("want", LevelFilter::Info)
        .level_for("sea_orm_migration", LevelFilter::Info)
        .level_for("framed_impl", LevelFilter::Info)
        // Output to stdout, files, and other Dispatch configurations
        .chain(info_logger)
        .chain(debug_logger)
        .chain(trace_logger)
        .apply()
        .expect("Failed to initialize logger");

    Ok(())
}
