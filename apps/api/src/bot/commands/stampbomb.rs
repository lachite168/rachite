use super::{Bot, Command, usage};
use crate::{
    bot::stampbomb::{Job, MAX_PER_JOB},
    traq::{Message, TraqClient},
};
use anyhow::Result;
use serde::{Deserialize, de::IgnoredAny};
use tokio::sync::mpsc;
use uuid::Uuid;

pub(super) const HELP: Command = Command::new(
    "stampbomb <stamp> <number> <message-link>",
    "指定したメッセージにスタンプを押します。回数は1〜100,000です。",
);

pub(super) async fn execute(
    bot: &Bot,
    message: &Message,
    mut args: std::str::SplitWhitespace<'_>,
) -> Result<String> {
    let (Some(stamp), Some(number), Some(link)) = (args.next(), args.next(), args.next()) else {
        return Ok(usage(&bot.mention, HELP.usage));
    };
    let Ok(count) = number.parse::<i64>() else {
        return Ok(usage(&bot.mention, HELP.usage));
    };
    if !(1..=i64::from(MAX_PER_JOB)).contains(&count) {
        return Ok(format!(
            "回数 <number> は1〜{MAX_PER_JOB}の間で指定してください。"
        ));
    }
    let count = count as u32;
    let Some(message_id) = target_message_id(&bot.client, message, stamp, number, link) else {
        return Ok("無効なメッセージリンクです。".into());
    };

    let stamp_name = stamp.replace(':', "");
    let Some(stamp) = bot
        .client
        .stamps()
        .await?
        .into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(&stamp_name))
    else {
        return Ok(format!("スタンプ {stamp_name} が見つかりませんでした。"));
    };

    match bot.stamp_jobs.try_send(Job {
        message_id,
        stamp_id: stamp.id,
        requester_channel: message.channel_id,
        count,
    }) {
        Ok(()) => Ok(format!(
            "@{} の代わりに :{}: を {} 回連打します。\n{}",
            message.user.name,
            stamp.name,
            count,
            bot.client.message_link(message_id)
        )),
        Err(mpsc::error::TrySendError::Full(_)) => {
            Ok("スタンプ送信が混雑中です。しばらくしてからお試しください。".into())
        }
        Err(mpsc::error::TrySendError::Closed(_)) => anyhow::bail!("Stamp worker is unavailable"),
    }
}

fn target_message_id(
    client: &TraqClient,
    message: &Message,
    stamp: &str,
    number: &str,
    link: &str,
) -> Option<Uuid> {
    if let Some(id) = client.message_id(link) {
        return Some(id);
    }

    // traQ replaces message links with [引用メッセージ] in plainText.
    let text = message.text.trim_start();
    let mut rest = if let Some(json) = text.strip_prefix('!').filter(|s| s.starts_with('{')) {
        let mut mention = serde_json::Deserializer::from_str(json).into_iter::<IgnoredAny>();
        mention.next()?.ok()?;
        &json[mention.byte_offset()..]
    } else {
        text.split_once(char::is_whitespace)?.1
    };
    for expected in ["stampbomb", stamp, number] {
        let (word, remaining) = rest.trim_start().split_once(char::is_whitespace)?;
        if !word.eq_ignore_ascii_case(expected) {
            return None;
        }
        rest = remaining;
    }

    let target = rest.trim_start();
    if let Some(json) = target.strip_prefix('!').filter(|s| s.starts_with('{')) {
        let mut embedded = serde_json::Deserializer::from_str(json).into_iter::<EmbeddedLink>();
        let EmbeddedLink::Message { id } = embedded.next()?.ok()?;
        let suffix = &json[embedded.byte_offset()..];
        return (suffix.is_empty() || suffix.starts_with(char::is_whitespace)).then_some(id);
    }
    client.message_id(target.split_whitespace().next()?)
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum EmbeddedLink {
    Message { id: Uuid },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bot::commands::arguments, traq::Event};
    use serde_json::json;

    #[test]
    fn uses_the_message_link_at_the_command_argument_position() {
        let id = Uuid::now_v7();
        let client =
            TraqClient::new("https://q.trap.jp".parse().unwrap(), "test-token", id).unwrap();
        let mention = r#"!{"type": "user", "raw": "@bot_lachite", "id": "00000000-0000-0000-0000-000000000001"}"#;
        let url = client.message_link(id);
        let quote = format!(r#"!{{"type": "message", "raw": "{url}", "id": "{id}"}}"#);
        let other_url = client.message_link(Uuid::now_v7());
        let foreign_url = format!("https://example.com/messages/{id}");
        for (target, plain, expected) in [
            (url.clone(), "[引用メッセージ]", Some(id)),
            (format!("<{url}>"), "<[引用メッセージ]>", Some(id)),
            (
                format!("{quote}\n{other_url}"),
                "[引用メッセージ]",
                Some(id),
            ),
            (format!("[引用メッセージ]\n{url}"), "[引用メッセージ]", None),
            (format!("{foreign_url}\n{url}"), foreign_url.as_str(), None),
            (String::new(), url.as_str(), Some(id)),
        ] {
            let event: Event = serde_json::from_value(json!({
                "type": "MESSAGE_CREATED",
                "body": {"message": {
                    "id": Uuid::now_v7(),
                    "channelId": Uuid::now_v7(),
                    "user": {"id": Uuid::from_u128(1), "name": "alice", "bot": false},
                    "text": format!("{mention}  STAMPBOMB\tw 1234\n{target}"),
                    "plainText": format!("@bot_lachite  STAMPBOMB\tw 1234\n{plain}"),
                    "createdAt": "2026-11-23T01:02:03Z"
                }}
            }))
            .unwrap();
            let Event::MessageCreated { body } = event else {
                panic!("expected a message creation event");
            };
            let message = body.message;
            let mut args = arguments("@BOT_lachite", &message.plain_text).unwrap();
            assert_eq!(args.next(), Some("STAMPBOMB"));
            assert_eq!(
                target_message_id(
                    &client,
                    &message,
                    args.next().unwrap(),
                    args.next().unwrap(),
                    args.next().unwrap(),
                ),
                expected,
                "{target}"
            );
        }
    }
}
