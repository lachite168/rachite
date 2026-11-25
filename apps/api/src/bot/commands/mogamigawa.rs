use super::{Bot, Command, usage};
use anyhow::Result;
use uuid::Uuid;

pub(super) const STATUS: Command = Command::new(
    "mogamigawa status",
    "このチャンネルで最上川検出が有効か確認します。",
);
pub(super) const ON: Command = Command::new(
    "mogamigawa on",
    "このチャンネルで最上川検出を有効にします。",
);
pub(super) const OFF: Command = Command::new(
    "mogamigawa off",
    "このチャンネルで最上川検出を無効にします。",
);

pub(super) async fn execute(bot: &Bot, channel_id: Uuid, action: Option<&str>) -> Result<String> {
    let enabled = match action {
        Some("status") => bot.store.is_mogamigawa_enabled(channel_id).await?,
        Some(action @ ("on" | "off")) => {
            let enabled = action == "on";
            bot.store
                .set_mogamigawa_enabled(channel_id, enabled)
                .await?;
            enabled
        }
        _ => return Ok(usage(&bot.mention, "mogamigawa <status|on|off>")),
    };
    let status = if enabled { "オン" } else { "オフ" };
    Ok(format!("このチャンネルの最上川検出は{status}です。"))
}
