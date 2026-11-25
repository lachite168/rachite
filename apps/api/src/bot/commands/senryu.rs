use super::{Bot, Command, usage};
use anyhow::Result;
use uuid::Uuid;

pub(super) const STATUS: Command = Command::new(
    "senryu status",
    "このチャンネルで川柳検出が有効か確認します。",
);
pub(super) const ON: Command =
    Command::new("senryu on", "このチャンネルで川柳検出を有効にします。");
pub(super) const OFF: Command = Command::new(
    "senryu off",
    "このチャンネルで新規の川柳検出を無効にします。",
);

pub(super) async fn execute(bot: &Bot, channel_id: Uuid, action: Option<&str>) -> Result<String> {
    let enabled = match action {
        Some("status") => bot.store.is_enabled(channel_id).await?,
        Some(action @ ("on" | "off")) => {
            let enabled = action == "on";
            bot.store.set_enabled(channel_id, enabled).await?;
            enabled
        }
        _ => return Ok(usage(&bot.mention, "senryu <status|on|off>")),
    };
    let status = if enabled { "オン" } else { "オフ" };
    Ok(format!("このチャンネルの川柳検出は{status}です。"))
}
