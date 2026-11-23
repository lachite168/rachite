mod detect;
mod store;

pub(crate) use detect::{detect_mogamigawa, detect_senryu, split_into_phrases};
pub(crate) use store::MogamigawaCursor;
pub use store::{Senryu, SenryuCursor, SenryuStore};

pub(crate) fn tokenizer() -> anyhow::Result<lindera::Tokenizer> {
    use lindera::{DictionaryConfig, DictionaryKind, Mode, Tokenizer, TokenizerConfig};

    Ok(Tokenizer::from_config(TokenizerConfig {
        dictionary: DictionaryConfig {
            kind: Some(DictionaryKind::IPADIC),
            path: None,
        },
        user_dictionary: None,
        mode: Mode::Normal,
    })?)
}
