mod detect;
mod dictionary;
mod store;

pub(crate) use detect::detect;
pub(crate) use store::MogamigawaCursor;
pub use store::{Senryu, SenryuCursor, SenryuStore};
