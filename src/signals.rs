use core::cell::RefCell;

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;

pub static SEED_PHRASE: Mutex<CriticalSectionRawMutex, RefCell<Option<[&'static str; 24]>>> =
    Mutex::new(RefCell::new(None));
pub enum Message {
    Next,
    Back,
}

pub static MESSAGE_QUEUE: Channel<CriticalSectionRawMutex, Message, 4> = Channel::new();
