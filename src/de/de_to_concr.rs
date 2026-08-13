use super::{
    error::{BoxedKind, ErrorKind, ResultKind},
    raise,
    source::*,
    PrivateMethod,
};
use crate::value::Scalar;
use core::ops::{Deref, DerefMut};
use either::Either;
use serde::{
    de::{
        value::StrDeserializer, DeserializeSeed, EnumAccess, Error, MapAccess, SeqAccess, Unexpected, VariantAccess,
        Visitor,
    },
    Deserialize, Deserializer,
};

impl<'de, T: Deserialize<'de>> super::Deserialize<'de> for T {
    #[expect(private_interfaces)]
    fn deserialize_with<R: Read<'de>>(der: &mut super::Deserializer<R>, _: PrivateMethod) -> ResultKind<Self> {
        T::deserialize(DeserializerWrapper(der))
    }
}

//==================================================================================================

macro_rules! recursion_guard {
    ($self:ident, $expr:expr) => {{
        $self.enter_nesting()?;
        let res = $expr;
        $self.exit_nesting();
        res
    }};
}

macro_rules! deserialize_integer {
    ($method:ident, $visiting:ident, $parsing:ident) => {
        fn $method<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
            self.eat_ws()?;
            if self.try_byte()? {
                visitor.visit_u8(self.parse_byte()?)
            } else {
                visitor.$visiting(self.$parsing()?)
            }
        }
    };
}

macro_rules! deserialize_float {
    ($method:ident, $visiting:ident, $parsing:ident) => {
        fn $method<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
            self.eat_ws()?;
            visitor.$visiting(self.$parsing()?)
        }
    };
}

/// Avoid direct use of [`super::Deserializer`] as [`serde::Deserializer`].
struct DeserializerWrapper<'a, R>(&'a mut super::Deserializer<R>);

impl<R> DeserializerWrapper<'_, R> {
    fn reborrow<'a>(&'a mut self) -> DeserializerWrapper<'a, R> {
        DeserializerWrapper(self.0)
    }
}

impl<R> Deref for DeserializerWrapper<'_, R> {
    type Target = super::Deserializer<R>;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<R> DerefMut for DeserializerWrapper<'_, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'de, R: Read<'de>> Deserializer<'de> for DeserializerWrapper<'_, R> {
    type Error = BoxedKind;

    fn deserialize_any<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let range_component = 'non_range: {
            let res = match self.0.src.begin(&mut self.0.buf)? {
                Indicator::Unit => visitor.visit_unit(),
                Indicator::Bool(b) => visitor.visit_bool(b),
                Indicator::Char(ch) => break 'non_range Either::Left(Scalar::Char(ch)),
                Indicator::Byte(byte) => break 'non_range Either::Left(Scalar::Number(byte.into())),
                Indicator::Number(kind) => break 'non_range Either::Left(Scalar::Number(self.parse_number(kind)?)),
                Indicator::String(kind) => match self.0.src.parse_string(kind, &mut self.0.buf)? {
                    Either::Left(ref_de) => visitor.visit_borrowed_str(ref_de),
                    Either::Right(ref_tmp) => visitor.visit_str(ref_tmp),
                },
                Indicator::Bytes(kind) => match self.0.src.parse_bytes(kind, &mut self.0.buf)? {
                    Either::Left(ref_de) => visitor.visit_borrowed_bytes(ref_de),
                    Either::Right(ref_tmp) => visitor.visit_bytes(ref_tmp),
                },
                Indicator::Initiator(init) => match init {
                    Initiator::Maybe => {
                        if !self.adjacent_to_delim()? {
                            recursion_guard!(self, visitor.visit_some(self.reborrow()))
                        } else {
                            visitor.visit_none()
                        }
                    }
                    Initiator::Array => recursion_guard!(self, {
                        let val = visitor.visit_seq(self.reborrow())?;
                        self.end_array()?;
                        Ok(val)
                    }),
                    Initiator::Tuple => recursion_guard!(self, {
                        let val = visitor.visit_seq(self.reborrow())?;
                        self.end_tuple()?;
                        Ok(val)
                    }),
                    Initiator::Map => recursion_guard!(self, {
                        let val = visitor.visit_map(MapLikeAccessor::<R, false> { der: self.reborrow() })?;
                        self.end_map_like()?;
                        Ok(val)
                    }),
                    Initiator::DotDot => {
                        break 'non_range Either::Right(RangeSeparator::DotDot);
                    }
                    Initiator::DotDotEq => {
                        break 'non_range Either::Right(RangeSeparator::DotDotEq);
                    }
                },
                Indicator::Identifier(_name) => match self.nominal_body_initiator()? {
                    Some(init) => match init {
                        NominalBodyInitiator::Tuple => recursion_guard!(self, {
                            let val = visitor.visit_seq(self.reborrow())?;
                            self.end_tuple()?;
                            Ok(val)
                        }),
                        NominalBodyInitiator::Struct => recursion_guard!(self, {
                            let val = visitor.visit_map(MapLikeAccessor::<R, true> { der: self.reborrow() })?;
                            self.end_map_like()?;
                            Ok(val)
                        }),
                    },
                    None => {
                        self.adjacent_to_delim_expected(ErrorKind::InvalidNominalBody)?;
                        visitor.visit_unit()
                    }
                },
                Indicator::ExplicitNewtype(_name) => {
                    recursion_guard!(self, visitor.visit_newtype_struct(self.reborrow()))
                }
                Indicator::ExplicitVariant(_name, variant) => visitor.visit_enum(EnumAccessor {
                    // SAFETY: Upheld by EnumAccessor's invariant.
                    variant: unsafe { core::mem::transmute::<&str, &str>(variant.as_str()) },
                    der: self,
                }),
            };
            return res;
        };

        let mut start = None;
        let mut end = None;
        match range_component {
            Either::Left(scalar) => {
                if let Some(sep) = self.range_separator()? {
                    if self.adjacent_to_scalar()? {
                        start = Some(scalar);
                        end = Some(self.deserialize_scalar()?);
                    } else if let RangeSeparator::DotDot = sep {
                        start = Some(scalar);
                    } else {
                        return raise(ErrorKind::UnexpectedRangeDotDotEq);
                    }
                }
            }
            Either::Right(sep) => {
                if self.adjacent_to_scalar()? {
                    end = Some(self.deserialize_scalar()?);
                } else if let RangeSeparator::DotDot = sep {
                    return visitor.visit_unit(); // RangeFull
                } else {
                    return raise(ErrorKind::UnexpectedRangeDotDotEq);
                }
            }
        }

        visitor.visit_map(AnyRangeAccessor { start, end })
    }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_any(visitor)
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.eat_ws()?;
        self.parse_unit()?;
        visitor.visit_unit()
    }

    fn deserialize_bool<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.eat_ws()?;
        visitor.visit_bool(self.parse_bool()?)
    }

    deserialize_integer!(deserialize_i8, visit_i8, parse_i8);
    deserialize_integer!(deserialize_i16, visit_i16, parse_i16);
    deserialize_integer!(deserialize_i32, visit_i32, parse_i32);
    deserialize_integer!(deserialize_i64, visit_i64, parse_i64);
    deserialize_integer!(deserialize_i128, visit_i128, parse_i128);
    deserialize_integer!(deserialize_u8, visit_u8, parse_u8);
    deserialize_integer!(deserialize_u16, visit_u16, parse_u16);
    deserialize_integer!(deserialize_u32, visit_u32, parse_u32);
    deserialize_integer!(deserialize_u64, visit_u64, parse_u64);
    deserialize_integer!(deserialize_u128, visit_u128, parse_u128);
    deserialize_float!(deserialize_f32, visit_f32, parse_f32);
    deserialize_float!(deserialize_f64, visit_f64, parse_f64);

    fn deserialize_char<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_char()?;
        visitor.visit_char(self.parse_char()?)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_str(visitor)
    }
    fn deserialize_str<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let kind = self.begin_string()?;
        match self.0.src.parse_string(kind, &mut self.0.buf)? {
            Either::Left(ref_de) => visitor.visit_borrowed_str(ref_de),
            Either::Right(ref_tmp) => visitor.visit_str(ref_tmp),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_bytes(visitor)
    }
    fn deserialize_bytes<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        let kind = self.begin_bytes()?;
        match self.0.src.parse_bytes(kind, &mut self.0.buf)? {
            Either::Left(ref_de) => visitor.visit_borrowed_bytes(ref_de),
            Either::Right(ref_tmp) => visitor.visit_bytes(ref_tmp),
        }
    }

    //------------------------------------------------------------------------------

    fn deserialize_option<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.begin_maybe()?;
        if !self.adjacent_to_delim()? {
            recursion_guard!(self, Ok(visitor.visit_some(self.reborrow())?))
        } else {
            visitor.visit_none()
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        recursion_guard!(self, {
            self.begin_array()?;
            let val = visitor.visit_seq(self.reborrow())?;
            self.end_array()?;
            Ok(val)
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(mut self, len: usize, visitor: V) -> ResultKind<V::Value> {
        let _ = len;
        recursion_guard!(self, {
            self.begin_tuple()?;
            let val = visitor.visit_seq(self.reborrow())?;
            self.end_tuple()?;
            Ok(val)
        })
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_map_like::<V, false>(visitor)
    }

    //------------------------------------------------------------------------------

    fn deserialize_unit_struct<V: Visitor<'de>>(mut self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        if name == "RangeFull" && self.range_to(false)? {
        } else {
            self.eat_ws()?;
            self.deserialize_struct_name(name, &visitor)?;
        }
        self.adjacent_to_delim_expected(ErrorKind::UnexpectedUnitBody)?;
        visitor.visit_unit()
    }
    fn deserialize_newtype_struct<V: Visitor<'de>>(mut self, name: &'static str, visitor: V) -> ResultKind<V::Value> {
        self.eat_ws()?;
        self.deserialize_newtype_name(name, &visitor)?;
        recursion_guard!(self, Ok(visitor.visit_newtype_struct(self.reborrow())?))
    }
    fn deserialize_tuple_struct<V: Visitor<'de>>(
        mut self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> ResultKind<V::Value> {
        self.eat_ws()?;
        self.deserialize_struct_name(name, &visitor)?;
        self.deserialize_tuple(len, visitor)
    }
    fn deserialize_struct<V: Visitor<'de>>(
        mut self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        match name {
            "RangeTo" if matches!(fields, ["end"]) => {
                if self.range_to(false)? {
                    return visitor.visit_map(RangeToAccessor { der: Some(self) });
                }
            }
            "RangeToInclusive" if matches!(fields, ["end"]) => {
                if self.range_to(true)? {
                    return visitor.visit_map(RangeToAccessor { der: Some(self) });
                }
            }
            "RangeFrom" if matches!(fields, ["start"]) => {
                if self.adjacent_to_scalar()? {
                    return visitor.visit_map(RangeFromAccessor { der: Some(self) });
                }
            }
            "Range" if matches!(fields, ["start", "end"]) => {
                if self.adjacent_to_scalar()? {
                    return visitor.visit_map(RangeAccessor::<R, false>::new(self));
                }
            }
            "RangeInclusive" if matches!(fields, ["start", "end"]) => {
                if self.adjacent_to_scalar()? {
                    return visitor.visit_map(RangeAccessor::<R, true>::new(self));
                }
            }
            _ => self.eat_ws()?,
        }
        self.deserialize_struct_name(name, &visitor)?;
        self.deserialize_map_like::<V, true>(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        mut self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> ResultKind<V::Value> {
        let _ = variants;
        self.eat_ws()?;
        let (name_parsed, variant) = self.0.src.parse_variant_name(&mut self.0.buf)?;
        if let Some(name_parsed) = name_parsed {
            if name_parsed.as_str() != name {
                return Err(Error::invalid_type(Unexpected::Other(name_parsed), &visitor));
            }
        }
        visitor.visit_enum(EnumAccessor {
            // SAFETY: Upheld by EnumAccessor's invariant.
            variant: unsafe { core::mem::transmute::<&str, &str>(variant.as_str()) },
            der: self,
        })
    }

    // NOTE: This method is called when deserialize struct field name.
    fn deserialize_identifier<V: Visitor<'de>>(mut self, visitor: V) -> ResultKind<V::Value> {
        self.eat_ws()?;
        visitor.visit_str(&self.0.src.parse_identifier(&mut self.0.buf)?)
    }
}

impl<'de, R: Read<'de>> DeserializerWrapper<'_, R> {
    #[inline]
    fn deserialize_newtype_name<V: Visitor<'de>>(&mut self, name: &'static str, visitor: &V) -> ResultKind {
        if let Some(name_parsed) = self.0.src.parse_newtype_name(&mut self.0.buf)? {
            if name_parsed.as_str() != name {
                return Err(Error::invalid_type(Unexpected::Other(name_parsed), visitor));
            }
        }
        Ok(())
    }

    #[inline]
    fn deserialize_struct_name<V: Visitor<'de>>(&mut self, name: &'static str, visitor: &V) -> ResultKind {
        if let Some(name_parsed) = self.0.src.parse_struct_name(&mut self.0.buf)? {
            if name_parsed.as_str() != name {
                return Err(Error::invalid_type(Unexpected::Other(name_parsed), visitor));
            }
        }
        Ok(())
    }

    #[inline]
    fn deserialize_map_like<V: Visitor<'de>, const STRUCT_MODE: bool>(mut self, visitor: V) -> ResultKind<V::Value> {
        recursion_guard!(self, {
            self.begin_map_like()?;
            let val = visitor.visit_map(MapLikeAccessor::<R, STRUCT_MODE> { der: self.reborrow() })?;
            self.end_map_like()?;
            Ok(val)
        })
    }
}

//==================================================================================================

impl<'de, R: Read<'de>> SeqAccess<'de> for DeserializerWrapper<'_, R> {
    type Error = BoxedKind;

    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> ResultKind<Option<T::Value>> {
        if self.adjacent_to_delim()? {
            return Ok(None);
        }
        let val = seed.deserialize(self.reborrow())?;
        self.delim(Delimiter::Comma)?;
        Ok(Some(val))
    }
}

struct MapLikeAccessor<'a, R, const STRUCT_MODE: bool> {
    der: DeserializerWrapper<'a, R>,
}

impl<'de, R: Read<'de>, const STRUCT_MODE: bool> MapAccess<'de> for MapLikeAccessor<'_, R, STRUCT_MODE> {
    type Error = BoxedKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        let MapLikeAccessor { der } = self;
        if der.adjacent_to_delim()? {
            return Ok(None);
        }
        let val = seed.deserialize(der.reborrow())?;
        if STRUCT_MODE {
            der.delim_expected(Delimiter::Colon, ErrorKind::ExpectedColon)?;
        } else {
            der.delim_expected(Delimiter::FatArrow, ErrorKind::ExpectedFatArrow)?;
        }
        Ok(Some(val))
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        let MapLikeAccessor { der } = self;
        let val = seed.deserialize(der.reborrow())?;
        der.delim(Delimiter::Comma)?;
        Ok(val)
    }
}

//------------------------------------------------------------------------------

struct RangeToAccessor<'a, R> {
    der: Option<DeserializerWrapper<'a, R>>,
}

impl<'de, R: Read<'de>> MapAccess<'de> for RangeToAccessor<'_, R> {
    type Error = BoxedKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        if let Some(der) = self.der.as_mut() {
            der.adjacent_to_scalar_expected()?;
            Ok(Some(seed.deserialize(StrDeserializer::<BoxedKind>::new("end"))?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        let Some(der) = self.der.take() else {
            panic!("contract violation")
        };
        seed.deserialize(der)
    }
}

struct RangeFromAccessor<'a, R> {
    der: Option<DeserializerWrapper<'a, R>>,
}

impl<'de, R: Read<'de>> MapAccess<'de> for RangeFromAccessor<'_, R> {
    type Error = BoxedKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        if self.der.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<BoxedKind>::new("start"))?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        let Some(mut der) = self.der.take() else {
            panic!("contract violation")
        };
        let num = seed.deserialize(der.reborrow())?;
        der.end_range_from()?;
        Ok(num)
    }
}

struct RangeAccessor<'a, R, const INCLUSIVE: bool> {
    der: Option<DeserializerWrapper<'a, R>>,
    end: bool,
}

impl<'a, R, const INCLUSIVE: bool> RangeAccessor<'a, R, INCLUSIVE> {
    fn new(der: DeserializerWrapper<'a, R>) -> Self {
        Self {
            der: Some(der),
            end: false,
        }
    }
}

impl<'de, R: Read<'de>, const INCLUSIVE: bool> MapAccess<'de> for RangeAccessor<'_, R, INCLUSIVE> {
    type Error = BoxedKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        if let Some(der) = self.der.as_mut() {
            if !self.end {
                Ok(Some(seed.deserialize(StrDeserializer::<BoxedKind>::new("start"))?))
            } else {
                der.range_to(INCLUSIVE)?.then_some(()).ok_or(match INCLUSIVE {
                    true => ErrorKind::ExpectedRangeDotDotEq,
                    false => ErrorKind::ExpectedRangeDotDot,
                })?;
                der.adjacent_to_scalar_expected()?;
                Ok(Some(seed.deserialize(StrDeserializer::<BoxedKind>::new("end"))?))
            }
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        let der = if !self.end {
            self.end = true;
            self.der.as_mut().unwrap()
        } else {
            &mut self.der.take().expect("contract violation")
        };
        seed.deserialize(der.reborrow())
    }
}

struct AnyRangeAccessor {
    start: Option<Scalar>,
    end: Option<Scalar>,
}

impl<'de> MapAccess<'de> for AnyRangeAccessor {
    type Error = BoxedKind;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> ResultKind<Option<K::Value>> {
        if self.start.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<BoxedKind>::new("start"))?))
        } else if self.end.is_some() {
            Ok(Some(seed.deserialize(StrDeserializer::<BoxedKind>::new("end"))?))
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> ResultKind<V::Value> {
        if let Some(scalar) = self.start.take() {
            seed.deserialize(scalar.deserializer())
        } else if let Some(scalar) = self.end.take() {
            seed.deserialize(scalar.deserializer())
        } else {
            panic!("contract violation")
        }
    }
}

//------------------------------------------------------------------------------

/// # Safety invariants
///
/// `variant` must not be used after access through `der` begins. The variant
/// reference may borrow from the deserializer's internal buffer, and mutable
/// access through `der` must not overlap with that borrow.
///
/// Implementations of [`EnumAccess`] must consume `variant` before returning
/// or exposing `der` for further use.
struct EnumAccessor<'variant, 'a, R> {
    variant: &'variant str,
    der: DeserializerWrapper<'a, R>,
}

impl<'a, 'de, R: Read<'de>> EnumAccess<'de> for EnumAccessor<'_, 'a, R> {
    type Error = BoxedKind;
    type Variant = DeserializerWrapper<'a, R>;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> ResultKind<(V::Value, Self::Variant)> {
        let EnumAccessor { variant, der } = self;
        Ok((seed.deserialize(StrDeserializer::<BoxedKind>::new(variant))?, der))
    }
}

impl<'de, R: Read<'de>> VariantAccess<'de> for DeserializerWrapper<'_, R> {
    type Error = BoxedKind;

    fn unit_variant(mut self) -> ResultKind<()> {
        self.adjacent_to_delim_expected(ErrorKind::UnexpectedUnitBody)?;
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(mut self, seed: T) -> ResultKind<T::Value> {
        recursion_guard!(self, {
            self.begin_tuple()?;
            let val = seed.deserialize(self.reborrow())?;
            self.delim(Delimiter::Comma)?;
            self.end_tuple()?;
            Ok(val)
        })
    }

    fn tuple_variant<V: Visitor<'de>>(self, len: usize, visitor: V) -> ResultKind<V::Value> {
        self.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V: Visitor<'de>>(self, fields: &'static [&'static str], visitor: V) -> ResultKind<V::Value> {
        let _ = fields;
        self.deserialize_map_like::<V, true>(visitor)
    }
}
