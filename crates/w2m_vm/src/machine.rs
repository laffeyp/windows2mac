// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! kernel.py's `Machine` in Rust (Sprint 197; reviewer-2's High 1 of its review of Sprints 193-196): the processes of a
//! run take turns, one at a time, on one machine clock. Each process is a VM of its own (Hypervisor.framework allows
//! one VM per process), a second `w2m-vm` for a child; a process runs its turn until it is idle (no thread ready) or its
//! slice ends while another process is due (`VM.steps` yields "idle" or "slice", vm_backend 5066, 5082), and the
//! supervisor gives the next turn (`_next`, kernel.py 384-399).

/// What a process reports at the end of its turn, and what the supervisor reads to choose the next: whether it has a
/// thread to run now (`VM.ready`, vm_backend 3653-3662, the machine clock's passed deadlines included), and whether one
/// of its threads waits with a deadline (`waits_with_deadline`, 3689-3690).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turn {
    pub pid: u64,
    /// not yet started: it has a turn to take (`p.steps is None`)
    pub started: bool,
    pub ready: bool,
    pub timed: bool,
}

/// The supervisor's choice (`_next`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// this process runs
    Run(u64),
    /// no process is ready; this one waits with a deadline and runs, its clock warped to the deadline on the
    /// instruction clock, or after a millisecond on the wall clock
    Timed(u64),
    /// every thread of every process waits with no timeout (`_deadlocked`)
    Deadlock,
}

/// `_next` (kernel.py 384-399) for processes on one vCPU each: in the rotation after `current` (current last), the
/// first not yet started or ready; else the first that waits with a deadline; else a deadlock. `live` is in the order
/// the processes joined (`machine.vms`).
pub fn next(live: &[Turn], current: u64) -> Next {
    if live.is_empty() {
        return Next::Deadlock;
    }
    let i = live.iter().position(|t| t.pid == current);
    let rotation: Vec<&Turn> = match i {
        Some(i) => live[i + 1..].iter().chain(live[..=i].iter()).collect(),
        None => live.iter().collect(),
    };
    if let Some(t) = rotation.iter().find(|t| !t.started || t.ready) {
        return Next::Run(t.pid);
    }
    match rotation.iter().find(|t| t.timed) {
        Some(t) => Next::Timed(t.pid),
        None => Next::Deadlock,
    }
}

/// `_machine_now` (vm_backend 3671-3678) on the instruction clock: the latest of the processes' clocks, each counting
/// only its own instructions, over the processes that have threads.
pub fn machine_now(clocks: &[u64]) -> u64 {
    clocks.iter().copied().max().unwrap_or(0)
}

/// `_catch_up` (3680-3688): the warp to add to a process whose clock reads `own` when it runs again, so that it reads
/// the machine's clock; 0 when it is not behind, or when it is the only process.
pub fn catch_up(own: u64, machine: u64, processes: usize) -> u64 {
    if processes < 2 { 0 } else { machine.saturating_sub(own) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turn(pid: u64, started: bool, ready: bool, timed: bool) -> Turn {
        Turn {
            pid,
            started,
            ready,
            timed,
        }
    }

    /// kernel.py 384-399: the rotation starts after the current process, which comes last; a child not yet started
    /// takes the next turn; with none ready a timed waiter runs; with none of either, a deadlock.
    #[test]
    fn the_next_turn_goes_round_from_the_current_process() {
        let parent = turn(4096, true, true, false);
        let child = turn(4100, false, false, false);
        assert_eq!(next(&[parent, child], 4096), Next::Run(4100));
        let child = turn(4100, true, false, false);
        assert_eq!(next(&[parent, child], 4096), Next::Run(4096));
        let (idle, sleeping) = (turn(4096, true, false, false), turn(4100, true, false, true));
        assert_eq!(next(&[idle, sleeping], 4100), Next::Timed(4100));
        let waiting = turn(4100, true, false, false);
        assert_eq!(next(&[idle, waiting], 4096), Next::Deadlock);
        assert_eq!(next(&[], 4096), Next::Deadlock);
    }

    /// vm_backend 3671-3688: the machine's clock is the latest; a process behind it is warped up when it runs again,
    /// and a lone process never is (twoproc.c's child reads a clock not behind its parent's).
    #[test]
    fn a_process_behind_the_machine_clock_is_caught_up() {
        assert_eq!(machine_now(&[900, 5_000, 1_200]), 5_000);
        assert_eq!(catch_up(1_200, 5_000, 2), 3_800);
        assert_eq!(catch_up(6_000, 5_000, 2), 0);
        assert_eq!(catch_up(1_200, 5_000, 1), 0);
    }
}
