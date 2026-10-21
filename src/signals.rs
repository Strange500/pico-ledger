use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;

pub static BUTTON_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();
pub static WORD_SIGNAL: Signal<CriticalSectionRawMutex, &'static str> = Signal::new();
