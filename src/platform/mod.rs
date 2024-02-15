use crate::Thread;

// #[cfg(unix)]
mod linux;
#[cfg(windows)]
mod windows;

pub fn get_threads() -> anyhow::Result<Vec<Thread>> {
    #[cfg(windows)]
    return windows::get_threads();
    #[cfg(unix)]
    return linux::get_threads();
}
