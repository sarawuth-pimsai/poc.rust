use logging::Logger;
use tracing::{error, info};

fn main() {
    println!("Starting Promotion Engine...");
    let _guard = Logger::new(); // เริ่มต้นระบบ Logging

    info!(
        event = "system_start",
        mode = "production",
        "Promotion Engine is running"
    );

    // ตัวอย่างการจำลองการทำงาน
    error!(
        status = "fail",
        error_code = "PROMO_APPLY_ERROR",
        promo_id = "PROMO123",
        user_id = "user_99",
        latency_ms = 12,
        "Failed to apply promotion"
    );
    println!("Promotion Engine is shutting down...");
}
