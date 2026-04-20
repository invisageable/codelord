//! Time utilities shared across the core crate.

/// Current wall-clock time in milliseconds since UNIX_EPOCH.
///
/// Returns 0 if the system clock is before 1970 (never happens in practice).
#[inline]
pub fn current_time_ms() -> u64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0)
}
