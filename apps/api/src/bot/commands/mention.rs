use super::Command;
use crate::traq::Message;

pub(super) const HELP: Command = Command::new("", "メンションを返します。");

pub(super) fn execute(message: &Message) -> String {
    format!("@{}", message.user.name)
}
