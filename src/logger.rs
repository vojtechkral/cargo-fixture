use clap::ValueEnum;
use log::{LevelFilter, Log, Metadata, Record};

#[derive(ValueEnum, Clone, Copy, Debug)]
#[clap(rename_all = "lower")]
pub enum LogLevel {
    Off,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Off => LevelFilter::Off,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Trace => LevelFilter::Trace,
        }
    }
}

#[derive(Debug)]
pub struct Logger;

static LOGGER: &Logger = &Logger;

impl Log for Logger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if record.level() == LevelFilter::Info {
            eprintln!("cargo-fixture: {}", record.args());
        } else {
            eprintln!("cargo-fixture {}: {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init(level: LogLevel) {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(level.into());
}
