pub mod de;
#[cfg(feature = "alloc")]
pub mod de4;
pub mod ser4;
pub mod value;

extern crate alloc;

pub use crate::{
    de::{
        error::{Error, ErrorKind},
        parse, parse_limited, parse_many, parse_many_limited,
    },
    ser4::{stringify, stringify_pretty, Serializer},
    value::Value,
};

struct PrivateMethod;

mod format {
    use core::num::NonZeroU8;
    use lexical_core::{
        parse_integer_options, write_integer_options, NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder,
        ParseIntegerOptions, WriteFloatOptions, WriteFloatOptionsBuilder, WriteIntegerOptions,
    };

    pub(crate) const NUMBER_FORMAT: u128 = NumberFormatBuilder::new()
        .case_sensitive_base_prefix(true)
        .case_sensitive_special(true)
        .no_positive_mantissa_sign(true)
        .required_integer_digits(true)
        .internal_digit_separator(true)
        .trailing_digit_separator(true)
        .consecutive_digit_separator(true)
        .digit_separator(NonZeroU8::new(b'_'))
        .build_strict();

    pub(crate) const NUMBER_FORMAT_HEX_NO_PREFIX: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(16)
        .build_strict();
    pub(crate) const NUMBER_FORMAT_HEX: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(16)
        .base_prefix(NonZeroU8::new(b'x'))
        .build_strict();
    pub(crate) const NUMBER_FORMAT_OCT: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(8)
        .base_prefix(NonZeroU8::new(b'o'))
        .build_strict();
    pub(crate) const NUMBER_FORMAT_BIN: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(2)
        .base_prefix(NonZeroU8::new(b'b'))
        .build_strict();

    pub(crate) const PARSE_INTEGER_OPTS: ParseIntegerOptions = parse_integer_options::STANDARD;

    pub(crate) const PARSE_FLOAT_OPTS: ParseFloatOptions = ParseFloatOptionsBuilder::new()
        .lossy(false)
        .exponent(b'e')
        .decimal_point(b'.')
        .nan_string(Some(b"NaN"))
        .inf_string(None)
        .infinity_string(Some(b"inf"))
        .build_strict();

    pub(crate) const WRITE_INTEGER_OPTS: WriteIntegerOptions = write_integer_options::STANDARD;

    pub(crate) const WRITE_FLOAT_OPTS: WriteFloatOptions = WriteFloatOptionsBuilder::new()
        .exponent(b'e')
        .decimal_point(b'.')
        .nan_string(Some(b"NaN"))
        .inf_string(None)
        .infinity_string(Some(b"inf"))
        .build_strict();
}
