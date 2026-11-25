use super::{Bot, Command, usage};
use crate::{
    bot::markov::{Job, MAX_ORDER},
    traq::{Destination, Message},
};
use std::str::SplitWhitespace;
use tokio::sync::mpsc::error::TrySendError;

pub(super) const HELP: Command = Command::new(
    "markov <username> [n] [--here]",
    "指定した人の発言から文章を生成します。任意で階数 n を 1〜4 の整数で指定できます。（既定は 2 ）、--here を末尾につけるとそのチャンネルの発言のみから生成します。",
);

pub(super) async fn submit(
    bot: &Bot,
    destination: Destination,
    message: &Message,
    args: SplitWhitespace<'_>,
) {
    let reply = if let Some((username, order, here)) = parse(args) {
        match bot.markov_jobs.try_send(Job {
            destination,
            message_id: message.id,
            created_at: message.created_at,
            username: username.into(),
            order,
            here,
        }) {
            Ok(()) => return,
            Err(TrySendError::Full(_)) => {
                "マルコフ生成が混雑中です。しばらくしてからお試しください。".into()
            }
            Err(TrySendError::Closed(_)) => "マルコフ生成の処理中にエラーが発生しました。".into(),
        }
    } else {
        usage(&bot.mention, HELP.usage)
    };
    if let Err(error) = bot.client.post_message(destination, &reply, false).await {
        tracing::warn!(%error, message_id = %message.id, "Could not send Markov reply");
    }
}

fn parse(mut args: SplitWhitespace<'_>) -> Option<(&str, usize, bool)> {
    let name = args.next()?;
    let username = name.strip_prefix('@').unwrap_or(name);
    let valid_characters = |s: &str| {
        s.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    };
    let valid_name = match username.split_once('#') {
        Some((prefix, suffix)) => {
            prefix.eq_ignore_ascii_case("Webhook") && suffix.len() == 22 && valid_characters(suffix)
        }
        None => (1..=32).contains(&username.len()) && valid_characters(username),
    };
    if !valid_name || name == "--here" {
        return None;
    }
    let (order, here) = match args.next() {
        None => (2, false),
        Some("--here") => (2, true),
        Some(value) => (
            value.parse().ok()?,
            match args.next() {
                None => false,
                Some("--here") => true,
                Some(_) => return None,
            },
        ),
    };
    ((1..=MAX_ORDER).contains(&order) && args.next().is_none()).then_some((username, order, here))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parses_order_and_trailing_scope() {
        for (input, expected) in [
            ("alice", Some(("alice", 2, false))),
            ("@Alice --here", Some(("Alice", 2, true))),
            ("alice 1", Some(("alice", 1, false))),
            ("alice 4 --here", Some(("alice", 4, true))),
            (
                "@webHOOK#abcdefghijklmnopqrstuv",
                Some(("webHOOK#abcdefghijklmnopqrstuv", 2, false)),
            ),
            ("@-alice", Some(("-alice", 2, false))),
            ("", None),
            ("--here", None),
            ("alice 0", None),
            ("alice 5", None),
            ("alice 2 --unknown", None),
            ("alice --here 3", None),
            ("alice 2 --here extra", None),
        ] {
            assert_eq!(parse(input.split_whitespace()), expected, "{input}");
        }
    }
}
