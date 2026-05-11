use tracing_appender::non_blocking;
use tracing_subscriber::{Registry, fmt, prelude::*};

pub struct Logger;

impl Logger {
    pub fn new() -> non_blocking::WorkerGuard {
        // 1. กำหนดปลายทางไฟล์ (Log จะถูกเขียนลง ./logs/promotion-engine.log)
        let file_appender = tracing_appender::rolling::daily("./logs", "promotion-engine.log");

        // 2. ทำให้การเขียนไฟล์เป็น Non-blocking
        // _guard ต้องถูกเก็บไว้ใน main เพื่อให้การเขียน log ที่ค้างอยู่เสร็จสิ้นก่อนปิดโปรแกรม
        let (non_blocking_writer, _guard) = non_blocking(file_appender);

        // 3. สร้าง Subscriber ที่พ่นออกมาเป็น JSON
        let subscriber = Registry::default()
            .with(tracing_subscriber::EnvFilter::new("info")) // กำหนด Log Level
            .with(
                fmt::layer()
                    .json() // พ่นออกเป็น JSON
                    .with_writer(non_blocking_writer),
            );

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");

        _guard
    }
}
