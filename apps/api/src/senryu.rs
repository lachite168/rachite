mod detect;
mod dictionary;
mod markdown;
mod store;

pub(crate) use detect::detect;
pub(crate) use dictionary::VerseDictionary;
pub(crate) use store::MogamigawaCursor;
pub use store::{Senryu, SenryuCursor, SenryuStore};
