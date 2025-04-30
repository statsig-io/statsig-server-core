#[cfg(target_env = "gnu")]
use crate::log_d;

// Manually free memory
#[cfg(target_env = "gnu")]
extern "C" {
    fn malloc_trim(pad: libc::size_t) -> libc::c_int;
}

#[cfg(target_env = "gnu")]
pub fn maybe_trim_malloc() {
    unsafe {
        // Free as much memory as possible
        let result = malloc_trim(0);
        if result == 0 {
            log_d!("MemoryUtils", "No memory was released by malloc_trim.");
        } else {
            log_d!("MemoryUtils", "Memory was released by malloc_trim.");
        }
    }
}
#[cfg(not(target_env = "gnu"))]
pub fn maybe_trim_malloc() {
    // No-op only glibc supports malloc_trim function
}
