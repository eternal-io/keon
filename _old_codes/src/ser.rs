use super::*;
use data_encoding::{BASE32_NOPAD, BASE64URL_NOPAD, HEXUPPER_PERMISSIVE};
use lexical_core::BUFFER_SIZE;
use serde::{
    ser::{
        SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple, SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Serialize,
};
use std::io::Write;

/// Conveniently serialize `value` to a String in the minimal way.
pub fn to_string<T: ?Sized + Serialize>(value: &T) -> Result<String> {
    let mut buf = Vec::new();
    to_writer(&mut buf, value)?;
    unsafe { Ok(String::from_utf8_unchecked(buf)) }
}

/// Conveniently serialize `value` to a String in a pretty way.
pub fn to_string_pretty<T: ?Sized + Serialize>(value: &T) -> Result<String> {
    let mut buf = Vec::new();
    to_writer_pretty(&mut buf, value)?;
    unsafe { Ok(String::from_utf8_unchecked(buf)) }
}

/// Conveniently serialize `value` into `writer` in the minimal way.
pub fn to_writer<W: Write, T: ?Sized + Serialize>(writer: W, value: &T) -> Result<()> {
    let mut ser = Serializer::new(writer, SerializeConfig::minimal());
    value.serialize(&mut ser)
}

/// Conveniently serialize `value` into `writer` in a pretty way.
pub fn to_writer_pretty<W: Write, T: ?Sized + Serialize>(writer: W, value: &T) -> Result<()> {
    let mut ser = Serializer::new(writer, SerializeConfig::comfort());
    value.serialize(&mut ser)
}

//==================================================================================================

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct SerializeConfig {
    pub minimize_after_depth: u8,
    pub bytes_flavor: BytesFlavor,
}

impl SerializeConfig {
    pub const fn minimal() -> Self {
        Self {
            minimize_after_depth: 0,
            bytes_flavor: BytesFlavor::Base64,
        }
    }

    pub const fn comfort() -> Self {
        Self {
            minimize_after_depth: 6,
            bytes_flavor: BytesFlavor::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BytesFlavor {
    Normal,
    Base16,
    Base32,
    Base64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ObjectType {
    Tuple,
    TupleDocile,
    Seq,
    Map,
    Struct,
    Something,
    MinNewtype,
    MinNullary,
}

//==================================================================================================

/// The KEON Serializer.
///
/// Usually convenience functions [`to_string`], [`to_string_pretty`]... are enough.
pub struct Serializer<W: Write> {
    dst: W,
    dep: usize,
    cfg: SerializeConfig,
    buf: Box<[u8; BUFFER_SIZE]>,
}

impl<W: Write> Serializer<W> {
    pub fn new(writer: W, cfg: SerializeConfig) -> Self {
        Self {
            dst: writer,
            dep: 0,
            cfg,
            buf: Box::new([0; BUFFER_SIZE]),
        }
    }

    #[inline]
    fn minimize(&self) -> bool {
        self.dep >= self.cfg.minimize_after_depth as usize
    }

    #[inline]
    fn write_newline(&mut self) -> Result<()> {
        Ok(writeln!(self.dst)?)
    }
    #[inline]
    fn write_space(&mut self) -> Result<()> {
        Ok(write!(self.dst, "\x20")?)
    }
    #[inline]
    fn write_indent(&mut self) -> Result<()> {
        for _ in 0..self.dep {
            write!(self.dst, "\x20\x20\x20\x20")?;
        }
        Ok(())
    }

    #[inline]
    fn write_ident(&mut self, ident: &str) -> Result<()> {
        match ident {
            ident @ ("true" | "false" | "inf" | "NaN") => write!(self.dst, "`{}", ident)?,
            ident => write!(self.dst, "{}", ident)?,
        }
        Ok(())
    }

    #[inline]
    fn maybe_write_struct_name(&mut self, name: &str) -> Result<bool> {
        if !self.minimize() && !name.is_empty() {
            write!(self.dst, "(")?;
            self.write_ident(name)?;
            write!(self.dst, ")")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    #[inline]
    fn maybe_write_enum_name(&mut self, name: &str) -> Result<()> {
        if !self.minimize() && !name.is_empty() {
            self.write_ident(name)?;
            write!(self.dst, "::")?;
        }
        Ok(())
    }
    #[inline]
    fn maybe_write_space(&mut self) -> Result<()> {
        if !self.minimize() {
            self.write_space()?;
        }
        Ok(())
    }

    #[inline]
    fn write_i64(&mut self, v: i64) -> Result<()> {
        Ok(self.dst.write_all(lexical_core::write(v, &mut *self.buf))?)
    }
    #[inline]
    fn write_u64(&mut self, v: u64) -> Result<()> {
        Ok(self.dst.write_all(lexical_core::write(v, &mut *self.buf))?)
    }
    #[inline]
    fn write_f64(&mut self, v: f64) -> Result<()> {
        Ok(self.dst.write_all(lexical_core::write(v, &mut *self.buf))?)
    }
    #[inline] // avoids ugly and unnecessary mantissas.
    fn write_f32(&mut self, v: f32) -> Result<()> {
        Ok(self.dst.write_all(lexical_core::write(v, &mut *self.buf))?)
    }

    #[inline]
    fn write_byte_escaped(&mut self, byte: u8) -> Result<()> {
        match byte {
            b'\0' => self.dst.write_all(br"\0")?,
            b'\n' => self.dst.write_all(br"\n")?,
            b'\t' => self.dst.write_all(br"\t")?,
            b'\r' => self.dst.write_all(br"\r")?,
            b'\'' => self.dst.write_all(br"\'")?,
            b'\"' => self.dst.write_all(b"\\\"")?,
            0x20..=0x7e => self.dst.write_all(&[byte])?,
            _ => write!(self.dst, "\\x{:02x}", byte)?,
        }
        Ok(())
    }
    #[inline]
    fn write_char_escaped(&mut self, ch: char) -> Result<()> {
        match ch {
            '\0' => self.dst.write_all(br"\0")?,
            '\n' => self.dst.write_all(br"\n")?,
            '\t' => self.dst.write_all(br"\t")?,
            '\r' => self.dst.write_all(br"\r")?,
            '\'' => self.dst.write_all(br"\'")?,
            '\"' => self.dst.write_all(b"\\\"")?,
            '\x01'..='\x19' | '\x7f' => write!(self.dst, "\\x{:02x}", ch as u8)?,
            _ => write!(self.dst, "{}", ch)?,
        }
        Ok(())
    }
}

//==================================================================================================

#[doc(hidden)]
pub struct SerializerEntry<'se, W: Write> {
    ser: &'se mut Serializer<W>,
    typ: ObjectType,
    ctr: usize,
}

impl<'se, W: Write> SerializerEntry<'se, W> {
    fn enter(ser: &'se mut Serializer<W>, typ: ObjectType) -> Result<Self> {
        ser.dep += 1;

        if ser.dep > RECURSION_LIMIT {
            Error::raise(ErrorKind::ExceededRecursionLimit)?
        }

        match typ {
            ObjectType::Seq => write!(ser.dst, "[")?,
            ObjectType::Tuple | ObjectType::TupleDocile => write!(ser.dst, "(")?,
            ObjectType::Map | ObjectType::Struct => write!(ser.dst, "{{")?,
            ObjectType::Something => {
                write!(ser.dst, "?")?;
                ser.maybe_write_space()?;
            }
            ObjectType::MinNewtype | ObjectType::MinNullary => write!(ser.dst, "%")?,
        }

        Ok(Self { ser, typ, ctr: 0 })
    }

    fn leave(mut self) -> Result<()> {
        self.ser.dep -= 1;

        if !self.ser.minimize() && self.ctr != 0 {
            self.write_separator()?
        }

        match self.typ {
            ObjectType::Seq => write!(self.ser.dst, "]")?,
            ObjectType::Tuple if self.ctr == 1 => write!(self.ser.dst, ",)")?,
            ObjectType::Tuple | ObjectType::TupleDocile => write!(self.ser.dst, ")")?,
            ObjectType::Map | ObjectType::Struct => write!(self.ser.dst, "}}")?,
            ObjectType::Something | ObjectType::MinNewtype | ObjectType::MinNullary => (),
        }

        Ok(())
    }

    fn write_separator(&mut self) -> Result<()> {
        if self.ctr != 0 {
            write!(self.ser.dst, ",")?;
        }

        self.ctr += 1;

        if !self.ser.minimize() {
            self.ser.write_newline()?;
            self.ser.write_indent()?;
        }

        Ok(())
    }
}

//==================================================================================================

impl<'se, W: Write> serde::Serializer for &'se mut Serializer<W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerializerEntry<'se, W>;
    type SerializeTuple = SerializerEntry<'se, W>;
    type SerializeTupleStruct = SerializerEntry<'se, W>;
    type SerializeTupleVariant = SerializerEntry<'se, W>;
    type SerializeMap = SerializerEntry<'se, W>;
    type SerializeStruct = SerializerEntry<'se, W>;
    type SerializeStructVariant = SerializerEntry<'se, W>;

    fn serialize_unit(self) -> Result<()> {
        Ok(write!(self.dst, "()")?)
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        match v {
            true => write!(self.dst, "true")?,
            false => write!(self.dst, "false")?,
        }
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_i64(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(v as u64)
    }
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.write_u64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.write_f32(v)
    }
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.write_f64(v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        write!(self.dst, "'")?;
        self.write_char_escaped(v)?;
        write!(self.dst, "'")?;
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<()> {
        write!(self.dst, "\"")?;
        for ch in v.chars() {
            self.write_char_escaped(ch)?;
        }
        write!(self.dst, "\"")?;
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        match self.cfg.bytes_flavor {
            BytesFlavor::Normal => {
                write!(self.dst, "b\"")?;
                for byte in v {
                    self.write_byte_escaped(*byte)?;
                }
                write!(self.dst, "\"")?;
            }
            BytesFlavor::Base16 => write!(self.dst, r#"b16"{}""#, HEXUPPER_PERMISSIVE.encode(v))?,
            BytesFlavor::Base32 => write!(self.dst, r#"b32"{}""#, BASE32_NOPAD.encode(v))?,
            BytesFlavor::Base64 => write!(self.dst, r#"b64"{}""#, BASE64URL_NOPAD.encode(v))?,
        }
        Ok(())
    }

    fn serialize_none(self) -> Result<()> {
        Ok(write!(self.dst, "?")?)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<()> {
        let entry = SerializerEntry::enter(self, ObjectType::Something)?;
        value.serialize(&mut *entry.ser)?;
        entry.leave()?;

        Ok(())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        SerializerEntry::enter(self, ObjectType::Tuple)
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        SerializerEntry::enter(self, ObjectType::Seq)
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        SerializerEntry::enter(self, ObjectType::Map)
    }

    //------------------------------------------------------------------------------

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        if !self.maybe_write_struct_name(name)? {
            self.serialize_unit()?
        }

        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(self, name: &'static str, value: &T) -> Result<()> {
        let leading = self.maybe_write_struct_name(name)?;

        let entry = match !self.minimize() {
            true if leading => SerializerEntry::enter(self, ObjectType::TupleDocile)?,
            true | false => SerializerEntry::enter(self, ObjectType::MinNewtype)?,
        };
        value.serialize(&mut *entry.ser)?;
        entry.leave()?;

        Ok(())
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        let leading = self.maybe_write_struct_name(name)?;

        match len {
            0 => SerializerEntry::enter(self, ObjectType::MinNullary),
            _ => match leading {
                true => SerializerEntry::enter(self, ObjectType::TupleDocile),
                false => SerializerEntry::enter(self, ObjectType::Tuple),
            },
        }
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.maybe_write_struct_name(name)?;
        self.maybe_write_space()?;

        SerializerEntry::enter(self, ObjectType::Struct)
    }

    //------------------------------------------------------------------------------

    fn serialize_unit_variant(self, name: &'static str, _variant_index: u32, variant: &'static str) -> Result<()> {
        self.maybe_write_enum_name(name)?;
        self.write_ident(variant)?;

        Ok(())
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        self.maybe_write_enum_name(name)?;
        self.write_ident(variant)?;

        let entry = match !self.minimize() {
            true => SerializerEntry::enter(self, ObjectType::TupleDocile)?,
            false => SerializerEntry::enter(self, ObjectType::MinNewtype)?,
        };
        value.serialize(&mut *entry.ser)?;
        entry.leave()?;

        Ok(())
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.maybe_write_enum_name(name)?;
        self.write_ident(variant)?;

        match len {
            0 => SerializerEntry::enter(self, ObjectType::MinNullary),
            _ => SerializerEntry::enter(self, ObjectType::TupleDocile),
        }
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.maybe_write_enum_name(name)?;
        self.write_ident(variant)?;
        self.maybe_write_space()?;

        SerializerEntry::enter(self, ObjectType::Struct)
    }
}

//==================================================================================================

impl<W: Write> SerializeSeq for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.write_separator()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}

impl<W: Write> SerializeTuple for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.write_separator()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}

impl<W: Write> SerializeTupleStruct for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.write_separator()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}

impl<W: Write> SerializeTupleVariant for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.write_separator()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}

impl<W: Write> SerializeMap for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<()> {
        self.write_separator()?;
        key.serialize(&mut *self.ser)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<()> {
        self.ser.maybe_write_space()?;
        write!(self.ser.dst, "=>")?;
        self.ser.maybe_write_space()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}

impl<W: Write> SerializeStruct for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.write_separator()?;
        self.ser.write_ident(key)?;
        write!(self.ser.dst, ":")?;
        self.ser.maybe_write_space()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}

impl<W: Write> SerializeStructVariant for SerializerEntry<'_, W> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, key: &'static str, value: &T) -> Result<()> {
        self.write_separator()?;
        self.ser.write_ident(key)?;
        write!(self.ser.dst, ":")?;
        self.ser.maybe_write_space()?;
        value.serialize(&mut *self.ser)
    }
    fn end(self) -> Result<()> {
        self.leave()
    }
}
