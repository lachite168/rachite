use super::{Command, command_catalog};

pub(super) const HELP: Command = Command::new("help", "コマンドの使い方を表示します。");

pub(super) fn execute(mention: &str) -> String {
    command_catalog()
        .iter()
        .map(|command| {
            let invocation = format!("{mention} {}", command.usage);
            format!("`{}`  {}", invocation.trim_end(), command.description)
        })
        .collect::<Vec<_>>()
        .join("\n")
}
