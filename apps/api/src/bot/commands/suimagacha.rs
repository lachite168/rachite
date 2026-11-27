use super::{Command, gacha};
use crate::traq::TraqClient;
use anyhow::Result;

pub(super) const HELP: Command = Command::new(
    "suimagacha",
    "suima で始まるスタンプからランダムに 1 つ出力します。",
);

pub(super) async fn execute(client: &TraqClient) -> Result<String> {
    gacha::draw(client, "suima", gacha::Match::Prefix).await
}
