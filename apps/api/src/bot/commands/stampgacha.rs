use super::{Bot, Command, gacha, usage};
use anyhow::Result;

pub(super) const HELP: Command = Command::new(
    "stampgacha <search>",
    "指定した文字列を名前に含むスタンプから抽選します。",
);

pub(super) async fn execute(bot: &Bot, search: Option<&str>) -> Result<String> {
    match search {
        Some(search) => gacha::draw(&bot.client, search, gacha::Match::Contains).await,
        None => Ok(usage(&bot.mention, HELP.usage)),
    }
}
