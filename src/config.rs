//! Tiny `key=value` config persisted in the OS config dir.
use std::path::PathBuf;

pub const INTERVALS: [u64; 3] = [1, 2, 5];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    pub bits: bool,
    pub interval_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self { bits: false, interval_secs: 1 }
    }
}

impl Config {
    pub fn parse(text: &str) -> Self {
        let mut c = Self::default();
        for line in text.lines() {
            match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
                Some(("bits", v)) => c.bits = v == "true",
                Some(("interval", v)) => {
                    if let Some(n) = v.parse().ok().filter(|n| INTERVALS.contains(n)) {
                        c.interval_secs = n;
                    }
                }
                _ => {}
            }
        }
        c
    }

    pub fn serialize(&self) -> String {
        format!("bits={}\ninterval={}\n", self.bits, self.interval_secs)
    }

    fn path() -> Option<PathBuf> {
        Some(dirs::config_dir()?.join("network-monitor").join("config"))
    }

    pub fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| Self::parse(&t))
            .unwrap_or_default()
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, self.serialize());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let c = Config { bits: true, interval_secs: 5 };
        assert_eq!(Config::parse(&c.serialize()), c);
    }

    #[test]
    fn bad_input_falls_back() {
        assert_eq!(Config::parse("interval=7\nbits=maybe\njunk"), Config::default());
    }
}
