/// Format a byte rate as a short human string, e.g. `1.2 MB/s`.
pub fn rate(bytes_per_sec: f64, bits: bool) -> String {
    let (mut v, units): (f64, [&str; 4]) = if bits {
        (bytes_per_sec * 8.0, ["b/s", "Kb/s", "Mb/s", "Gb/s"])
    } else {
        (bytes_per_sec, ["B/s", "KB/s", "MB/s", "GB/s"])
    };
    let mut i = 0;
    while v >= 1000.0 && i < units.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 {
        format!("{:.0} {}", v, units[i])
    } else {
        format!("{:.1} {}", v, units[i])
    }
}

/// e.g. `CPU 12% MEM 45% DSK 60%`.
pub fn sys_line(cpu: f32, mem: f32, disk: f32) -> String {
    format!("CPU {cpu:.0}% MEM {mem:.0}% DSK {disk:.0}%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes() {
        assert_eq!(rate(0.0, false), "0 B/s");
        assert_eq!(rate(999.0, false), "999 B/s");
        assert_eq!(rate(1_500.0, false), "1.5 KB/s");
        assert_eq!(rate(2_500_000.0, false), "2.5 MB/s");
    }

    #[test]
    fn sys() {
        assert_eq!(sys_line(12.4, 45.0, 59.6), "CPU 12% MEM 45% DSK 60%");
    }

    #[test]
    fn bits() {
        assert_eq!(rate(125_000.0, true), "1.0 Mb/s");
    }
}
