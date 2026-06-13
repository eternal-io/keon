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
    ser4::{stringify, stringify_pretty, SerializeConfig, Serializer},
    value::Value,
};

trait Sealed {}

mod format {
    use core::num::NonZeroU8;
    use lexical_core::{
        NumberFormatBuilder, ParseFloatOptions, ParseFloatOptionsBuilder, ParseIntegerOptions,
        ParseIntegerOptionsBuilder, WriteFloatOptions, WriteFloatOptionsBuilder, WriteIntegerOptions,
        WriteIntegerOptionsBuilder,
    };

    // [1456789ABFGHIJKMN-_]
    pub(crate) const NUMBER_FORMAT: u128 = NumberFormatBuilder::new()
        .required_digits(true)
        .required_fraction_digits(false)
        .digit_separator(NonZeroU8::new(b'_'))
        .internal_digit_separator(true)
        .trailing_digit_separator(true)
        .consecutive_digit_separator(true)
        .special_digit_separator(false)
        .no_positive_mantissa_sign(true)
        .case_sensitive_base_prefix(true)
        .case_sensitive_special(true)
        .build();

    pub(crate) const NUMBER_FORMAT_HEX: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(16)
        .base_prefix(NonZeroU8::new(b'x'))
        .build();
    pub(crate) const NUMBER_FORMAT_OCT: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(8)
        .base_prefix(NonZeroU8::new(b'o'))
        .build();
    pub(crate) const NUMBER_FORMAT_BIN: u128 = NumberFormatBuilder::rebuild(NUMBER_FORMAT)
        .mantissa_radix(2)
        .base_prefix(NonZeroU8::new(b'b'))
        .build();

    pub(crate) const PARSE_INTEGER_OPTS: ParseIntegerOptions = ParseIntegerOptionsBuilder::new()
        .no_multi_digit(false)
        .build_unchecked();

    pub(crate) const PARSE_FLOAT_OPTS: ParseFloatOptions = ParseFloatOptionsBuilder::new()
        .lossy(false)
        .exponent(b'e')
        .decimal_point(b'.')
        .nan_string(Some(b"NaN"))
        .inf_string(Some(b"inf"))
        .infinity_string(None)
        .build_unchecked();

    pub(crate) const WRITE_INTEGER_OPTS: WriteIntegerOptions = WriteIntegerOptionsBuilder::new().build_unchecked();

    pub(crate) const WRITE_FLOAT_OPTS: WriteFloatOptions = WriteFloatOptionsBuilder::new().build_unchecked();
}
