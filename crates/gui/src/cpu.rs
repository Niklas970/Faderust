//! Writes to the log which thread of Faderust keeps the processor busy.
//!
//! Every 30 seconds the CPU time of each thread is compared with the last
//! look. Threads above 5 % of a core are logged by name, so a report like
//! "uses 14 % after sleep" says whether it is the audio engine, the window
//! or something else. Quiet times log nothing.

use std::collections::HashMap;

/// CPU time of one thread in 100 ns units.
#[cfg_attr(not(windows), allow(dead_code))]
pub struct Sample {
    pub id: u32,
    pub name: String,
    pub ticks: u64,
}

/// Threads above `min_percent` of one core since `before`, busiest first.
/// A thread that is new counts from zero.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn busy(
    before: &HashMap<u32, u64>,
    after: &[Sample],
    seconds: f64,
    min_percent: f64,
) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = after
        .iter()
        .filter_map(|s| {
            let used = s
                .ticks
                .saturating_sub(before.get(&s.id).copied().unwrap_or(0));
            let percent = used as f64 / 1e7 / seconds * 100.0;
            (percent >= min_percent).then(|| (s.name.clone(), percent))
        })
        .collect();
    out.sort_by(|a, b| b.1.total_cmp(&a.1));
    out
}

#[cfg(windows)]
pub fn start() {
    use std::time::{Duration, Instant};
    const EVERY: Duration = Duration::from_secs(30);
    let _ = std::thread::Builder::new()
        .name("faderust-cpu".into())
        .spawn(|| {
            let mut before: HashMap<u32, u64> = windows_impl::sample()
                .iter()
                .map(|s| (s.id, s.ticks))
                .collect();
            let mut at = Instant::now();
            loop {
                std::thread::sleep(EVERY);
                let now = windows_impl::sample();
                let seconds = at.elapsed().as_secs_f64();
                at = Instant::now();
                let top = busy(&before, &now, seconds, 5.0);
                before = now.iter().map(|s| (s.id, s.ticks)).collect();
                if !top.is_empty() {
                    let list: Vec<String> = top
                        .iter()
                        .take(4)
                        .map(|(name, p)| format!("{name} {p:.0} %"))
                        .collect();
                    faderust_core::log::write(&format!("CPU: {}", list.join(", ")));
                }
            }
        });
}

#[cfg(windows)]
mod windows_impl {
    use super::Sample;
    use windows::Win32::Foundation::{CloseHandle, LocalFree, FILETIME, HLOCAL};
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcessId, GetThreadDescription, GetThreadTimes, OpenThread,
        THREAD_QUERY_LIMITED_INFORMATION,
    };

    fn ticks(t: FILETIME) -> u64 {
        (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime)
    }

    /// CPU time and name of every thread of this process.
    pub fn sample() -> Vec<Sample> {
        let mut out = Vec::new();
        // SAFETY: plain Win32 calls; every handle opened here is closed here.
        unsafe {
            let pid = GetCurrentProcessId();
            let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) else {
                return out;
            };
            let mut entry = THREADENTRY32 {
                dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };
            let mut more = Thread32First(snapshot, &mut entry).is_ok();
            while more {
                if entry.th32OwnerProcessID == pid {
                    if let Ok(thread) =
                        OpenThread(THREAD_QUERY_LIMITED_INFORMATION, false, entry.th32ThreadID)
                    {
                        let (mut created, mut exited, mut kernel, mut user) = (
                            FILETIME::default(),
                            FILETIME::default(),
                            FILETIME::default(),
                            FILETIME::default(),
                        );
                        if GetThreadTimes(thread, &mut created, &mut exited, &mut kernel, &mut user)
                            .is_ok()
                        {
                            let name = match GetThreadDescription(thread) {
                                Ok(text) => {
                                    let name = text.to_string().unwrap_or_default();
                                    let _ = LocalFree(Some(HLOCAL(text.0.cast())));
                                    name
                                }
                                Err(_) => String::new(),
                            };
                            let name = if name.is_empty() {
                                format!("thread {}", entry.th32ThreadID)
                            } else {
                                name
                            };
                            out.push(Sample {
                                id: entry.th32ThreadID,
                                name,
                                ticks: ticks(kernel) + ticks(user),
                            });
                        }
                        let _ = CloseHandle(thread);
                    }
                }
                more = Thread32Next(snapshot, &mut entry).is_ok();
            }
            let _ = CloseHandle(snapshot);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_busy_threads_are_named_busiest_first() {
        let before = HashMap::from([(1, 0), (2, 0), (3, 0)]);
        let s = |id, name: &str, secs: f64| Sample {
            id,
            name: name.into(),
            ticks: (secs * 1e7) as u64,
        };
        // Over 30 s: 3 s = 10 %, 0.3 s = 1 %, 6 s = 20 %.
        let after = [s(1, "gui", 3.0), s(2, "idle", 0.3), s(3, "engine", 6.0)];
        let top = busy(&before, &after, 30.0, 5.0);
        let names: Vec<&str> = top.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["engine", "gui"]);
        assert!((top[0].1 - 20.0).abs() < 0.01);
    }
}
