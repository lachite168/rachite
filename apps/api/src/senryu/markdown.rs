use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};
use regex::{Captures, Regex};
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::OnceLock;
use uuid::Uuid;

const EMBEDDING: char = '\u{fffc}';

#[derive(Deserialize)]
struct Embedding {
    #[serde(rename = "type")]
    kind: String,
    #[serde(rename = "id")]
    _id: Uuid,
}

fn mark_embeddings(text: &str) -> Cow<'_, str> {
    static JSON: OnceLock<Regex> = OnceLock::new();
    let regex =
        JSON.get_or_init(|| Regex::new(r#"!\{(?:[^{}"\r\n]|"(?:\\.|[^"\\\r\n])*")*\}"#).unwrap());
    // A non-whitespace marker preserves Markdown structure (e.g. list indentation).
    regex.replace_all(text, |captures: &Captures<'_>| {
        let source = &captures[0];
        match serde_json::from_str::<Embedding>(&source[1..]) {
            Ok(embedding)
                if matches!(
                    embedding.kind.as_str(),
                    "channel" | "file" | "group" | "message" | "user"
                ) =>
            {
                EMBEDDING.to_string()
            }
            _ => source.to_owned(),
        }
    })
}

pub(super) fn extract_text(source: &str) -> String {
    let source = mark_embeddings(source);
    let options = Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_MATH;
    let mut text = String::new();
    let mut ignored_depth = 0;
    for event in Parser::new_ext(&source, options) {
        if ignored_depth > 0 {
            match event {
                Event::Start(_) => ignored_depth += 1,
                Event::End(_) => ignored_depth -= 1,
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(
                Tag::CodeBlock(_)
                | Tag::Image { .. }
                | Tag::Link {
                    link_type: LinkType::Autolink | LinkType::Email,
                    ..
                },
            ) => {
                ignored_depth = 1;
                text.push(' ');
            }
            Event::Text(value) | Event::Html(value) | Event::InlineHtml(value) => {
                text.push_str(&value);
            }
            Event::SoftBreak | Event::HardBreak | Event::Rule | Event::DisplayMath(_) => {
                text.push('\n');
            }
            Event::End(
                TagEnd::Paragraph | TagEnd::Heading(_) | TagEnd::Item | TagEnd::TableCell,
            ) => text.push('\n'),
            Event::Code(_) | Event::InlineMath(_) => text.push(' '),
            _ => {}
        }
    }

    static OMITTED: OnceLock<Regex> = OnceLock::new();
    let omitted = OMITTED.get_or_init(|| {
        Regex::new(concat!(
            r"(?s:!!\S(?:.*?\S)??!!)|", // traQ spoilers
            r"(?i:(?:(?:https?|ftp)://|www\.)[^\s<>\x{fffc}]+)|",
            r":[a-zA-Z0-9_/@+-]+:|@[a-zA-Z0-9_-]{1,32}|[0-9#*]\x{fe0f}?\x{20e3}|\x{fffc}",
        ))
        .unwrap()
    });
    static HIGHLIGHT: OnceLock<Regex> = OnceLock::new();
    let highlight = HIGHLIGHT.get_or_init(|| Regex::new(r"==([^=\r\n]+)==").unwrap());
    let text = omitted.replace_all(&text, " ");
    highlight
        .replace_all(&text, "$1")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::extract_text;

    #[test]
    fn extracts_prose_across_markdown_blocks() {
        assert_eq!(
            extract_text("1. **古池や**\n2. [蛙飛びこむ](https://example.com)\n\n---\n\n> 水の音"),
            "古池や 蛙飛びこむ 水の音"
        );
        assert_eq!(extract_text("# コー**ヒー**\n\n==好き=="), "コーヒー 好き");
        assert_eq!(
            extract_text("| 古池や | 蛙飛びこむ |\n| --- | --- |\n| 水の音 | |"),
            "古池や 蛙飛びこむ 水の音"
        );
    }

    #[test]
    fn omits_non_prose_and_preserves_literal_citation_text() {
        let embedded =
            r#"!{"raw":"引用","type":"message","id":"ee764d5f-71d9-4a40-bc7b-547d8d097c91"}"#;
        assert_eq!(
            extract_text(&format!(
                "@BOT_lachite 古池や :frog: `code` $x$ !!ネタバレ!! {embedded}\n\n蛙飛びこむ https://example.com\n\n~~~rust\n水の音\n~~~\n\n水の音"
            )),
            "古池や 蛙飛びこむ 水の音"
        );
        assert_eq!(extract_text("[引用メッセージ]"), "[引用メッセージ]");
        assert_eq!(
            extract_text("古池や zzzzz 蛙飛びこむ"),
            "古池や zzzzz 蛙飛びこむ"
        );
    }
}
