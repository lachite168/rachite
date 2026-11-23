use super::Command;
use crate::traq::Message;
use chrono::Utc;

pub(super) const HELP: Command = Command::new(
    "ping [文字列]",
    "pong と、投稿から処理までの経過時間を返します。",
);

pub(super) fn execute(message: &Message, text: Option<&str>) -> String {
    let elapsed = (Utc::now() - message.created_at).num_milliseconds();
    format!(
        "@{} pong! {} ({elapsed} ms)",
        message.user.name,
        text.unwrap_or("")
    )
}
