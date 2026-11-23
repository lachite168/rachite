mod gacha;
mod help;
mod join;
mod lachitegacha;
mod leave;
mod mention;
mod mogamigawa;
mod n3gacha;
mod ping;
mod senryu;
mod stampbomb;
mod stampgacha;
#[cfg(test)]
mod tests;

use super::Bot;
use crate::traq::Message;
use anyhow::Result;
use serde::Serialize;
use std::str::SplitWhitespace;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct Command {
    pub usage: &'static str,
    pub description: &'static str,
}

impl Command {
    const fn new(usage: &'static str, description: &'static str) -> Self {
        Self { usage, description }
    }
}

const COMMANDS: &[Command] = &[
    mention::HELP,
    ping::HELP,
    join::HELP,
    leave::HELP,
    lachitegacha::HELP,
    n3gacha::HELP,
    stampgacha::HELP,
    stampbomb::HELP,
    senryu::STATUS,
    senryu::ON,
    senryu::OFF,
    mogamigawa::STATUS,
    mogamigawa::ON,
    mogamigawa::OFF,
    help::HELP,
];

pub fn command_catalog() -> &'static [Command] {
    COMMANDS
}

pub(super) fn arguments<'a>(mention: &str, text: &'a str) -> Option<SplitWhitespace<'a>> {
    let mut words = text.split_whitespace();
    words.next()?.eq_ignore_ascii_case(mention).then_some(words)
}

fn usage(mention: &str, command: &str) -> String {
    format!("使い方: `{mention} {command}`")
}

impl Bot {
    pub(super) async fn dispatch(
        &self,
        message: &Message,
        mut args: SplitWhitespace<'_>,
    ) -> Result<()> {
        let reply = match args.next().map(str::to_ascii_lowercase).as_deref() {
            None => mention::execute(message),
            Some("ping") => ping::execute(message, args.next()),
            Some("join") => join::execute(&self.client, message.channel_id).await?,
            Some("leave") => leave::execute(&self.client, message.channel_id).await?,
            Some("lachitegacha") => lachitegacha::execute(&self.client).await?,
            Some("n3gacha") => n3gacha::execute(&self.client).await?,
            Some("stampgacha") => stampgacha::execute(self, args.next()).await?,
            Some("stampbomb") => stampbomb::execute(self, message, args).await?,
            Some("senryu") => senryu::execute(self, message.channel_id, args.next()).await?,
            Some("mogamigawa") => {
                mogamigawa::execute(self, message.channel_id, args.next()).await?
            }
            Some("help") => help::execute(&self.mention),
            _ => return Ok(()),
        };

        self.client.reply(message.channel_id, &reply).await
    }
}
