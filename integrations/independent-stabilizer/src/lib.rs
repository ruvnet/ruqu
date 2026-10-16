//! Independent, dependency-free CPU stabilizer simulation.
//!
//! Each row represents `(-1)^r i^popcount(x & z) X^x Z^z`, with qubit
//! `q` stored in bit `q % 32` of word `q / 32`. Rows `0..n` are
//! destabilizers and rows `n..2*n` are stabilizers. Unused word bits are zero.
//!
//! Mathematical provenance: the destabilizer/stabilizer tableau construction
//! and Clifford/measurement rules are described by Aaronson and Gottesman,
//! Physical Review A 70, 052328 (2004). The packed implementation and product
//! phase expression here are independently derived from the above Hermitian
//! Pauli convention. This crate implements CPU operations only.

use std::fmt;

/// Maximum aggregate requested backing storage, including measurement scratch.
/// Allocator bookkeeping and the fixed-size `Tableau` object are excluded.
pub const MAX_STORAGE_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    SizeOverflow,
    MemoryLimitExceeded { required: usize, limit: usize },
    AllocationFailed,
    InvalidQubit { qubit: usize, qubits: usize },
    SameQubit { qubit: usize },
    NonCommutingRows,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeOverflow => write!(f, "tableau size arithmetic overflow"),
            Self::MemoryLimitExceeded { required, limit } => {
                write!(f, "tableau requires {required} bytes; limit is {limit}")
            }
            Self::AllocationFailed => write!(f, "tableau allocation failed"),
            Self::InvalidQubit { qubit, qubits } => {
                write!(f, "qubit {qubit} is outside 0..{qubits}")
            }
            Self::SameQubit { qubit } => {
                write!(f, "CNOT control and target are both qubit {qubit}")
            }
            Self::NonCommutingRows => write!(f, "row product has a non-Hermitian phase"),
        }
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    H(usize),
    S(usize),
    X(usize),
    Y(usize),
    Z(usize),
    /// Controlled X: control, target.
    Cx(usize, usize),
    /// Reset to zero using the supplied bit if measurement is random.
    Reset(usize, bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowView<'a> {
    pub x: &'a [u32],
    pub z: &'a [u32],
    pub negative: bool,
}

#[derive(Debug)]
pub struct Tableau {
    n: usize,
    words: usize,
    x: Vec<u32>,
    z: Vec<u32>,
    negative: Vec<u8>,
    scratch_x: Vec<u32>,
    scratch_z: Vec<u32>,
}

fn zeros<T: Default + Clone>(len: usize) -> Result<Vec<T>, Error> {
    let mut v = Vec::new();
    v.try_reserve_exact(len).map_err(|_| Error::AllocationFailed)?;
    v.resize(len, T::default());
    Ok(v)
}

impl Tableau {
    /// Construct the all-zero computational state. Zero qubits are supported.
    /// All backing buffers are allocated here; gates and measurements allocate
    /// no additional storage.
    pub fn new(n: usize) -> Result<Self, Error> {
        let words = n.checked_add(31).ok_or(Error::SizeOverflow)? / 32;
        let rows = n.checked_mul(2).ok_or(Error::SizeOverflow)?;
        let cells = rows.checked_mul(words).ok_or(Error::SizeOverflow)?;
        let required = cells
            .checked_add(words)
            .and_then(|v| v.checked_mul(2))
            .and_then(|v| v.checked_mul(std::mem::size_of::<u32>()))
            .and_then(|v| v.checked_add(rows))
            .ok_or(Error::SizeOverflow)?;
        if required > MAX_STORAGE_BYTES {
            return Err(Error::MemoryLimitExceeded { required, limit: MAX_STORAGE_BYTES });
        }
        let mut result = Self {
            n,
            words,
            x: zeros(cells)?,
            z: zeros(cells)?,
            negative: zeros(rows)?,
            scratch_x: zeros(words)?,
            scratch_z: zeros(words)?,
        };
        for q in 0..n {
            result.x[q * words + q / 32] = 1 << (q % 32);
            result.z[(n + q) * words + q / 32] = 1 << (q % 32);
        }
        Ok(result)
    }

    pub fn qubits(&self) -> usize { self.n }

    /// Inspect a destabilizer or stabilizer row without permitting mutation.
    pub fn row(&self, index: usize) -> Option<RowView<'_>> {
        let negative = *self.negative.get(index)? != 0;
        let start = index * self.words;
        Some(RowView {
            x: &self.x[start..start + self.words],
            z: &self.z[start..start + self.words],
            negative,
        })
    }

    fn validate(&self, q: usize) -> Result<(), Error> {
        if q < self.n { Ok(()) }
        else { Err(Error::InvalidQubit { qubit: q, qubits: self.n }) }
    }

    /// Apply a Clifford gate or reset, validating every operand before mutation.
    pub fn apply(&mut self, gate: Gate) -> Result<(), Error> {
        let q = match gate {
            Gate::H(q) | Gate::S(q) | Gate::X(q) | Gate::Y(q) |
            Gate::Z(q) | Gate::Reset(q, _) | Gate::Cx(q, _) => q,
        };
        self.validate(q)?;
        if let Gate::Cx(_, t) = gate {
            self.validate(t)?;
            if q == t { return Err(Error::SameQubit { qubit: q }); }
        }
        if let Gate::Reset(_, random_bit) = gate {
            if self.measure_z(q, random_bit)? { self.apply(Gate::X(q))?; }
            return Ok(());
        }
        let word = q / 32;
        let mask = 1u32 << (q % 32);
        for row in 0..2 * self.n {
            let offset = row * self.words;
            let i = offset + word;
            let x = self.x[i] & mask != 0;
            let z = self.z[i] & mask != 0;
            match gate {
                Gate::H(_) => {
                    self.negative[row] ^= u8::from(x && z);
                    if x != z { self.x[i] ^= mask; self.z[i] ^= mask; }
                }
                Gate::S(_) => {
                    self.negative[row] ^= u8::from(x && z);
                    if x { self.z[i] ^= mask; }
                }
                Gate::X(_) => self.negative[row] ^= u8::from(z),
                Gate::Y(_) => self.negative[row] ^= u8::from(x ^ z),
                Gate::Z(_) => self.negative[row] ^= u8::from(x),
                Gate::Cx(_, t) => {
                    let j = offset + t / 32;
                    let tmask = 1u32 << (t % 32);
                    let xt = self.x[j] & tmask != 0;
                    let zt = self.z[j] & tmask != 0;
                    self.negative[row] ^= u8::from(x && zt && (xt ^ z ^ true));
                    if x { self.x[j] ^= tmask; }
                    if zt { self.z[i] ^= mask; }
                }
                Gate::Reset(_, _) => unreachable!("reset handled above"),
            }
        }
        Ok(())
    }

    /// Measure Z. A random branch returns `random_bit`; a deterministic branch
    /// ignores it. There is no RNG or hidden random-bit consumption in this API.
    pub fn measure_z(&mut self, q: usize, random_bit: bool) -> Result<bool, Error> {
        self.validate(q)?;
        let word = q / 32;
        let mask = 1u32 << (q % 32);
        let pivot = (self.n..2 * self.n)
            .find(|&r| self.x[r * self.words + word] & mask != 0);
        if let Some(p) = pivot {
            let paired = p - self.n;
            // Preserve the pivot until every product completes. This is an
            // immutable snapshot logically, without a separate allocation.
            // Preflight all phases before changing any exported row.
            for r in 0..2 * self.n {
                if r != p && r != paired && self.x[r * self.words + word] & mask != 0 {
                    product_negative(self.row(r).unwrap(), self.row(p).unwrap())?;
                }
            }
            for r in 0..2 * self.n {
                if r != p && r != paired && self.x[r * self.words + word] & mask != 0 {
                    let sign = product_negative(self.row(r).unwrap(), self.row(p).unwrap())?;
                    for w in 0..self.words {
                        self.x[r * self.words + w] ^= self.x[p * self.words + w];
                        self.z[r * self.words + w] ^= self.z[p * self.words + w];
                    }
                    self.negative[r] = u8::from(sign);
                }
            }
            for w in 0..self.words {
                self.x[paired * self.words + w] = self.x[p * self.words + w];
                self.z[paired * self.words + w] = self.z[p * self.words + w];
                self.x[p * self.words + w] = 0;
                self.z[p * self.words + w] = 0;
            }
            self.negative[paired] = self.negative[p];
            self.z[p * self.words + word] = mask;
            self.negative[p] = u8::from(random_bit);
            Ok(random_bit)
        } else {
            self.scratch_x.fill(0);
            self.scratch_z.fill(0);
            let mut negative = false;
            for d in 0..self.n {
                if self.x[d * self.words + word] & mask != 0 {
                    let s = d + self.n;
                    negative = product_negative(
                        RowView { x: &self.scratch_x, z: &self.scratch_z, negative },
                        self.row(s).unwrap(),
                    )?;
                    for w in 0..self.words {
                        self.scratch_x[w] ^= self.x[s * self.words + w];
                        self.scratch_z[w] ^= self.z[s * self.words + w];
                    }
                }
            }
            Ok(negative)
        }
    }
}

// From XZ = -ZX, the product exponent is:
// 2ra + 2rb + |xa&za| + |xb&zb| + 2|za&xb| - |(xa^xb)&(za^zb)|.
// Reduce each word modulo four to keep arithmetic bounded independently of n.
// Only internal equal-width, commuting rows are accepted by callers.
fn product_negative(a: RowView<'_>, b: RowView<'_>) -> Result<bool, Error> {
    let mut e = 2 * i32::from(a.negative) + 2 * i32::from(b.negative);
    for w in 0..a.x.len() {
        e += (a.x[w] & a.z[w]).count_ones() as i32
            + (b.x[w] & b.z[w]).count_ones() as i32
            + 2 * (a.z[w] & b.x[w]).count_ones() as i32
            - ((a.x[w] ^ b.x[w]) & (a.z[w] ^ b.z[w])).count_ones() as i32;
        e = e.rem_euclid(4);
    }
    if e & 1 != 0 { Err(Error::NonCommutingRows) }
    else { Ok(e & 2 != 0) }
}
