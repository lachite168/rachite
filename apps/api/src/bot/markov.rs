mod generate;

use super::commands;
use crate::{
    text::{Dictionary, Quotes, extract_text},
    traq::{Destination, TraqClient},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::{collections::HashSet, sync::Arc};
use tokio::{sync::mpsc, task::spawn_blocking};
use uuid::Uuid;

pub(super) const MAX_ORDER: usize = 4;

pub(super) struct Job {
    pub(super) destination: Destination,
    pub(super) message_id: Uuid,
    pub(super) created_at: DateTime<Utc>,
    pub(super) username: String,
    pub(super) order: usize,
    pub(super) here: bool,
}

pub(super) async fn run(
    client: &TraqClient,
    dictionary: &Arc<Dictionary>,
    mention: &str,
    mut queue: mpsc::Receiver<Job>,
) {
    while let Some(job) = queue.recv().await {
        let reply = match execute(client, dictionary, mention, &job).await {
            Ok(reply) => reply,
            Err(error) => {
                tracing::warn!(%error, message_id = %job.message_id, "Markov job failed");
                "マルコフ生成の処理中にエラーが発生しました。".into()
            }
        };
        if let Err(error) = client.post_message(job.destination, &reply, false).await {
            tracing::warn!(%error, message_id = %job.message_id, "Could not send Markov reply");
        }
    }
}

async fn execute(
    client: &TraqClient,
    dictionary: &Arc<Dictionary>,
    mention: &str,
    job: &Job,
) -> Result<String> {
    let Some(user_id) = client.user_id(&job.username).await? else {
        return Ok(format!(
            "ユーザー `{}` が見つかりませんでした。",
            job.username
        ));
    };
    let channel = job.here.then(|| job.destination.channel_id());
    let (history, scope) = match (job.here, job.destination) {
        (true, Destination::Direct { user_id, .. }) => (
            client.direct_history(user_id, job.created_at).await?,
            "このDM",
        ),
        _ => (
            client
                .search_history(user_id, channel, job.created_at)
                .await?,
            if job.here {
                "このチャンネル"
            } else {
                "公開チャンネル"
            },
        ),
    };
    let mention = mention.to_owned();
    let dictionary = Arc::clone(dictionary);
    let order = job.order;
    let message_id = job.message_id;
    let result = spawn_blocking(move || {
        let mut seen = HashSet::new();
        let corpus: Vec<_> = history
            .into_iter()
            .filter(|post| {
                post.id != message_id
                    && post.user_id == user_id
                    && channel.is_none_or(|id| id == post.channel_id)
                    && seen.insert(post.id)
                    && !is_command(&mention, &post.content)
            })
            .map(|post| extract_text(&post.content, Quotes::Omit))
            .filter(|text| !text.is_empty())
            .collect();
        let has_material = !corpus.is_empty();
        generate::generate(&dictionary, &corpus, order, &mut rand::rng())
            .map(|text| (has_material, text))
    })
    .await??;
    Ok(match result {
        (_, Some(text)) => format_reply(&job.username, order, &text),
        (false, None) => format!(
            "{scope}に `{}` の生成に使える発言がありません。",
            job.username
        ),
        (true, None) => {
            "この階数では新しい文を作れませんでした。`n` を下げて試してください。".into()
        }
    })
}

fn is_command(mention: &str, source: &str) -> bool {
    if commands::arguments(mention, source).is_some() {
        return true;
    }
    // In stored Markdown, traQ may represent the leading mention as a JSON embedding.
    let Some(json) = source
        .trim_start()
        .strip_prefix('!')
        .filter(|s| s.starts_with('{'))
    else {
        return false;
    };
    let mut embeddings = serde_json::Deserializer::from_str(json).into_iter::<EmbeddedMention>();
    let Some(Ok(EmbeddedMention::User { raw })) = embeddings.next() else {
        return false;
    };
    let rest = &json[embeddings.byte_offset()..];
    raw.eq_ignore_ascii_case(mention) && (rest.is_empty() || rest.starts_with(char::is_whitespace))
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EmbeddedMention {
    User { raw: String },
}

fn format_reply(username: &str, order: usize, text: &str) -> String {
    let fence_length = text.split(|ch| ch != '`').map(str::len).max().unwrap_or(0) + 1;
    let fence = "`".repeat(fence_length.max(3));
    format!("マルコフ生成 · `{username}` · {order}階\n{fence}\n{text}\n{fence}")
}

#[cfg(test)]
mod tests {
    use super::{format_reply, is_command};

    #[test]
    fn skips_leading_bot_mentions_in_history() {
        let mention = "@BOT_lachite";
        let embedded =
            r#"!{"type":"user","id":"00000000-0000-0000-0000-000000000001","raw":"@bot_LACHITE"}"#;
        assert!(is_command(mention, "@BOT_lachite ping"));
        assert!(is_command(mention, &format!(" {embedded} markov alice")));
        assert!(!is_command(mention, &format!("{embedded}さん")));
        assert!(!is_command(mention, "hello @BOT_lachite"));
    }

    #[test]
    fn formats_generated_text_as_literal_markdown() {
        assert_eq!(
            format_reply("alice", 2, "例 ``` Markdown"),
            "マルコフ生成 · `alice` · 2階\n````\n例 ``` Markdown\n````"
        );
    }
}
