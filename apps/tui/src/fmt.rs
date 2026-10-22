use std::time::Duration;

/// Human-scaled duration: `850ns`, `12.4µs`, `3.21ms`, `1.05s`.
pub fn duration(d: Duration) -> String {
    let ns = d.as_nanos() as f64;
    if ns < 1e3 {
        format!("{ns:.0}ns")
    } else if ns < 1e6 {
        format!("{:.1}µs", ns / 1e3)
    } else if ns < 1e9 {
        format!("{:.2}ms", ns / 1e6)
    } else {
        format!("{:.2}s", ns / 1e9)
    }
}

/// Wall-clock style uptime: `42s`, `3m07s`, `1h02m`.
pub fn uptime(d: Duration) -> String {
    let s = d.as_secs();
    match s {
        0..60 => format!("{s}s"),
        60..3600 => format!("{}m{:02}s", s / 60, s % 60),
        _ => format!("{}h{:02}m", s / 3600, (s % 3600) / 60),
    }
}

pub fn bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = b as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{b} B")
    } else {
        format!("{v:.1} {}", UNITS[unit])
    }
}

/// SI-scaled rate: `1.23 M/s`.
pub fn rate(per_sec: f64) -> String {
    let (v, suffix) = if per_sec >= 1e9 {
        (per_sec / 1e9, "G")
    } else if per_sec >= 1e6 {
        (per_sec / 1e6, "M")
    } else if per_sec >= 1e3 {
        (per_sec / 1e3, "k")
    } else {
        (per_sec, "")
    };
    format!("{v:.2} {suffix}/s")
}

pub fn count(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('_');
        }
        out.push(c);
    }
    out
}

pub fn sci(v: f64) -> String {
    if v == 0.0 {
        "0".into()
    } else {
        format!("{v:.2e}")
    }
}

/// Compact numeric cell (at most ~8 chars) with about four significant digits.
pub fn value(v: f64, integer: bool) -> String {
    let a = v.abs();
    if !v.is_finite() {
        format!("{v}")
    } else if integer && a < 1e9 {
        format!("{v:.0}")
    } else if a == 0.0 {
        "0".into()
    } else if (1e-3..1e5).contains(&a) {
        let prec = (3 - a.log10().floor() as i32).clamp(0, 5) as usize;
        format!("{v:.prec$}")
    } else {
        format!("{v:.2e}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(duration(Duration::from_nanos(850)), "850ns");
        assert_eq!(value(4096.0, true), "4096");
        assert_eq!(value(1.23456, false), "1.235");
        assert_eq!(value(-0.0012344, false), "-0.00123");
        assert_eq!(value(123456.0, false), "1.23e5");
        assert_eq!(duration(Duration::from_micros(12_400)), "12.40ms");
        assert_eq!(uptime(Duration::from_secs(187)), "3m07s");
        assert_eq!(bytes(1536), "1.5 KiB");
        assert_eq!(rate(1_230_000.0), "1.23 M/s");
        assert_eq!(count(1_234_567), "1_234_567");
        assert_eq!(count(999), "999");
    }
}
