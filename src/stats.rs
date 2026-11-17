use std::collections::VecDeque;
use std::time::Instant;

use sysinfo::{Disks, Networks, System};

pub const HISTORY: usize = 60;

#[derive(Clone, Debug, Default)]
pub struct IfaceRate {
    pub name: String,
    pub down: f64,
    pub up: f64,
}

#[derive(Clone, Debug, Default)]
pub struct Sample {
    pub down: f64,
    pub up: f64,
    pub ifaces: Vec<IfaceRate>,
    pub cpu: f32,
    pub mem: f32,
    pub disk: f32,
}

/// Skip loopback and virtual interfaces.
pub fn is_virtual(name: &str) -> bool {
    const PREFIXES: [&str; 12] = [
        "lo", "utun", "awdl", "llw", "veth", "docker", "br-", "bridge", "gif", "stf", "ap", "vmnet",
    ];
    let n = name.to_ascii_lowercase();
    n.contains("loopback") || PREFIXES.iter().any(|p| n.starts_with(p))
}

/// Percentage of `used` in `total`; 0 when total is 0.
pub fn pct(used: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    (used as f64 / total as f64 * 100.0) as f32
}

/// Bytes/sec from two counter readings; counter reset yields 0.
pub fn calc_rate(prev: u64, now: u64, secs: f64) -> f64 {
    if secs <= 0.0 {
        return 0.0;
    }
    now.saturating_sub(prev) as f64 / secs
}

pub struct Monitor {
    networks: Networks,
    sys: System,
    disks: Disks,
    last: Instant,
    pub history_down: VecDeque<f64>,
    pub history_up: VecDeque<f64>,
    pub total_down: u64,
    pub total_up: u64,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list().expect("network list"),
            sys: System::new().expect("system"),
            disks: Disks::new_with_refreshed_list().expect("disk list"),
            last: Instant::now(),
            history_down: VecDeque::with_capacity(HISTORY),
            history_up: VecDeque::with_capacity(HISTORY),
            total_down: 0,
            total_up: 0,
        }
    }

    /// Refresh counters and return current rates. `received()`/`transmitted()`
    /// are deltas since the previous refresh, so no manual bookkeeping needed.
    pub fn sample(&mut self) -> Sample {
        let now = Instant::now();
        let secs = now.duration_since(self.last).as_secs_f64();
        self.last = now;
        self.networks.refresh(true);

        let mut s = Sample::default();
        for (name, data) in self.networks.iter() {
            let name = name.to_string_lossy().into_owned();
            if is_virtual(&name) {
                continue;
            }
            let (d, u) = (data.received(), data.transmitted());
            self.total_down += d;
            self.total_up += u;
            let r = IfaceRate {
                name,
                down: calc_rate(0, d, secs),
                up: calc_rate(0, u, secs),
            };
            s.down += r.down;
            s.up += r.up;
            s.ifaces.push(r);
        }
        s.ifaces.sort_by(|a, b| a.name.cmp(&b.name));
        self.sys.refresh_cpu_usage();
        self.sys.refresh_memory();
        self.disks.refresh(true);
        s.cpu = self.sys.global_cpu_usage();
        s.mem = pct(self.sys.used_memory(), self.sys.total_memory());
        s.disk = self
            .disks
            .iter()
            .find(|d| d.mount_point() == std::path::Path::new("/"))
            .or_else(|| self.disks.iter().next())
            .map_or(0.0, |d| pct(d.total_space() - d.available_space(), d.total_space()));
        push(&mut self.history_down, s.down);
        push(&mut self.history_up, s.up);
        s
    }
}

fn push(q: &mut VecDeque<f64>, v: f64) {
    if q.len() == HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_math() {
        assert_eq!(calc_rate(100, 300, 2.0), 100.0);
        assert_eq!(calc_rate(500, 100, 1.0), 0.0); // reset
        assert_eq!(calc_rate(0, 100, 0.0), 0.0);
    }

    #[test]
    fn percent() {
        assert_eq!(pct(1, 4), 25.0);
        assert_eq!(pct(5, 0), 0.0);
    }

    #[test]
    fn filter() {
        for n in ["lo", "lo0", "utun3", "awdl0", "docker0", "veth12", "br-ab"] {
            assert!(is_virtual(n), "{n}");
        }
        for n in ["en0", "eth0", "wlan0", "Wi-Fi", "Ethernet"] {
            assert!(!is_virtual(n), "{n}");
        }
    }

    #[test]
    fn ring() {
        let mut q = VecDeque::new();
        for i in 0..(HISTORY + 5) {
            push(&mut q, i as f64);
        }
        assert_eq!(q.len(), HISTORY);
        assert_eq!(q[0], 5.0);
    }
}
