/// Log simultaneously to both defmt (SWD RTT) and USB CDC-ACM serial.
#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        ::defmt::info!($($arg)*);
        ::log::info!($($arg)*);
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {{
        ::defmt::warn!($($arg)*);
        ::log::warn!($($arg)*);
    }};
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        ::defmt::error!($($arg)*);
        ::log::error!($($arg)*);
    }};
}
