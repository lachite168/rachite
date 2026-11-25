use crate::text::Dictionary;
use anyhow::Result;
use rand::{Rng, prelude::IndexedRandom};
use std::collections::HashMap;

const ATTEMPTS: usize = 16;
const MAX_TOKENS: usize = 100;
const MAX_CHARACTERS: usize = 500;

#[derive(Clone, Copy)]
struct Word<'a> {
    surface: &'a str,
    space_before: bool,
}

type Transitions<'a> = HashMap<Vec<Option<&'a str>>, Vec<Option<Word<'a>>>>;

pub(super) fn generate(
    dictionary: &Dictionary,
    corpus: &[String],
    order: usize,
    rng: &mut impl Rng,
) -> Result<Option<String>> {
    let mut transitions = Transitions::new();
    for text in corpus {
        let mut state = vec![None; order];
        let mut previous_end = 0;
        for token in dictionary.segment(text)? {
            let surface = &text[token.byte_start..token.byte_end];
            if surface.trim().is_empty() {
                continue;
            }
            let word = Word {
                surface,
                space_before: text[previous_end..token.byte_start].contains(' '),
            };
            transitions
                .entry(state.clone())
                .or_default()
                .push(Some(word));
            state.rotate_left(1);
            state[order - 1] = Some(surface);
            previous_end = token.byte_end;
        }
        if previous_end > 0 {
            transitions.entry(state).or_default().push(None);
        }
    }

    Ok((0..ATTEMPTS).find_map(|_| {
        let text = sample(&transitions, order, rng)?;
        (!text.is_empty() && !corpus.contains(&text)).then_some(text)
    }))
}

fn sample(transitions: &Transitions<'_>, order: usize, rng: &mut impl Rng) -> Option<String> {
    let mut state = vec![None; order];
    let mut text = String::new();
    let mut characters = 0;
    for _ in 0..=MAX_TOKENS {
        let Some(word) = *transitions.get(&state)?.choose(rng)? else {
            return Some(text);
        };
        if word.space_before && !text.is_empty() {
            text.push(' ');
            characters += 1;
        }
        characters += word.surface.chars().count();
        if characters > MAX_CHARACTERS {
            return None;
        }
        text.push_str(word.surface);
        state.rotate_left(1);
        state[order - 1] = Some(word.surface);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    #[test]
    fn order_controls_recombination_and_original_posts_are_rejected() {
        let dictionary = Dictionary::sudachidict().unwrap();
        let corpus = ["Red marker today".into(), "Blue marker tomorrow".into()];
        let mut rng = StdRng::seed_from_u64(1);
        let generated = generate(&dictionary, &corpus, 1, &mut rng)
            .unwrap()
            .unwrap();
        assert!(["Red marker tomorrow", "Blue marker today"].contains(&generated.as_str()));
        assert_eq!(generate(&dictionary, &corpus, 2, &mut rng).unwrap(), None);
    }
}
