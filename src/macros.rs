/// Log simultaneously to both defmt (SWD RTT) and USB CDC-ACM serial.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        ::defmt::info!($($arg)*);
        ::log::info!($($arg)*);
    }};
}
