// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! A word list at a base address, with labels and branches resolved at the end: `vm_backend.Assembler`
//! (`src/w2m/vm_backend.py` 444-476), carried.

/// A branch's target: an address, or a label bound somewhere in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Va(u64),
    Label(usize),
}

type Make = Box<dyn Fn(i64) -> u32>;

pub struct Assembler {
    base: u64,
    words: Vec<u32>,
    fix: Vec<(usize, Make, Target)>,
    labels: Vec<Option<u64>>,
}

impl Assembler {
    pub fn new(base_va: u64) -> Assembler {
        Assembler {
            base: base_va,
            words: Vec::new(),
            fix: Vec::new(),
            labels: Vec::new(),
        }
    }

    /// The address the next word goes to.
    pub fn here(&self) -> u64 {
        self.base + 4 * self.words.len() as u64
    }

    pub fn emit(&mut self, words: &[u32]) {
        self.words.extend_from_slice(words);
    }

    pub fn label(&mut self) -> Target {
        self.labels.push(None);
        Target::Label(self.labels.len() - 1)
    }

    pub fn bind(&mut self, label: Target) {
        let Target::Label(i) = label else {
            panic!("bind takes a label")
        };
        self.labels[i] = Some(self.here());
    }

    /// A branch to `target`, its word made by `make` from the offset when the list is finished.
    pub fn branch(&mut self, make: impl Fn(i64) -> u32 + 'static, target: Target) {
        self.fix.push((self.words.len(), Box::new(make), target));
        self.words.push(0);
    }

    /// The words, every branch resolved. A label never bound panics, as the Python's KeyError did.
    pub fn finish(mut self) -> Vec<u32> {
        for (i, make, target) in &self.fix {
            let va = match target {
                Target::Va(va) => *va,
                Target::Label(l) => self.labels[*l].expect("a branch to a label never bound"),
            };
            self.words[*i] = make(va as i64 - (self.base + 4 * *i as u64) as i64);
        }
        self.words
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a64_encoder as a64;

    #[test]
    fn branches_resolve_forward_and_backward() {
        let mut a = Assembler::new(0x1000);
        let back = a.label();
        a.bind(back);
        a.emit(&[a64::hvc(1)]);
        let fwd = a.label();
        a.branch(|off| a64::cbz(16, off), fwd);
        a.branch(a64::b, back);
        a.bind(fwd);
        a.branch(a64::b, Target::Va(0x2000));
        let w = a.finish();
        assert_eq!(w, [a64::hvc(1), a64::cbz(16, 8), a64::b(-8), a64::b(0x2000 - 0x100c)]);
    }
}
