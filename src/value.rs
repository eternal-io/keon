use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, Copy)]
pub enum Number {
    Int(i64),
    UInt(u64),
    Float(f64),
}

//------------------------------------------------------------------------------
impl Number {
    #[inline]
    pub const fn saturating_to_i64(self) -> i64 {
        match self {
            Self::Int(i) => i,
            Self::UInt(u) => match u >= i64::MAX as u64 {
                true => i64::MAX,
                false => u as i64,
            },
            Self::Float(f) => f.clamp(i64::MIN as f64, i64::MAX as f64) as i64,
        }
    }

    #[inline]
    pub const fn saturating_to_u64(self) -> u64 {
        match self {
            Self::Int(i) => match i >= 0 {
                true => i as u64,
                false => 0,
            },
            Self::UInt(u) => u,
            Self::Float(f) => f.clamp(u64::MIN as f64, u64::MAX as f64) as u64,
        }
    }

    #[inline]
    pub const fn to_f64_lossy(self) -> f64 {
        match self {
            Self::Int(i) => i as f64,
            Self::UInt(u) => u as f64,
            Self::Float(f) => f,
        }
    }
}

macro_rules! impl_from_T_for_number {
    ( $from:ty => $into:ident ) => {
        impl From<$from> for Number {
            #[inline]
            fn from(value: $from) -> Self {
                Self::$into(value as _)
            }
        }
    };
}

impl_from_T_for_number!(    i8 => Int );
impl_from_T_for_number!(   i16 => Int );
impl_from_T_for_number!(   i32 => Int );
impl_from_T_for_number!(   i64 => Int );
impl_from_T_for_number!( isize => Int );
impl_from_T_for_number!(    u8 => UInt );
impl_from_T_for_number!(   u16 => UInt );
impl_from_T_for_number!(   u32 => UInt );
impl_from_T_for_number!(   u64 => UInt );
impl_from_T_for_number!( usize => UInt );
impl_from_T_for_number!(   f32 => Float );
impl_from_T_for_number!(   f64 => Float );

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::UInt(a), Self::UInt(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.is_nan() && b.is_nan() || a == b,
            _ => false,
        }
    }
}

impl Eq for Number {}

/// In order to be able to use [`Number`] as a map key,
/// `NaN` is greater than any other number and equal to themselves.
#[allow(clippy::non_canonical_partial_ord_impl)]
impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(match self {
            Number::Int(i) => match other {
                Number::Int(j) => i.cmp(j),
                Number::UInt(_) => Ordering::Less,
                Number::Float(_) => Ordering::Less,
            },
            Number::UInt(u) => match other {
                Number::Int(_) => Ordering::Greater,
                Number::UInt(v) => u.cmp(v),
                Number::Float(_) => Ordering::Less,
            },
            Number::Float(f) => match other {
                Number::Int(_) => Ordering::Greater,
                Number::UInt(_) => Ordering::Greater,
                Number::Float(g) => match (f.is_nan(), g.is_nan()) {
                    (false, false) => f.partial_cmp(g).unwrap(),
                    (false, true) => Ordering::Less,
                    (true, false) => Ordering::Greater,
                    (true, true) => Ordering::Equal,
                },
            },
        })
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Number::Int(i) => state.write_i64(*i),
            Number::UInt(u) => state.write_u64(*u),
            Number::Float(f) => state.write_u64(f.to_bits()),
        }
    }
}
