//! Base-52 `[a-z][A-Z]` ordering used for file_order and section_order.
//!
//! Digit values: a=0, b=1, …, z=25, A=26, B=27, …, Z=51. Total 52 per digit.
//! Orders are big-endian fixed-width ("full digit") strings. The width is a
//! property of the container: every row must use the same width for its
//! file_order, and every file must use the same width for its section_order.
//!
//! The all-lowest-digit order (e.g. `a`, `aa`, `aaa`) denotes the `header`
//! pseudo-file when used as a file_order.

use crate::error::{Result, TmceError};

pub const RADIX: u64 = 52;

/// Parse a single base-52 digit character.
pub fn digit_value(c: char) -> Option<u64> {
    match c {
        'a'..='z' => Some((c as u8 - b'a') as u64),
        'A'..='Z' => Some((c as u8 - b'A') as u64 + 26),
        _ => None,
    }
}

/// Render a single digit value (0..52) as its character.
pub fn digit_char(v: u64) -> Option<char> {
    match v {
        0..=25 => Some((b'a' + v as u8) as char),
        26..=51 => Some((b'A' + (v - 26) as u8) as char),
        _ => None,
    }
}

/// A fixed-width base-52 order string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Order {
    /// Most-significant digit first. Length = width of this order.
    digits: Vec<u8>,
}

impl Order {
    pub fn width(&self) -> usize {
        self.digits.len()
    }

    pub fn value(&self) -> u64 {
        self.digits.iter().fold(0u64, |acc, &d| acc * RADIX + d as u64)
    }

    /// Construct from an integer value at a given fixed width.
    /// Returns `None` if the value does not fit.
    pub fn from_value(mut v: u64, width: usize) -> Option<Self> {
        let mut digits = vec![0u8; width];
        for i in (0..width).rev() {
            digits[i] = (v % RADIX) as u8;
            v /= RADIX;
        }
        if v != 0 {
            return None;
        }
        Some(Order { digits })
    }

    /// The header file_order at a given width: all lowest digit (`a…a`).
    pub fn header(width: usize) -> Self {
        Order {
            digits: vec![0u8; width],
        }
    }

    /// Parse from a string. Every char must be a base-52 digit.
    /// Width is inferred from the string length.
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(TmceError::InvalidOrder(
                "order string cannot be empty".into(),
            ));
        }
        let mut digits = Vec::with_capacity(s.len());
        for c in s.chars() {
            let v = digit_value(c).ok_or_else(|| {
                TmceError::InvalidOrder(format!(
                    "char {:?} is not a base-52 digit in {:?}",
                    c, s
                ))
            })?;
            digits.push(v as u8);
        }
        Ok(Order { digits })
    }

    pub fn render(&self) -> String {
        self.digits
            .iter()
            .map(|&d| digit_char(d as u64).unwrap())
            .collect()
    }

    /// Minimum width needed to represent `count` distinct positions
    /// (i.e. enough to index values 0..count).
    pub fn min_width_for(count: u64) -> usize {
        if count <= 1 {
            return 1;
        }
        let mut w = 0usize;
        let mut cap = 1u64;
        while cap < count {
            cap = cap.saturating_mul(RADIX);
            w += 1;
            if cap >= count {
                break;
            }
        }
        w.max(1)
    }

    pub fn is_header(&self) -> bool {
        self.digits.iter().all(|&d| d == 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_roundtrip() {
        for v in 0..RADIX {
            let c = digit_char(v).unwrap();
            assert_eq!(digit_value(c), Some(v));
        }
    }

    #[test]
    fn parse_render_roundtrip() {
        for s in ["a", "b", "z", "A", "Z", "aa", "ba", "zZ", "ZZ"] {
            let o = Order::parse(s).unwrap();
            assert_eq!(o.render(), s);
        }
    }

    #[test]
    fn value_matches_expectation() {
        assert_eq!(Order::parse("a").unwrap().value(), 0);
        assert_eq!(Order::parse("b").unwrap().value(), 1);
        assert_eq!(Order::parse("z").unwrap().value(), 25);
        assert_eq!(Order::parse("A").unwrap().value(), 26);
        assert_eq!(Order::parse("Z").unwrap().value(), 51);
        assert_eq!(Order::parse("aa").unwrap().value(), 0);
        assert_eq!(Order::parse("ab").unwrap().value(), 1);
        assert_eq!(Order::parse("ba").unwrap().value(), 52);
    }

    #[test]
    fn header_fifty_two_files() {
        // 52 member files + 1 header = 53 positions, needs 2-digit width.
        let w = Order::min_width_for(53);
        assert_eq!(w, 2);
        assert_eq!(Order::header(w).render(), "aa");
        assert_eq!(Order::from_value(52, w).unwrap().render(), "ba");
    }

    #[test]
    fn invalid_digit_rejected() {
        assert!(Order::parse("0").is_err());
        assert!(Order::parse("").is_err());
    }
}
