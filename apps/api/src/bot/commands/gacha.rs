use crate::traq::TraqClient;
use anyhow::Result;
use rand::prelude::IndexedRandom;

#[derive(Clone, Copy)]
pub(super) enum Match {
    Prefix,
    Contains,
}

impl Match {
    fn accepts(self, stamp_name: &str, query: &str) -> bool {
        let name = stamp_name.to_ascii_lowercase();
        match self {
            Self::Prefix => name.starts_with(query),
            Self::Contains => name.contains(query),
        }
    }
}

pub(super) async fn draw(client: &TraqClient, search: &str, mode: Match) -> Result<String> {
    let query = search.to_ascii_lowercase();
    let candidates = client
        .stamps()
        .await?
        .into_iter()
        .filter(|stamp| mode.accepts(&stamp.name, &query))
        .map(|stamp| stamp.name)
        .collect::<Vec<_>>();

    Ok(draw_reply(&candidates).unwrap_or_else(|| match mode {
        Match::Prefix => format!("{search} のスタンプが見つかりませんでした"),
        Match::Contains => format!("{search} を含むスタンプが見つかりませんでした"),
    }))
}

pub(super) fn draw_reply(candidates: &[String]) -> Option<String> {
    candidates
        .choose(&mut rand::rng())
        .map(|name| format!(":{name}.ex-large:"))
}

#[cfg(test)]
mod tests {
    use super::Match;

    #[test]
    fn distinguishes_prefix_from_substring_search() {
        assert!(Match::Prefix.accepts("Lachite_smile", "lachite"));
        assert!(!Match::Prefix.accepts("old_lachite", "lachite"));
        assert!(Match::Contains.accepts("old_Lachite", "lachite"));
        assert!(!Match::Contains.accepts("old_lachite", "missing"));
    }
}
