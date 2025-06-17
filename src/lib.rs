use ecow::{EcoString, EcoVec};

pub mod error;
pub mod parse;
pub mod value;

pub use error::{Error, ErrorKind};
pub use value::Number;
