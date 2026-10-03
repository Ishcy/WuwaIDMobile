//! Debug-only fixed-label timings. Never records paths, UID text, or payloads.
#[cfg(debug_assertions)]
use std::cell::Cell;
#[cfg(debug_assertions)]
thread_local! { static REQUEST: Cell<u64> = const { Cell::new(0) }; }

pub fn request_id() -> u64 {
    #[cfg(debug_assertions)]
    { REQUEST.with(Cell::get) }
    #[cfg(not(debug_assertions))]
    { 0 }
}

pub struct Request {
    #[cfg(debug_assertions)]
    previous: u64,
}
impl Request {
    pub fn enter(id: u64) -> Self {
        #[cfg(debug_assertions)]
        { Self { previous: REQUEST.with(|value| value.replace(id)) } }
        #[cfg(not(debug_assertions))]
        { let _ = id; Self {} }
    }
}
impl Drop for Request {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        REQUEST.with(|value| value.set(self.previous));
    }
}

pub struct Span {
    #[cfg(debug_assertions)]
    name: &'static str,
    #[cfg(debug_assertions)]
    start: u64,
}
impl Span {
    pub fn new(name: &'static str) -> Self {
        #[cfg(debug_assertions)]
        { let start = monotonic_ns(); mark(name, "begin", 0); Self { name, start } }
        #[cfg(not(debug_assertions))]
        { let _ = name; Self {} }
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        #[cfg(debug_assertions)]
        mark(self.name, "end", monotonic_ns().saturating_sub(self.start));
    }
}

pub fn mark(phase: &'static str, event: &'static str, value: u64) {
    #[cfg(debug_assertions)]
    {
        let line = format!("id={} phase={phase} event={event} monoNs={} value={value}", request_id(), monotonic_ns());
        #[cfg(target_os = "android")]
        {
            #[link(name = "log")]
            extern "C" { fn __android_log_write(priority: i32, tag: *const std::ffi::c_char, text: *const std::ffi::c_char) -> i32; }
            if let Ok(line) = std::ffi::CString::new(line) {
                unsafe { __android_log_write(3, c"WuwaIDTrace".as_ptr(), line.as_ptr()); }
            }
        }
        #[cfg(not(target_os = "android"))]
        eprintln!("WuwaIDTrace {line}");
    }
    #[cfg(not(debug_assertions))]
    { let _ = (phase, event, value); }
}

#[cfg(all(debug_assertions, target_os = "android"))]
fn monotonic_ns() -> u64 {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut time); }
    time.tv_sec as u64 * 1_000_000_000 + time.tv_nsec as u64
}
#[cfg(all(debug_assertions, not(target_os = "android")))]
fn monotonic_ns() -> u64 {
    static START: std::sync::LazyLock<std::time::Instant> = std::sync::LazyLock::new(std::time::Instant::now);
    START.elapsed().as_nanos() as u64
}
