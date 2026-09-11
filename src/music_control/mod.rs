#[cfg(windows)]
mod controller;
#[cfg(windows)]
mod service;
#[cfg(windows)]
mod widget;

#[cfg(windows)]
pub use service::MusicControlService;

#[cfg(not(windows))]
mod linux_service;

#[cfg(not(windows))]
pub use linux_service::MusicControlService;
