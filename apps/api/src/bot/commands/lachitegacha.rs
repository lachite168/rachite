use super::{Command, gacha};
use crate::traq::TraqClient;
use anyhow::Result;

pub(super) const HELP: Command =
    Command::new("lachitegacha", "lachite で始まるスタンプから抽選します。");

pub(super) async fn execute(client: &TraqClient) -> Result<String> {
    gacha::draw(client, "lachite", gacha::Match::Prefix).await
}
