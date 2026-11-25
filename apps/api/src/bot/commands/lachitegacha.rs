use super::{Command, gacha};
use crate::traq::TraqClient;
use anyhow::Result;

pub(super) const HELP: Command = Command::new(
    "lachitegacha",
    "lachite で始まるスタンプからランダムに 1 つ出力します。",
);

pub(super) async fn execute(client: &TraqClient) -> Result<String> {
    gacha::draw(client, "lachite", gacha::Match::Prefix).await
}
