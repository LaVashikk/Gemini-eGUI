use log::{Level, Metadata, Record, SetLoggerError};
use std::sync::{mpsc, Mutex, OnceLock};

pub struct LogEvent {
    pub level: Level,
    pub message: String,
}

static LOG_RECEIVER: OnceLock<Mutex<mpsc::Receiver<LogEvent>>> = OnceLock::new();

struct GlobalLogger {
    inner: env_logger::Logger,
    sender: Mutex<mpsc::Sender<LogEvent>>,
}

impl log::Log for GlobalLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        // Always forward to env_logger for console/terminal output
        if self.inner.enabled(record.metadata()) {
            self.inner.log(record);
        }

        // Capture Warn and Error logs for the UI
        if record.level() <= Level::Warn {
            // We format the message immediately.
            let msg = format!("{}", record.args());
            
            // Send to the channel
            if let Ok(sender) = self.sender.lock() {
                 let _ = sender.send(LogEvent {
                     level: record.level(),
                     message: msg,
                 });
            }
        }
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

pub fn init() -> Result<(), SetLoggerError> {
    let (tx, rx) = mpsc::channel();
    
    // Store the receiver globally so the UI can access it later
    if LOG_RECEIVER.set(Mutex::new(rx)).is_err() {
        eprintln!("Logger already initialized");
        return Ok(());
    }

    let logger = GlobalLogger {
        inner: env_logger::Builder::from_default_env().build(),
        sender: Mutex::new(tx),
    };

    log::set_max_level(logger.inner.filter());
    log::set_boxed_logger(Box::new(logger))
}

pub fn pop_logs() -> Vec<LogEvent> {
    let mut logs = Vec::new();
    if let Some(rx_mutex) = LOG_RECEIVER.get() {
        if let Ok(rx) = rx_mutex.lock() {
            while let Ok(log) = rx.try_recv() {
                logs.push(log);
            }
        }
    }
    logs
}
