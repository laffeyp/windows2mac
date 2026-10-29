// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (C) 2026 Peter Laffey
//! CPython 3.12's `random.Random` as the frozen Python host uses it for the guest's random bytes on the instruction
//! clock (Sprint 203): `random.Random(0x5EED).randbytes(n)` (vm_backend 2357, 3840-3842), so two runs of one tree make
//! the same calls. MT19937 as `Modules/_randommodule.c` has it (fetched 2026-10-28 from the 3.12 branch: N, M and the
//! masks 85-89, `genrand_uint32` 129-162, `init_genrand` 190-211, `init_by_array` 213-242, an integer seed's words
//! from the least significant 282-345, `getrandbits` 488-535), and `randbytes` as `Lib/random.py` 284-286 has it,
//! `getrandbits(n * 8).to_bytes(n, 'little')`.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

pub struct Random {
    state: [u32; N],
    index: usize,
}

impl Random {
    /// `random.Random(seed)` for a non-negative integer seed: `init_by_array` over its 32-bit words, least significant
    /// first, one word for 0.
    pub fn new(seed: u128) -> Random {
        let mut key = Vec::new();
        let mut n = seed;
        loop {
            key.push(n as u32);
            n >>= 32;
            if n == 0 {
                break;
            }
        }
        let mut r = Random {
            state: [0; N],
            index: N + 1,
        };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.state[0] = s;
        for i in 1..N {
            let prev = self.state[i - 1];
            self.state[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mt = &mut self.state;
        let (mut i, mut j) = (1usize, 0usize);
        for _ in 0..N.max(key.len()) {
            let prev = mt[i - 1];
            mt[i] = (mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                mt[0] = mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..N - 1 {
            let prev = mt[i - 1];
            mt[i] = (mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                mt[0] = mt[N - 1];
                i = 1;
            }
        }
        mt[0] = 0x8000_0000;
    }

    /// `genrand_uint32`
    pub fn next_u32(&mut self) -> u32 {
        let mt = &mut self.state;
        if self.index >= N {
            let mag = |y: u32| if y & 1 != 0 { MATRIX_A } else { 0 };
            for kk in 0..N - M {
                let y = (mt[kk] & UPPER_MASK) | (mt[kk + 1] & LOWER_MASK);
                mt[kk] = mt[kk + M] ^ (y >> 1) ^ mag(y);
            }
            for kk in N - M..N - 1 {
                let y = (mt[kk] & UPPER_MASK) | (mt[kk + 1] & LOWER_MASK);
                mt[kk] = mt[kk + M - N] ^ (y >> 1) ^ mag(y);
            }
            let y = (mt[N - 1] & UPPER_MASK) | (mt[0] & LOWER_MASK);
            mt[N - 1] = mt[M - 1] ^ (y >> 1) ^ mag(y);
            self.index = 0;
        }
        let mut y = mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `randbytes(n)`: `getrandbits(8 n)` filled by 32-bit words from the least significant, the last word shifted
    /// right when fewer than 32 bits remain, then the integer's `n` little-endian bytes.
    pub fn randbytes(&mut self, n: usize) -> Vec<u8> {
        let mut k = 8 * n;
        if k == 0 {
            return Vec::new();
        }
        if k <= 32 {
            let r = self.next_u32() >> (32 - k);
            return r.to_le_bytes()[..n].to_vec();
        }
        let mut out = Vec::with_capacity(n + 3);
        while k > 0 {
            let mut r = self.next_u32();
            if k < 32 {
                r >>= 32 - k;
            }
            out.extend(r.to_le_bytes());
            k = k.saturating_sub(32);
        }
        out.truncate(n);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Random;

    /// mt19937ar.out (Matsumoto and Nishimura's reference output, fetched 2026-10-28): `init_by_array({0x123, 0x234,
    /// 0x345, 0x456})`, the first ten outputs of `genrand_int32`.
    #[test]
    fn the_reference_vector_is_reproduced() {
        let mut r = Random {
            state: [0; 624],
            index: 625,
        };
        r.init_by_array(&[0x123, 0x234, 0x345, 0x456]);
        let first: Vec<u32> = (0..10).map(|_| r.next_u32()).collect();
        assert_eq!(
            first,
            [
                1067595299, 955945823, 477289528, 4107218783, 4228976476, 3344332714, 3355579695, 227628506, 810200273,
                2591290167
            ]
        );
    }

    /// `random_seed` (`_randommodule.c`) splits the seed's absolute value into 32-bit words, least significant first:
    /// the reference key `{0x123, 0x234, 0x345, 0x456}` is the seed 0x456_00000345_00000234_00000123.
    #[test]
    fn a_seed_is_split_into_words_least_significant_first() {
        let mut r = Random::new(0x456_0000_0345_0000_0234_0000_0123);
        let first: Vec<u32> = (0..3).map(|_| r.next_u32()).collect();
        assert_eq!(first, [1067595299, 955945823, 477289528]);
    }

    /// `randbytes` is `getrandbits(8 n).to_bytes(n, 'little')`: whole words little-endian, and for a byte count that
    /// is not a multiple of 4 the last word shifted right, its low bytes kept.
    #[test]
    fn randbytes_takes_whole_words_little_endian_and_shifts_the_last() {
        let mut a = Random::new(0x5EED);
        let mut b = Random::new(0x5EED);
        let (w0, w1) = (b.next_u32(), b.next_u32());
        let mut want = w0.to_le_bytes().to_vec();
        want.extend(&(w1 >> 16).to_le_bytes()[..2]);
        assert_eq!(a.randbytes(6), want);
        let mut c = Random::new(0x5EED);
        assert_eq!(c.randbytes(2), (w0 >> 16).to_le_bytes()[..2].to_vec());
    }
}
