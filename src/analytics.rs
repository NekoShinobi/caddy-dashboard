use crate::log_parser::LogEntry;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const MINUTE_SECS: u64 = 60;
pub const HOUR_SECS: u64 = 3_600;
pub const DAY_SECS: u64 = 86_400;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Resolution {
    Minute,
    Hour,
    Day,
}

impl Resolution {
    pub fn seconds(self) -> u64 {
        match self {
            Self::Minute => MINUTE_SECS,
            Self::Hour => HOUR_SECS,
            Self::Day => DAY_SECS,
        }
    }
}

#[derive(Clone, Debug)]
pub enum RangeSegment {
    Raw { start: f64, end: f64 },
    Rollup { resolution: Resolution, bucket: u64 },
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Histogram {
    bins: BTreeMap<u16, u64>,
    count: u64,
}

impl Histogram {
    const SCALE: f64 = 16.0;
    const OFFSET: i32 = 512;
    const MAX_BIN: i32 = 1_023;

    pub fn add(&mut self, value: f64) {
        let bin = if !value.is_finite() || value <= 0.0 {
            0
        } else {
            ((value.log2() * Self::SCALE).round() as i32 + Self::OFFSET).clamp(1, Self::MAX_BIN)
                as u16
        };
        *self.bins.entry(bin).or_insert(0) += 1;
        self.count += 1;
    }

    pub fn merge(&mut self, other: &Self) {
        for (bin, count) in &other.bins {
            *self.bins.entry(*bin).or_insert(0) += count;
        }
        self.count += other.count;
    }

    pub fn quantile(&self, percentile: f64) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        let target = ((percentile.clamp(0.0, 100.0) / 100.0) * self.count as f64)
            .ceil()
            .max(1.0) as u64;
        let mut seen = 0u64;
        for (bin, count) in &self.bins {
            seen += count;
            if seen >= target {
                if *bin == 0 {
                    return 0.0;
                }
                return 2f64.powf((*bin as i32 - Self::OFFSET) as f64 / Self::SCALE);
            }
        }
        0.0
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct PathRollup {
    pub count: u64,
    pub duration_sum_ms: f64,
    pub durations: Histogram,
}

impl PathRollup {
    fn add(&mut self, duration_ms: f64) {
        self.count += 1;
        self.duration_sum_ms += duration_ms;
        self.durations.add(duration_ms);
    }

    fn merge(&mut self, other: &Self) {
        self.count += other.count;
        self.duration_sum_ms += other.duration_sum_ms;
        self.durations.merge(&other.durations);
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Rollup {
    pub total: u64,
    pub status_codes: HashMap<u16, u64>,
    pub paths: HashMap<String, PathRollup>,
    pub hosts: HashMap<String, u64>,
    pub ips: HashMap<String, u64>,
    pub duration_sum_ms: f64,
    pub durations: Histogram,
    pub total_bytes: u64,
    pub sizes: Histogram,
    pub client_hashes: Vec<u64>,
    pub referrers: HashMap<String, u64>,
    pub user_agents: HashMap<String, u64>,
    pub methods: HashMap<String, u64>,
}

impl Rollup {
    pub fn add_entry(&mut self, entry: &LogEntry) {
        self.total += 1;
        *self.status_codes.entry(entry.status).or_insert(0) += 1;

        let duration_ms = entry.duration * 1_000.0;
        let path = format!("{}{}", entry.request.host, entry.request.uri);
        self.paths.entry(path).or_default().add(duration_ms);
        *self.hosts.entry(entry.request.host.clone()).or_insert(0) += 1;
        *self.ips.entry(entry.request.client_ip.clone()).or_insert(0) += 1;
        *self
            .methods
            .entry(entry.request.method.clone())
            .or_insert(0) += 1;

        self.duration_sum_ms += duration_ms;
        self.durations.add(duration_ms);
        self.total_bytes = self.total_bytes.saturating_add(entry.size);
        self.sizes.add(entry.size as f64);
        self.client_hashes
            .push(stable_hash(entry.request.client_ip.as_bytes()));

        if let Some(value) = header_first(entry, &["Referer", "Referrer"]) {
            if !value.is_empty() {
                *self.referrers.entry(value.to_owned()).or_insert(0) += 1;
            }
        }
        if let Some(value) = header_first(entry, &["User-Agent"]) {
            if !value.is_empty() {
                *self.user_agents.entry(value.to_owned()).or_insert(0) += 1;
            }
        }
    }

    pub fn finish_bucket(&mut self) {
        self.client_hashes.sort_unstable();
        self.client_hashes.dedup();
    }

    pub fn merge(&mut self, other: &Self) {
        self.total += other.total;
        merge_counts(&mut self.status_codes, &other.status_codes);
        merge_counts(&mut self.hosts, &other.hosts);
        merge_counts(&mut self.ips, &other.ips);
        merge_counts(&mut self.referrers, &other.referrers);
        merge_counts(&mut self.user_agents, &other.user_agents);
        merge_counts(&mut self.methods, &other.methods);
        for (path, value) in &other.paths {
            self.paths.entry(path.clone()).or_default().merge(value);
        }

        self.duration_sum_ms += other.duration_sum_ms;
        self.durations.merge(&other.durations);
        self.total_bytes = self.total_bytes.saturating_add(other.total_bytes);
        self.sizes.merge(&other.sizes);
        self.client_hashes.extend_from_slice(&other.client_hashes);
    }

    pub fn unique_clients(&self) -> usize {
        self.client_hashes
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len()
    }
}

pub fn bucket_start(ts: f64, bucket_secs: u64) -> u64 {
    let seconds = ts.max(0.0) as u64;
    (seconds / bucket_secs) * bucket_secs
}

pub fn build_updates(entries: &[LogEntry], bucket_secs: u64) -> HashMap<u64, Rollup> {
    let mut updates: HashMap<u64, Rollup> = HashMap::new();
    for entry in entries {
        updates
            .entry(bucket_start(entry.ts, bucket_secs))
            .or_default()
            .add_entry(entry);
    }
    for rollup in updates.values_mut() {
        rollup.finish_bucket();
    }
    updates
}

pub fn range_segments(start: f64, end: f64) -> Vec<RangeSegment> {
    let mut segments = Vec::new();
    if !start.is_finite() || !end.is_finite() || start >= end {
        return segments;
    }

    let mut cursor = start.max(0.0);
    if cursor.fract() != 0.0 || (cursor as u64) % MINUTE_SECS != 0 {
        let next_minute = ((cursor / MINUTE_SECS as f64).floor() + 1.0) * MINUTE_SECS as f64;
        let raw_end = next_minute.min(end);
        segments.push(RangeSegment::Raw {
            start: cursor,
            end: raw_end,
        });
        cursor = raw_end;
    }

    while cursor < end {
        let second = cursor as u64;
        let choice = if second % DAY_SECS == 0 && cursor + DAY_SECS as f64 <= end {
            Some(Resolution::Day)
        } else if second % HOUR_SECS == 0 && cursor + HOUR_SECS as f64 <= end {
            Some(Resolution::Hour)
        } else if second % MINUTE_SECS == 0 && cursor + MINUTE_SECS as f64 <= end {
            Some(Resolution::Minute)
        } else {
            None
        };

        if let Some(resolution) = choice {
            segments.push(RangeSegment::Rollup {
                resolution,
                bucket: second,
            });
            cursor += resolution.seconds() as f64;
        } else {
            segments.push(RangeSegment::Raw { start: cursor, end });
            break;
        }
    }
    segments
}

fn merge_counts<K>(target: &mut HashMap<K, u64>, source: &HashMap<K, u64>)
where
    K: Clone + Eq + std::hash::Hash,
{
    for (key, value) in source {
        *target.entry(key.clone()).or_insert(0) += value;
    }
}

fn header_first<'a>(entry: &'a LogEntry, names: &[&str]) -> Option<&'a str> {
    names
        .iter()
        .find_map(|name| entry.request.headers.get(*name))
        .and_then(|values| values.first())
        .map(String::as_str)
}

fn stable_hash(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_quantiles_are_mergeable() {
        let mut left = Histogram::default();
        let mut right = Histogram::default();
        for value in 1..=50 {
            left.add(value as f64);
        }
        for value in 51..=100 {
            right.add(value as f64);
        }
        left.merge(&right);

        assert_eq!(left.count, 100);
        assert!((left.quantile(50.0) - 50.0).abs() < 4.0);
        assert!((left.quantile(99.0) - 99.0).abs() < 8.0);
    }

    #[test]
    fn range_decomposition_uses_large_aligned_buckets() {
        let segments = range_segments(30.0, DAY_SECS as f64 + HOUR_SECS as f64 + 90.0);
        assert!(matches!(segments.first(), Some(RangeSegment::Raw { .. })));
        assert!(segments.iter().any(|segment| matches!(
            segment,
            RangeSegment::Rollup {
                resolution: Resolution::Hour,
                ..
            }
        )));
        assert!(segments.iter().any(|segment| matches!(
            segment,
            RangeSegment::Rollup {
                resolution: Resolution::Minute,
                ..
            }
        )));
    }
}
