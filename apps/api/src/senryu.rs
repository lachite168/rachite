mod detect;
mod store;

pub(crate) use detect::detect_senryu;
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
