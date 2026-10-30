// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! The machine's state a measurement is read against (Sprint 240; reviewer-3's Medium 1 of its review at 240): the
//! load averages, as `run_frontier.py`'s `host_load` read them with `os.getloadavg` (147) and wrote them at a run's end
//! (173), and the QoS class the process runs at. A process cannot pin its threads to cores on Apple silicon:
//! `thread_policy_set` with `THREAD_AFFINITY_POLICY` returns `KERN_NOT_SUPPORTED` there, and the QoS class is what
//! steers a thread toward performance or efficiency cores.

unsafe extern "C" {
    /// `_stdlib.h` 328
    fn getloadavg(loadavg: *mut f64, nelem: i32) -> i32;
    /// `sys/qos.h` 172
    fn qos_class_self() -> u32;
}

/// The 1, 5 and 15 minute load averages; None when the kernel gives fewer than three.
pub fn load_average() -> Option<[f64; 3]> {
    let mut load = [0.0f64; 3];
    // SAFETY: `load` holds the three doubles `nelem` names
    let n = unsafe { getloadavg(load.as_mut_ptr(), 3) };
    (n == 3).then_some(load)
}

/// The calling thread's QoS class by its `sys/qos.h` name (130-143), or its number when the header names none.
pub fn qos_class() -> String {
    // SAFETY: no arguments, no memory
    let class = unsafe { qos_class_self() };
    match class {
        0x21 => "QOS_CLASS_USER_INTERACTIVE".into(),
        0x19 => "QOS_CLASS_USER_INITIATED".into(),
        0x15 => "QOS_CLASS_DEFAULT".into(),
        0x11 => "QOS_CLASS_UTILITY".into(),
        0x09 => "QOS_CLASS_BACKGROUND".into(),
        0x00 => "QOS_CLASS_UNSPECIFIED".into(),
        n => format!("qos class 0x{n:x}"),
    }
}

#[cfg(test)]
mod tests {
    /// The kernel gives three finite, non-negative load averages, and the QoS class is one `sys/qos.h` names.
    #[test]
    fn the_load_and_the_qos_class_are_read() {
        let load = super::load_average().expect("three load averages");
        assert!(load.iter().all(|l| l.is_finite() && *l >= 0.0), "{load:?}");
        assert!(super::qos_class().starts_with("QOS_CLASS_"), "{}", super::qos_class());
    }
}
