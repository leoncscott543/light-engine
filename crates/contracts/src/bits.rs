//! Exact value representations, including bits a collector did not observe.
//!
//! Bytes are canonical little endian: byte 0 contains bits 0..7. Display order
//! is most significant bit first. These values describe a captured payload;
//! converting an API scalar to bits does not establish register residency.

use std::fmt;

/// Bounded to keep one future vector/register payload small.
pub const MAX_CAPTURE_BITS: u16 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitPatternError {
    InvalidWidth(u16),
    ByteCount { expected: usize, actual: usize },
    MaskCount { expected: usize, actual: usize },
    NonzeroPadding,
}

impl fmt::Display for BitPatternError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidWidth(width) => {
                write!(f, "capture width {width} is outside 1..={MAX_CAPTURE_BITS}")
            }
            Self::ByteCount { expected, actual } => {
                write!(f, "expected {expected} bytes, got {actual}")
            }
            Self::MaskCount { expected, actual } => {
                write!(f, "expected {expected} mask bytes, got {actual}")
            }
            Self::NonzeroPadding => f.write_str("bits outside capture width must be zero"),
        }
    }
}

impl std::error::Error for BitPatternError {}

/// Lossless captured bits with an equally wide validity mask. Unknown bits
/// render as `?`; their stored values are canonicalized to zero, never evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitPattern {
    width: u16,
    le_bytes: Vec<u8>,
    known_mask: Vec<u8>,
}

impl BitPattern {
    pub fn from_le_bytes(
        width: u16,
        mut bytes: Vec<u8>,
        known_mask: Vec<u8>,
    ) -> Result<Self, BitPatternError> {
        if width == 0 || width > MAX_CAPTURE_BITS {
            return Err(BitPatternError::InvalidWidth(width));
        }
        let expected = usize::from(width).div_ceil(8);
        if bytes.len() != expected {
            return Err(BitPatternError::ByteCount {
                expected,
                actual: bytes.len(),
            });
        }
        if known_mask.len() != expected {
            return Err(BitPatternError::MaskCount {
                expected,
                actual: known_mask.len(),
            });
        }
        let remainder = width % 8;
        if remainder != 0 {
            let padding = !((1u8 << remainder) - 1);
            if (bytes[expected - 1] | known_mask[expected - 1]) & padding != 0 {
                return Err(BitPatternError::NonzeroPadding);
            }
        }
        for (byte, mask) in bytes.iter_mut().zip(&known_mask) {
            *byte &= mask;
        }
        Ok(Self {
            width,
            le_bytes: bytes,
            known_mask,
        })
    }

    pub fn from_u64(bits: u64) -> Self {
        Self {
            width: 64,
            le_bytes: bits.to_le_bytes().to_vec(),
            known_mask: vec![0xff; 8],
        }
    }

    /// Observes the API value's binary64 representation, not a CPU register.
    pub fn from_f64(value: f64) -> Self {
        Self::from_u64(value.to_bits())
    }

    pub fn width(&self) -> u16 {
        self.width
    }
    pub fn le_bytes(&self) -> &[u8] {
        &self.le_bytes
    }
    pub fn known_mask(&self) -> &[u8] {
        &self.known_mask
    }

    /// `None` means unobserved or outside the payload.
    pub fn bit(&self, index: u16) -> Option<bool> {
        if index >= self.width {
            return None;
        }
        let (byte, mask) = (usize::from(index / 8), 1 << (index % 8));
        (self.known_mask[byte] & mask != 0).then(|| self.le_bytes[byte] & mask != 0)
    }

    pub fn known_bits(&self) -> u32 {
        self.known_mask.iter().map(|byte| byte.count_ones()).sum()
    }

    pub fn binary(&self) -> String {
        (0..self.width)
            .rev()
            .map(|index| match self.bit(index) {
                Some(true) => '1',
                Some(false) => '0',
                None => '?',
            })
            .collect()
    }

    /// A hex digit requires every bit inside the payload's nibble to be known.
    /// A nibble containing an unobserved bit is `?`.
    pub fn hex(&self) -> String {
        (0..self.width.div_ceil(4))
            .rev()
            .map(|nibble| {
                let mut value = 0u32;
                for offset in 0..4 {
                    let bit = nibble * 4 + offset;
                    if bit >= self.width {
                        continue;
                    }
                    match self.bit(bit) {
                        Some(true) => value |= 1 << offset,
                        Some(false) => {}
                        None => return '?',
                    }
                }
                char::from_digit(value, 16).expect("nibble")
            })
            .collect()
    }

    /// XOR only where both observations are known. Width changes cannot be
    /// compared. The caller must also establish matching sample/field identity.
    pub fn changes_since(&self, previous: &Self) -> Option<Self> {
        if self.width != previous.width {
            return None;
        }
        let known_mask: Vec<_> = self
            .known_mask
            .iter()
            .zip(&previous.known_mask)
            .map(|(a, b)| a & b)
            .collect();
        let le_bytes = self
            .le_bytes
            .iter()
            .zip(&previous.le_bytes)
            .zip(&known_mask)
            .map(|((a, b), mask)| (a ^ b) & mask)
            .collect();
        Some(Self {
            width: self.width,
            le_bytes,
            known_mask,
        })
    }
}

/// Binary64 decoding without floating point arithmetic, preserving signed
/// zero, subnormals and every NaN payload exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binary64(pub u64);

impl Binary64 {
    pub fn from_value(value: f64) -> Self {
        Self(value.to_bits())
    }
    pub fn sign(self) -> u8 {
        (self.0 >> 63) as u8
    }
    pub fn exponent(self) -> u16 {
        ((self.0 >> 52) & 0x7ff) as u16
    }
    pub fn fraction(self) -> u64 {
        self.0 & ((1u64 << 52) - 1)
    }
    pub fn class(self) -> &'static str {
        match (self.exponent(), self.fraction()) {
            (0, 0) => "zero",
            (0, _) => "subnormal",
            (0x7ff, 0) => "infinity",
            (0x7ff, _) => "NaN",
            _ => "normal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_binary64_special_values_and_byte_order() {
        for raw in [
            0,
            1 << 63,
            1,
            0x0010_0000_0000_0000,
            0x3ff0_0000_0000_0000,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x7ff8_1234_5678_9abc,
            0x7ff0_1234_5678_9abc,
        ] {
            let value = f64::from_bits(raw);
            assert_eq!(Binary64::from_value(value).0, raw);
            let bits = BitPattern::from_f64(value);
            assert_eq!(bits.hex(), format!("{raw:016x}"));
            assert_eq!(bits.binary(), format!("{raw:064b}"));
            assert_eq!(bits.le_bytes(), raw.to_le_bytes());
            assert_eq!(bits.known_bits(), 64);
        }
        assert_eq!(Binary64(1 << 63).sign(), 1);
        assert_eq!(Binary64(1).class(), "subnormal");
        assert_eq!(
            Binary64(0x7ff8_1234_5678_9abc).fraction(),
            0x8_1234_5678_9abc
        );
        assert_eq!(Binary64(0x7ff8_1234_5678_9abc).class(), "NaN");
    }

    #[test]
    fn partial_observations_never_promote_unknown_bits() {
        let before = BitPattern::from_le_bytes(8, vec![0xff], vec![0x0f]).unwrap();
        let after = BitPattern::from_le_bytes(8, vec![0x05], vec![0xff]).unwrap();
        assert_eq!(before.le_bytes(), &[0x0f]);
        assert_eq!(before.binary(), "????1111");
        assert_eq!(before.hex(), "?f");
        assert_eq!(after.changes_since(&before).unwrap().binary(), "????1010");
        assert_eq!(before.bit(8), None);
        assert!(before.changes_since(&BitPattern::from_u64(0)).is_none());
    }

    #[test]
    fn capture_width_lengths_and_padding_are_checked() {
        for width in [0, MAX_CAPTURE_BITS + 1, u16::MAX] {
            assert!(matches!(
                BitPattern::from_le_bytes(width, vec![], vec![]),
                Err(BitPatternError::InvalidWidth(_))
            ));
        }
        assert!(matches!(
            BitPattern::from_le_bytes(9, vec![0], vec![0]),
            Err(BitPatternError::ByteCount { .. })
        ));
        assert!(matches!(
            BitPattern::from_le_bytes(9, vec![0, 0], vec![0]),
            Err(BitPatternError::MaskCount { .. })
        ));
        assert_eq!(
            BitPattern::from_le_bytes(9, vec![0, 2], vec![0xff, 1]),
            Err(BitPatternError::NonzeroPadding)
        );
        assert_eq!(
            BitPattern::from_le_bytes(9, vec![0xff, 1], vec![0xff, 1])
                .unwrap()
                .binary(),
            "111111111"
        );
        let width = MAX_CAPTURE_BITS;
        assert_eq!(
            BitPattern::from_le_bytes(width, vec![0; 512], vec![0; 512])
                .unwrap()
                .known_bits(),
            0
        );
    }
}
