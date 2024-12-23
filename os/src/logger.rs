use crate::{task::current_task, utils::hart_id};
use log::{Level, LevelFilter, Metadata, Record};

#[cfg(feature = "error")]
const LOG_LEVEL: log::LevelFilter = LevelFilter::Error;
#[cfg(feature = "warn")]
const LOG_LEVEL: log::LevelFilter = LevelFilter::Warn;
#[cfg(feature = "info")]
const LOG_LEVEL: log::LevelFilter = LevelFilter::Info;
#[cfg(feature = "debug")]
const LOG_LEVEL: log::LevelFilter = LevelFilter::Debug;
#[cfg(feature = "trace")]
const LOG_LEVEL: log::LevelFilter = LevelFilter::Trace;

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= NOW_LEVEL
    }
    fn log(&self, record: &Record) {
        let (pid, tid) = if let Some(task) = current_task() {
            (task.pid() as isize, task.tid() as isize)
        } else {
            (-1, -1)
        };
        if self.enabled(record.metadata()) {
            println!(
                "\x1b[{}m[{}] [HART{}] [PID {}] [TID {}] {}\x1b[0m",
                level_color(record.level()),
                record.level(),
                hart_id(),
                pid,
                tid,
                record.args()
            );
        }
    }
    fn flush(&self) {}
}

static LOGGER: SimpleLogger = SimpleLogger;
static NOW_LEVEL: log::LevelFilter = LOG_LEVEL;

pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LOG_LEVEL))
        .unwrap();
}

fn level_color(level: Level) -> u8 {
    match level {
        Level::Error => 31,
        Level::Warn => 93,
        Level::Info => 34,
        Level::Debug => 32,
        Level::Trace => 90,
    }
}
