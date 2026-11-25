use super::Command;
use crate::traq::TraqClient;
use anyhow::Result;
use uuid::Uuid;

pub(super) const HELP: Command = Command::new("leave", "このチャンネルから Bot が退出します。");

pub(super) async fn execute(client: &TraqClient, channel_id: Uuid) -> Result<String> {
    client.set_channel_membership(channel_id, false).await?;
    Ok("このチャンネルは退出しています".into())
}
