use lindera::Tokenizer;
use regex::Regex;
use std::sync::OnceLock;

const IGNORE_CHARS: [char; 59] = [
    '「', '」', '『', '』', '（', '）', '〔', '〕', '［', '］', '[', ']', '{', '}', '｟', '｠',
    '〈', '〉', '《', '》', '<', '>', '【', '】', '〖', '〗', '〘', '〙', '〚', '〛', '。', '、',
    '・', '゠', '＝', '！', '？', '!', ',', '.', '?', 'ぁ', 'ぃ', 'ぅ', 'ぇ', 'ぉ', 'ゃ', 'ゅ',
    'ょ', 'ゎ', 'ァ', 'ィ', 'ゥ', 'ェ', 'ォ', 'ャ', 'ュ', 'ョ', 'ヮ',
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Phrase {
    text: String,
    mora: usize,
}

fn count_mora(text: &str) -> usize {
    let mut count = 0;
    for c in text.chars() {
        if c.is_ascii_punctuation() || c.is_whitespace() || IGNORE_CHARS.contains(&c) {
            continue;
        }
        count += 1;
    }
    count
}

fn sanitize_text(text: &str) -> String {
    static RE_CODE_BLOCK: OnceLock<Regex> = OnceLock::new();
    static RE_INLINE_CODE: OnceLock<Regex> = OnceLock::new();
    static RE_MATH_BLOCK: OnceLock<Regex> = OnceLock::new();
    static RE_MATH_INLINE: OnceLock<Regex> = OnceLock::new();
    static RE_SPOILER: OnceLock<Regex> = OnceLock::new();
    static RE_LINK: OnceLock<Regex> = OnceLock::new();
    static RE_STAMP: OnceLock<Regex> = OnceLock::new();
    static RE_MENTION: OnceLock<Regex> = OnceLock::new();

    let re_code_block = RE_CODE_BLOCK.get_or_init(|| Regex::new(r"```[\s\S]*?```").unwrap());
    let re_inline_code = RE_INLINE_CODE.get_or_init(|| Regex::new(r"`[^`]+`").unwrap());
    let re_math_block = RE_MATH_BLOCK.get_or_init(|| Regex::new(r"\$\$[\s\S]*?\$\$").unwrap());
    let re_math_inline = RE_MATH_INLINE.get_or_init(|| Regex::new(r"\$[^$]+\$").unwrap());
    let re_spoiler = RE_SPOILER.get_or_init(|| Regex::new(r"!![\s\S]*?!!").unwrap());
    let re_link = RE_LINK.get_or_init(|| Regex::new(r"https?://[\S]+").unwrap());
    let re_stamp = RE_STAMP.get_or_init(|| Regex::new(r":[a-zA-Z0-9_/@+-]+:").unwrap());
    let re_mention = RE_MENTION.get_or_init(|| Regex::new(r"@[a-zA-Z0-9_-]{1,32}").unwrap());

    let mut result = text.to_string();
    result = re_code_block.replace_all(&result, " ").to_string();
    result = re_inline_code.replace_all(&result, " ").to_string();
    result = re_math_block.replace_all(&result, " ").to_string();
    result = re_math_inline.replace_all(&result, " ").to_string();
    result = re_spoiler.replace_all(&result, " ").to_string();
    result = re_link.replace_all(&result, " ").to_string();
    result = re_stamp.replace_all(&result, " ").to_string();
    result = re_mention.replace_all(&result, " ").to_string();

    result
}

pub(crate) fn split_into_phrases(text: &str, tokenizer: &Tokenizer) -> Option<Vec<Phrase>> {
    let text = &sanitize_text(text);
    let mut tokens = tokenizer.tokenize(text).ok()?;
    let mut phrases = vec![];
    let mut current_phrase = Phrase {
        text: String::new(),
        mora: 0,
    };

    /* 文節ごとに切って拍数を数える */
    for token in &mut tokens {
        let t_text_trimmed = token.text.trim();

        if t_text_trimmed.is_empty() {
            continue;
        }

        let mut word_class = String::new();
        let mut word_subclass = String::new();
        let mut reading = t_text_trimmed.to_string();

        if let Some(details) = token.get_details() {
            if !details.is_empty() {
                word_class = details[0].to_string();
            }
            if details.len() > 1 {
                word_subclass = details[1].to_string();
            }
            if details.len() > 7 && details[7] != "*" && !details[7].is_empty() {
                reading = details[7].to_string();
            }
        }

        let is_start = match word_class.as_str() {
            "助詞" | "助動詞" | "記号" => false,
            _ => !matches!(word_subclass.as_str(), "接尾" | "非自立"),
        };

        if is_start && !current_phrase.text.is_empty() {
            phrases.push(current_phrase);
            current_phrase = Phrase {
                text: String::new(),
                mora: 0,
            };
        }

        current_phrase.text.push_str(t_text_trimmed);
        current_phrase.mora += count_mora(&reading);
    }

    if !current_phrase.text.is_empty() {
        phrases.push(current_phrase);
    }
    Some(phrases)
}

pub(crate) fn detect_mogamigawa(phrases: &[Phrase]) -> Option<String> {
    let mut mora = 0;
    let mut boundary = None;
    for (index, phrase) in phrases.iter().enumerate() {
        mora += phrase.mora;
        if mora == 5 && boundary.is_none() {
            boundary = Some(index + 1);
        }
        if mora > 12 {
            return None;
        }
    }
    if mora != 12 {
        return None;
    }
    let (first, second) = phrases.split_at(boundary?);
    let first: String = first.iter().map(|phrase| phrase.text.as_str()).collect();
    let second: String = second.iter().map(|phrase| phrase.text.as_str()).collect();
    Some(format!("{first}\n{second}\n最上川"))
}

pub(crate) fn detect_senryu(phrases: &[Phrase]) -> Option<String> {
    /* 拍数を全通り試して5,7,5を検出する */
    for start in 0..phrases.len() {
        let target_mora = [5, 7, 5];
        let mut current_phase = 0;
        let mut current_mora = 0;
        let mut phrase = [String::new(), String::new(), String::new()];
        let mut is_valid = false;

        for p in &phrases[start..] {
            if current_mora == 0 && p.mora == 0 {
                let first_char = p.text.chars().next().unwrap_or(' ');
                if first_char.is_ascii_punctuation()
                    || first_char.is_whitespace()
                    || IGNORE_CHARS.contains(&first_char)
                {
                    continue;
                }
            }

            current_mora += p.mora;
            phrase[current_phase].push_str(&p.text);

            if current_mora == target_mora[current_phase] {
                current_phase += 1;
                current_mora = 0;

                if current_phase == 3 {
                    is_valid = true;
                    break;
                }
            } else if current_mora > target_mora[current_phase] {
                break;
            }
        }
        if is_valid {
            return Some(format!("{}\n{}\n{}", phrase[0], phrase[1], phrase[2]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{count_mora, detect_mogamigawa, sanitize_text, split_into_phrases};
    use lindera::Tokenizer;

    fn detect_senryu(text: &str, tokenizer: &Tokenizer) -> Option<String> {
        super::detect_senryu(&split_into_phrases(text, tokenizer)?)
    }

    #[test]
    fn test_count_mora() {
        assert_eq!(count_mora("あさごはん"), 5);
        assert_eq!(count_mora("こんにちは"), 5);
        assert_eq!(count_mora("「こんぱいる」えらあ"), 8);
        assert_eq!(count_mora("とうきょう"), 4);
        assert_eq!(count_mora("コーヒー"), 4);
        assert_eq!(count_mora("Rust!"), 4);
        assert_eq!(count_mora("！こ？れ、は。す!ご?い,."), 6);
    }

    #[test]
    fn mogamigawa_requires_a_five_mora_boundary_and_consumes_the_whole_message() {
        let tokenizer = super::super::tokenizer().unwrap();
        for text in [
            "古池や蛙飛びこむ",
            "古池や\n蛙飛びこむ",
            "「古池や、蛙飛びこむ。」 :frog: https://example.com",
            "コーヒーは 朝に飲みます",
        ] {
            assert!(
                detect_mogamigawa(&split_into_phrases(text, &tokenizer).unwrap()).is_some(),
                "{text}"
            );
        }
        for text in [
            "古池や",
            "古池や 蛙飛びこむ 水の音",
            "昨日 古池や 蛙飛びこむ",
            "朝ごはんを食べましたよ",
        ] {
            assert!(
                detect_mogamigawa(&split_into_phrases(text, &tokenizer).unwrap()).is_none(),
                "{text}"
            );
        }
        assert_eq!(
            detect_senryu("コーヒーは 朝に飲みます ひとやすみ", &tokenizer).as_deref(),
            Some("コーヒーは\n朝に飲みます\nひとやすみ")
        );
    }

    #[test]
    fn test_sanitize_text() {
        let code_block = "ここから\n```rust\nfn main() {}\n```\nここまで";
        assert_eq!(sanitize_text(code_block), "ここから\n \nここまで");

        assert_eq!(sanitize_text("変数 `x` に代入"), "変数   に代入");

        let math_block = "公式\n$$\nE = mc^2\n$$\nです";
        assert_eq!(sanitize_text(math_block), "公式\n \nです");

        assert_eq!(sanitize_text("速度 $v$ を求める"), "速度   を求める");

        assert_eq!(sanitize_text("犯人は!!ヤス!!です"), "犯人は です");

        assert_eq!(
            sanitize_text("参考: https://q.trap.jp/messages/12345 を見てね"),
            "参考:   を見てね"
        );

        assert_eq!(
            sanitize_text("ありがとう :arigato: :+1: :blob_dance:"),
            "ありがとう      "
        );

        assert_eq!(
            sanitize_text("やあ @BOT_lachite と @user-123"),
            "やあ   と  "
        );

        let complex = "@BOT_lachite これ見て!!秘密!! https://example.com/ :neko: `true`";
        assert_eq!(sanitize_text(complex), "  これ見て       ");

        assert_eq!(
            sanitize_text("普通の日本語テキストです"),
            "普通の日本語テキストです"
        );
    }

    #[test]
    fn test_detect_senryu() {
        let tokenizer = super::super::tokenizer().unwrap();

        assert_eq!(
            detect_senryu("古池や蛙飛びこむ水の音", &tokenizer).unwrap(),
            "古池や\n蛙飛びこむ\n水の音"
        );
        assert_eq!(
            detect_senryu("菜の花や月は東に日は西に", &tokenizer).unwrap(),
            "菜の花や\n月は東に\n日は西に"
        );
        assert_eq!(
            detect_senryu("雪とけて村いっぱいの子どもかな", &tokenizer).unwrap(),
            "雪とけて\n村いっぱいの\n子どもかな"
        );
        assert_eq!(
            detect_senryu("柿くへば鐘が鳴るなり法隆寺", &tokenizer).unwrap(),
            "柿くへば\n鐘が鳴るなり\n法隆寺"
        );
        /* 枯野が読めないらしい */
        assert_eq!(
            detect_senryu("遠山に日の当たりたる枯野かな", &tokenizer),
            None
        );
        /* 門波が読めないらしい */
        assert_eq!(
            detect_senryu("流氷や宗谷の門波荒れやまず", &tokenizer),
            None
        );
        /* 5,7,5ではない */
        assert_eq!(
            detect_senryu("どうしようもないわたしが歩いている", &tokenizer),
            None
        );
        /* 初むるが読めないらしい */
        assert_eq!(
            detect_senryu("万緑の中や吾子の歯生え初むる", &tokenizer),
            None
        );
        assert_eq!(
            detect_senryu("鮟鱇の骨まで凍ててぶちきらる", &tokenizer).unwrap(),
            "鮟鱇の\n骨まで凍てて\nぶちきらる"
        );
        /* 墓 */
        assert_eq!(
            detect_senryu("新宿ははるかなる墓碑鳥渡る", &tokenizer).unwrap(),
            "新宿は\nはるかなる墓碑\n鳥渡る"
        );
        assert_eq!(
            detect_senryu("三月の甘納豆のうふふふふ", &tokenizer).unwrap(),
            "三月の\n甘納豆の\nうふふふふ"
        );

        assert_eq!(
            detect_senryu(
                "古池や :frog: 蛙飛びこむ 水の音 https://example.com",
                &tokenizer
            )
            .unwrap(),
            "古池や\n蛙飛びこむ\n水の音"
        );
        assert_eq!(
            detect_senryu(
                "!!ネタバレ!! 柿くへば 鐘が鳴るなり $$ E=mc^2 $$ 法隆寺",
                &tokenizer
            )
            .unwrap(),
            "柿くへば\n鐘が鳴るなり\n法隆寺"
        );
        assert_eq!(
            detect_senryu(
                "雪とけて `const SNOW = 0;` 村いっぱいの子どもかな",
                &tokenizer
            )
            .unwrap(),
            "雪とけて\n村いっぱいの\n子どもかな"
        );

        assert_eq!(
            detect_senryu("@BOT_lachite 古池や 蛙飛びこむ 水の音 @someone", &tokenizer).unwrap(),
            "古池や\n蛙飛びこむ\n水の音"
        );

        assert_eq!(
            detect_senryu("@BOT_lachite !!ネタバレ!! 柿くへば :stamp: 鐘が鳴るなり $$ E=mc^2 $$ 法隆寺 https://example.com", &tokenizer).unwrap(),
            "柿くへば\n鐘が鳴るなり\n法隆寺"
        );
    }
}
