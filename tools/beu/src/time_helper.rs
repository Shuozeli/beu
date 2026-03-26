use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Get current UTC timestamp in ISO 8601 format with millisecond precision.
pub fn utc_now() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock is before UNIX epoch");
    let secs = dur.as_secs();
    let millis = dur.subsec_millis();
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;
    let (year, month, day) = days_to_ymd(days);
    format!("{year:04}-{month:02}-{day:02}T{hours:02}:{minutes:02}:{seconds:02}.{millis:03}Z")
}

/// Generate a unique ID using FNV hash of timestamp + atomic counter + PID.
/// Returns a prefixed hex string like "j-a1b2c3d4e5f6" (12 hex digits / 48 bits).
pub fn generate_id(prefix: &str) -> String {
    let ts = utc_now();
    let counter = ID_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    let pid = std::process::id();
    let mut h: u64 = 0xcbf29ce484222325;
    for byte in ts
        .as_bytes()
        .iter()
        .chain(&counter.to_le_bytes())
        .chain(&pid.to_le_bytes())
    {
        h ^= *byte as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{prefix}-{h:012x}", h = h & 0xFFFFFFFFFFFF)
}

fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_now_has_iso_format() {
        let ts = utc_now();
        assert!(ts.ends_with('Z'));
        assert!(ts.contains('T'));
        assert_eq!(ts.len(), 24); // YYYY-MM-DDThh:mm:ss.mmmZ
    }

    #[test]
    fn generate_id_has_prefix() {
        let id = generate_id("j");
        assert!(id.starts_with("j-"));
        assert_eq!(id.len(), 2 + 12); // prefix + dash + 12 hex chars
    }

    #[test]
    fn generate_id_is_unique() {
        let id1 = generate_id("x");
        let id2 = generate_id("x");
        assert_ne!(id1, id2);
    }
}
