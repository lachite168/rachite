use crate::text::Dictionary;
use anyhow::Result;
use unicode_normalization::UnicodeNormalization;

pub(super) struct Word<'a> {
    pub surface: &'a str,
    pub reading: Reading,
    pub joins_previous: bool,
    pub joins_next: bool,
}

pub(super) enum Reading {
    Kana(String),
    Symbol,
    Unknown,
}

pub(super) fn words<'a>(dictionary: &Dictionary, text: &'a str) -> Result<Vec<Word<'a>>> {
    let tokens = dictionary.segment(text)?;
    let mut words = Vec::with_capacity(tokens.len());
    for mut token in tokens {
        let surface = &text[token.byte_start..token.byte_end];
        if surface.trim().is_empty() {
            continue;
        }

        let part_of_speech = token.get("part_of_speech").unwrap_or_default();
        let joins_next = matches!(part_of_speech, "接頭辞" | "接頭詞");
        let attaches = matches!(
            part_of_speech,
            "助詞" | "助動詞" | "接尾辞" | "記号" | "補助記号"
        );
        let attaches = attaches
            || matches!(
                token.get("part_of_speech_subcategory_1"),
                Some("接尾" | "非自立" | "非自立可能")
            );

        let reading = if surface.chars().any(char::is_alphanumeric) {
            token
                .get("reading")
                .and_then(kana)
                .or_else(|| kana(surface))
                .map_or(Reading::Unknown, Reading::Kana)
        } else {
            Reading::Symbol
        };
        let joins_previous = attaches || matches!(reading, Reading::Symbol);
        words.push(Word {
            surface,
            reading,
            joins_previous,
            joins_next,
        });
    }
    Ok(words)
}

fn kana(text: &str) -> Option<String> {
    let mut reading = String::new();
    for character in text.nfkc() {
        match character {
            'ぁ'..='ゖ' | 'ゝ'..='ゞ' | 'ァ'..='ヺ' | 'ー'..='ヾ' | 'ㇰ'..='ㇿ' => {
                reading.push(character);
            }
            '・' | '\u{3099}' | '\u{309a}' => {}
            _ => return None,
        }
    }
    (!reading.is_empty()).then_some(reading)
}
