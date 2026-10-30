// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! `approximate.py` (the tag frozen-python-2026-10-29; Sprint 237): RCPPS, RCPSS, RSQRTPS and RSQRTSS as the
//! reference CPU computes them. The SDM bounds their relative error by 1.5 * 2^-12 and leaves the bits to the CPU; the
//! tables in `specs/approximations.json` are one CPU's (Intel Xeon 8573C). A normal input's result is the entry for its
//! mantissa's top 12 bits with the exponent moved; zeros and denormals give a signed infinity, infinities a signed
//! zero (RSQRT of -inf the default NaN), NaNs are quieted, negative RSQRT inputs give the default NaN, and a result
//! below the normal range is a signed zero. They raise no SIMD floating-point exception.

use std::sync::OnceLock;

const DEFAULT_NAN: u32 = 0xFFC0_0000;

/// The three tables, 4,096 entries each, read once from the spec compiled in.
struct Tables {
    rcp: Vec<u32>,
    rsqrt_even: Vec<u32>,
    rsqrt_odd: Vec<u32>,
}

fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let spec: serde_json::Value = serde_json::from_str(include_str!("../../../specs/approximations.json"))
            .expect("specs/approximations.json");
        let table = |name: &str| -> Vec<u32> {
            let t: Vec<u32> = spec[name]
                .as_array()
                .unwrap_or_else(|| panic!("specs/approximations.json: no {name}"))
                .iter()
                .map(|v| v.as_u64().expect("a table entry") as u32)
                .collect();
            assert_eq!(
                t.len(),
                4096,
                "specs/approximations.json: {name} has {} entries",
                t.len()
            );
            t
        };
        Tables {
            rcp: table("rcp"),
            rsqrt_even: table("rsqrt_even"),
            rsqrt_odd: table("rsqrt_odd"),
        }
    })
}

/// `_special` (approximate.py 31-40): a NaN, an infinity, a zero or denormal, or a negative RSQRT input, by rule.
fn special(x: u32, rsqrt: bool) -> Option<u32> {
    let (sign, e, m) = (x >> 31, (x >> 23) & 0xFF, x & 0x7F_FFFF);
    if e == 0xFF && m != 0 {
        return Some(x | 0x40_0000); // NaN: quieted
    }
    if e == 0xFF {
        return Some(if rsqrt && sign != 0 { DEFAULT_NAN } else { sign << 31 }); // infinity
    }
    if e == 0 {
        return Some(sign << 31 | 0x7F80_0000); // zero or denormal: an infinity of its sign
    }
    (rsqrt && sign != 0).then_some(DEFAULT_NAN)
}

/// `rcp` (approximate.py 51-57)
pub fn rcp(x: u32) -> u32 {
    if let Some(s) = special(x, false) {
        return s;
    }
    let t = tables().rcp[((x >> 11) & 0xFFF) as usize];
    let e = i64::from((t >> 23) & 0xFF) - (i64::from((x >> 23) & 0xFF) - 127);
    (x & 0x8000_0000)
        | if (1..=254).contains(&e) {
            t & 0x7F_FFFF | (e as u32) << 23
        } else {
            0
        }
}

/// `rsqrt` (approximate.py 60-68): Python's `%` and `//` floor, as `rem_euclid` and `div_euclid` do by 2.
pub fn rsqrt(x: u32) -> u32 {
    if let Some(s) = special(x, true) {
        return s;
    }
    let e_in = i64::from((x >> 23) & 0xFF);
    let even = (e_in - 127).rem_euclid(2) == 0;
    let e0 = if even { 127 } else { 128 };
    let table = if even {
        &tables().rsqrt_even
    } else {
        &tables().rsqrt_odd
    };
    let t = table[((x >> 11) & 0xFFF) as usize];
    let e = i64::from((t >> 23) & 0xFF) - (e_in - e0).div_euclid(2);
    if (1..=254).contains(&e) {
        t & 0x7F_FFFF | (e as u32) << 23
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{rcp, rsqrt};

    /// Every special input the reference CPU was measured on (`specials` in the spec: input -> [RCP, RSQRT]) gives
    /// the CPU's result.
    #[test]
    fn the_specials_are_the_reference_cpus() {
        let spec: serde_json::Value = serde_json::from_str(include_str!("../../../specs/approximations.json")).unwrap();
        let specials = spec["specials"].as_object().unwrap();
        assert!(!specials.is_empty());
        for (input, want) in specials {
            let x = u32::from_str_radix(input, 16).unwrap();
            let (r, s) = (want[0].as_u64().unwrap() as u32, want[1].as_u64().unwrap() as u32);
            assert_eq!(rcp(x), r, "RCP of {input}: 0x{:08x}, the CPU 0x{r:08x}", rcp(x));
            assert_eq!(rsqrt(x), s, "RSQRT of {input}: 0x{:08x}, the CPU 0x{s:08x}", rsqrt(x));
        }
    }
}
