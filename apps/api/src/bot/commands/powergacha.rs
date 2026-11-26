use super::Command;
use num_bigint::BigUint;

pub(super) const HELP: Command = Command::new(
    "powergacha [n]",
    "2 のべき乗を返します。 n を指定するとそのべき乗になります。（n は 1 ~ 64）",
);

pub(super) fn execute(text: Option<&str>) -> String {
    let base = match text.map_or(Ok(2), str::parse::<u128>) {
        Err(e) => return format!("整数のパースに失敗しました。\n```{e}```"),
        Ok(n @ 1..=64) => BigUint::from(n),
        Ok(_) => return "n は 1 ~ 64 で指定してください".into(),
    };
    let power = rand::random_range(1..1024);

    format!("{}", base.pow(power))
}
