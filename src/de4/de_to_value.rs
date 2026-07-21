use super::{error::*, source::*};
use crate::value::Value2;

impl<'de> super::Deserialize<'de> for Value2 {
    fn deserialize_with<R: Source<'de>>(der: &mut super::Deserializer<R>) -> ResultKind<Self> {
        match der.src.begin()? {
            Indicator::Unit => todo!(),
            Indicator::Bool(_) => todo!(),
            Indicator::Char(_) => todo!(),
            Indicator::Bytes(bytes_kind) => todo!(),
            Indicator::Number(number_kind) => todo!(),
            Indicator::String(string_kind) => todo!(),
            Indicator::PunctStart(punct_start) => todo!(),
            Indicator::NominalPath(nominal_path_ref) => todo!(),
        }
    }
}
