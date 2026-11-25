mod markdown;

use anyhow::Result;
use lindera::{dictionary::load_dictionary, mode::Mode, segmenter::Segmenter, token::Token};
use std::borrow::Cow;

pub(crate) use markdown::{Quotes, extract_text};

pub(crate) struct Dictionary(Segmenter);

impl Dictionary {
    pub(crate) fn sudachidict() -> Result<Self> {
        let dictionary = load_dictionary("embedded://sudachidict")?;
        Ok(Self(Segmenter::new(Mode::Normal, dictionary, None)))
    }

    pub(crate) fn segment<'a>(&'a self, text: &'a str) -> Result<Vec<Token<'a>>> {
        // SudachiDict registers Latin spellings in lowercase. Folding ASCII keeps
        // byte offsets valid for callers that need the original spelling.
        let lookup = if text.bytes().any(|byte| byte.is_ascii_uppercase()) {
            Cow::Owned(text.to_ascii_lowercase())
        } else {
            Cow::Borrowed(text)
        };
        Ok(self.0.segment(lookup)?)
    }
}
