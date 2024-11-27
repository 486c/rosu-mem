pub mod error;
pub mod process;
pub mod signature;

cfg_if::cfg_if! {
    if #[cfg(target_os = "linux")] {
        mod linux;
    } else if #[cfg(target_os = "windows")] {
        pub mod windows;
    }
}
