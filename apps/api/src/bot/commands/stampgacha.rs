mod regexp;

use super::{Bot, Command, gacha, usage};
use anyhow::{Context, Result};
use regexp::MatchError;
use std::time::Duration;

const REGEXP_BUDGET: Duration = Duration::from_millis(250);
const INVALID_REGEXP: &str = "無効な正規表現です。パターンとフラグを確認してください";

pub(super) const HELP: Command = Command::new(
    "stampgacha <search>",
    "指定した文字列を名前に含むスタンプから抽選します。",
);

pub(super) const REGEX_HELP: Command = Command::new(
    "stampgacha /pattern/[flags]",
    "ECMAScript の正規表現に一致する名前から抽選します。既定では大文字・小文字を区別しません。区別する例は /(?-i:^N3)/ です。",
);

pub(super) async fn execute(bot: &Bot, search: Option<&str>) -> Result<String> {
    let Some(search) = search else {
        return Ok(usage(&bot.mention, HELP.usage));
    };
    let Some((pattern, flags)) = regexp_literal(search) else {
        return gacha::draw(&bot.client, search, gacha::Match::Contains).await;
    };
    if pattern.is_empty() {
        return Ok(INVALID_REGEXP.into());
    }

    let pattern = pattern.to_owned();
    let mut flags = flags.to_owned();
    if !flags.contains('i') {
        flags.push('i');
    }
    let names = bot
        .client
        .stamps()
        .await?
        .into_iter()
        .map(|stamp| stamp.name)
        .collect();
    let matches = tokio::task::spawn_blocking(move || {
        regexp::matching_names(&pattern, &flags, names, REGEXP_BUDGET)
    })
    .await
    .context("stampgacha regular expression task failed")?;

    match matches {
        Ok(names) => Ok(gacha::draw_reply(&names)
            .unwrap_or_else(|| "正規表現に一致するスタンプが見つかりませんでした".into())),
        Err(MatchError::InvalidPattern) => Ok(INVALID_REGEXP.into()),
        Err(MatchError::TimedOut) => Ok("正規表現の照合が時間切れになりました".into()),
        Err(error) => Err(error.into()),
    }
}

fn regexp_literal(search: &str) -> Option<(&str, &str)> {
    let expression = search.strip_prefix('/')?;
    let mut escaped = false;
    let mut in_class = false;
    let mut class_slash = None;

    for (index, ch) in expression.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' => in_class = true,
            ']' => {
                in_class = false;
                class_slash = None;
            }
            '/' if !in_class => return Some((&expression[..index], &expression[index + 1..])),
            '/' => class_slash = Some(index),
            _ => {}
        }
    }

    // Treat a possible closing slash in an unterminated class as a syntax error,
    // so a typo such as /[/ gets a RegExp error instead of a literal search.
    let index = class_slash?;
    Some((&expression[..index], &expression[index + 1..]))
}
