use log::{LevelFilter, Log, Metadata, Record};

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

pub fn init(verbosity: u32) {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match verbosity {
        0 => LevelFilter::Off,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    });
}
