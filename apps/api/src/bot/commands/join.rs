use super::Command;
use crate::traq::TraqClient;
use anyhow::Result;
use uuid::Uuid;

pub(super) const HELP: Command = Command::new("join", "このチャンネルに Bot が参加します。");

pub(super) async fn execute(client: &TraqClient, channel_id: Uuid) -> Result<String> {
    client.set_channel_membership(channel_id, true).await?;
    Ok("このチャンネルは参加しています".into())
}
