//! CPU and memory of a process tree, read from /proc.

use std::collections::HashMap;

const PAGE_BYTES: u64 = 4096;
/// Kernel clock ticks per second (USER_HZ), 100 on every mainstream Linux.
pub const TICKS_PER_SECOND: f64 = 100.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcStat {
    pub pid: u32,
    pub parent: u32,
    /// User + system CPU time, in clock ticks.
    pub ticks: u64,
    pub memory_bytes: u64,
}

/// Parses one `/proc/<pid>/stat` line. The process name may contain spaces and brackets.
pub fn parse_stat(line: &str) -> Option<ProcStat> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let pid = line[..open].trim().parse().ok()?;
    let rest: Vec<&str> = line[close + 1..].split_whitespace().collect();
    Some(ProcStat {
        pid,
        parent: rest.get(1)?.parse().ok()?,
        ticks: rest.get(11)?.parse::<u64>().ok()? + rest.get(12)?.parse::<u64>().ok()?,
        memory_bytes: rest.get(21)?.parse::<u64>().ok()? * PAGE_BYTES,
    })
}

pub fn snapshot() -> Vec<ProcStat> {
    let Ok(dir) = std::fs::read_dir("/proc") else { return vec![] };
    dir.flatten()
        .filter(|e| e.file_name().to_string_lossy().bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|e| std::fs::read_to_string(e.path().join("stat")).ok())
        .filter_map(|s| parse_stat(&s))
        .collect()
}

/// `root` and everything started beneath it.
pub fn tree(all: &[ProcStat], root: u32) -> Vec<ProcStat> {
    let mut children: HashMap<u32, Vec<&ProcStat>> = HashMap::new();
    for p in all {
        children.entry(p.parent).or_default().push(p);
    }
    let mut out: Vec<ProcStat> = all.iter().filter(|p| p.pid == root).copied().collect();
    let mut queue = vec![root];
    while let Some(pid) = queue.pop() {
        for child in children.get(&pid).into_iter().flatten() {
            out.push(**child);
            queue.push(child.pid);
        }
    }
    out
}

/// Percent of one CPU core used between two readings.
pub fn cpu_percent(ticks_before: u64, ticks_now: u64, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        return 0.0;
    }
    ticks_now.saturating_sub(ticks_before) as f64 / TICKS_PER_SECOND / seconds * 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINE: &str = "4242 (Web Content (x) y) S 100 4242 4242 0 -1 4194304 500 0 0 0 150 50 0 0 20 0 8 0 12345 1000000 2560 18446744073709551615 1 1 0 0 0 0 0 0 0 0 0 0 17 3 0 0 0 0 0";

    #[test]
    fn parses_a_stat_line_with_an_awkward_name() {
        let p = parse_stat(LINE).unwrap();
        assert_eq!((p.pid, p.parent, p.ticks), (4242, 100, 200));
        assert_eq!(p.memory_bytes, 2560 * 4096);
        assert_eq!(parse_stat("garbage"), None);
    }

    #[test]
    fn collects_a_whole_process_tree() {
        let p = |pid, parent| ProcStat { pid, parent, ticks: 1, memory_bytes: 10 };
        let all = [p(1, 0), p(10, 1), p(11, 10), p(12, 11), p(20, 1), p(21, 20)];
        let mut pids: Vec<u32> = tree(&all, 10).iter().map(|p| p.pid).collect();
        pids.sort();
        assert_eq!(pids, vec![10, 11, 12]);
        assert!(tree(&all, 999).is_empty());
    }

    #[test]
    fn cpu_percent_is_per_core() {
        assert_eq!(cpu_percent(100, 300, 2.0), 100.0);
        assert_eq!(cpu_percent(300, 100, 2.0), 0.0);
        assert_eq!(cpu_percent(0, 50, 0.0), 0.0);
    }

    #[test]
    fn reads_this_process_from_proc() {
        let me = std::process::id();
        assert!(snapshot().iter().any(|p| p.pid == me && p.memory_bytes > 0));
    }
}
