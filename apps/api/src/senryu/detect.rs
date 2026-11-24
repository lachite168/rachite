use super::{
    dictionary::{Reading, VerseDictionary, Word},
    markdown::extract_text,
};
use anyhow::Result;

#[derive(Default)]
struct Phrase {
    text: String,
    mora: usize,
}

#[derive(Default)]
pub(crate) struct Verses {
    pub senryu: Option<String>,
    pub mogamigawa: Option<String>,
}

pub(crate) fn detect(source: &str, dictionary: &VerseDictionary) -> Result<Verses> {
    let text = extract_text(source);
    let words = dictionary.words(&text)?;
    Ok(detect_words(&words))
}

fn detect_words(words: &[Word<'_>]) -> Verses {
    let is_unreadable = |word: &Word<'_>| matches!(word.reading, Reading::Unknown);
    if words.iter().any(is_unreadable) {
        let senryu = words
            .split(is_unreadable)
            .find_map(|group| detect_senryu(&split_into_phrases(group)));
        return Verses {
            senryu,
            mogamigawa: None,
        };
    }
    let phrases = split_into_phrases(words);
    Verses {
        senryu: detect_senryu(&phrases),
        mogamigawa: detect_mogamigawa(&phrases),
    }
}

fn count_mora(reading: &str) -> usize {
    reading
        .chars()
        .filter(|character| {
            !"ぁぃぅぇぉゃゅょゎァィゥェォャュョヮㇰㇱㇲㇳㇴㇵㇶㇷㇸㇹㇺㇻㇼㇽㇾㇿ"
                .contains(*character)
        })
        .count()
}

fn split_into_phrases(words: &[Word<'_>]) -> Vec<Phrase> {
    let mut phrases = Vec::new();
    let mut current = Phrase::default();
    let mut after_prefix = false;
    for word in words {
        if !word.joins_previous && !after_prefix && current.mora > 0 {
            phrases.push(std::mem::take(&mut current));
        }
        current.text.push_str(word.surface);
        if let Reading::Kana(reading) = &word.reading {
            current.mora += count_mora(reading);
            after_prefix = word.joins_next;
        }
    }
    if current.mora > 0 {
        phrases.push(current);
    }
    phrases
}

fn detect_mogamigawa(phrases: &[Phrase]) -> Option<String> {
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

fn detect_senryu(phrases: &[Phrase]) -> Option<String> {
    /* 拍数を全通り試して5,7,5を検出する */
    for start in 0..phrases.len() {
        let target_mora = [5, 7, 5];
        let mut current_phase = 0;
        let mut current_mora = 0;
        let mut phrase = [String::new(), String::new(), String::new()];
        let mut is_valid = false;

        for p in &phrases[start..] {
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
    use super::{VerseDictionary, count_mora, detect};
    use std::sync::OnceLock;

    fn dictionary() -> &'static VerseDictionary {
        static DICTIONARY: OnceLock<VerseDictionary> = OnceLock::new();
        DICTIONARY.get_or_init(|| VerseDictionary::sudachidict().unwrap())
    }

    #[test]
    fn detects_readable_latin_words_with_the_original_spelling() {
        let dictionary = dictionary();
        assert_eq!(
            detect_senryu("YouTube Microsoft YouTube", dictionary).as_deref(),
            Some("YouTube\nMicrosoft\nYouTube")
        );
        assert_eq!(
            detect("YouTube Microsoft", dictionary)
                .unwrap()
                .mogamigawa
                .as_deref(),
            Some("YouTube\nMicrosoft\n最上川")
        );
    }

    fn detect_senryu(text: &str, dictionary: &VerseDictionary) -> Option<String> {
        detect(text, dictionary).unwrap().senryu
    }

    #[test]
    fn test_count_mora() {
        assert_eq!(count_mora("あさごはん"), 5);
        assert_eq!(count_mora("こんにちは"), 5);
        assert_eq!(count_mora("こんぱいるえらあ"), 8);
        assert_eq!(count_mora("とうきょう"), 4);
        assert_eq!(count_mora("コーヒー"), 4);
        assert_eq!(count_mora("キャッチ"), 3);
    }

    #[test]
    fn mogamigawa_requires_a_five_mora_boundary_and_consumes_the_whole_message() {
        let dictionary = dictionary();
        for text in [
            "古池や蛙飛びこむ",
            "古池や\n蛙飛びこむ",
            "「古池や、蛙飛びこむ。」 :frog: https://example.com",
            "古池や 1️⃣ 蛙飛びこむ",
            "コーヒーは 朝に飲みます",
            "ｺｰﾋｰは 朝に飲みます",
        ] {
            assert!(
                detect(text, dictionary).unwrap().mogamigawa.is_some(),
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
                detect(text, dictionary).unwrap().mogamigawa.is_none(),
                "{text}"
            );
        }
        assert_eq!(
            detect_senryu("コーヒーは 朝に飲みます ひとやすみ", dictionary).as_deref(),
            Some("コーヒーは\n朝に飲みます\nひとやすみ")
        );
    }

    #[test]
    fn rejects_unknown_words_and_symbols_and_resumes_after_a_gap() {
        let dictionary = dictionary();
        for text in [
            "aaaaa aaaaaaa aaaaa",
            "aaaaa aaaaaaa",
            "古池や zzzzz 蛙飛びこむ",
            "古池や zzzzz 蛙飛びこむ 水の音",
            "---\n\n✨✨✨✨✨ 🔶🔶🔶🔶🔶🔶🔶\n\n***",
        ] {
            let verses = detect(text, dictionary).unwrap();
            assert!(verses.senryu.is_none(), "{text}");
            assert!(verses.mogamigawa.is_none(), "{text}");
        }
        for text in [
            "zzzzz 古池や 蛙飛びこむ 水の音",
            "古池や 蛙飛びこむ 水の音 zzzzz",
            "1. **古池や**\n2. [蛙飛びこむ](https://example.com)\n\n---\n\n> 水の音",
        ] {
            let verses = detect(text, dictionary).unwrap();
            assert_eq!(
                verses.senryu.as_deref(),
                Some("古池や\n蛙飛びこむ\n水の音"),
                "{text}"
            );
            assert!(verses.mogamigawa.is_none(), "{text}");
        }
    }

    #[test]
    fn test_detect_senryu() {
        let dictionary = dictionary();

        assert_eq!(
            detect_senryu("古池や蛙飛びこむ水の音", dictionary).unwrap(),
            "古池や\n蛙飛びこむ\n水の音"
        );
        assert_eq!(
            detect_senryu("菜の花や 月は東に 日は西に", dictionary).unwrap(),
            "菜の花や\n月は東に\n日は西に"
        );
        // SudachiDict treats the continuous title 月は東に日は西に as one word.
        assert_eq!(detect_senryu("菜の花や月は東に日は西に", dictionary), None);
        assert_eq!(
            detect_senryu("雪とけて村いっぱいの子どもかな", dictionary).unwrap(),
            "雪とけて\n村いっぱいの\n子どもかな"
        );
        assert_eq!(
            detect_senryu("柿くへば鐘が鳴るなり法隆寺", dictionary).unwrap(),
            "柿くへば\n鐘が鳴るなり\n法隆寺"
        );
        assert_eq!(
            detect_senryu("遠山に日の当たりたる枯野かな", dictionary).as_deref(),
            Some("遠山に\n日の当たりたる\n枯野かな")
        );
        assert_eq!(
            detect_senryu("流氷や宗谷の門波荒れやまず", dictionary).as_deref(),
            Some("流氷や\n宗谷の門波\n荒れやまず")
        );
        /* 5,7,5ではない */
        assert_eq!(
            detect_senryu("どうしようもないわたしが歩いている", dictionary),
            None
        );
        assert_eq!(
            detect_senryu("万緑の中や吾子の歯生え初むる", dictionary),
            None
        );
        assert_eq!(
            detect_senryu("鮟鱇の骨まで凍ててぶちきらる", dictionary).unwrap(),
            "鮟鱇の\n骨まで凍てて\nぶちきらる"
        );
        assert_eq!(
            detect_senryu("新宿ははるかなる墓碑鳥渡る", dictionary).unwrap(),
            "新宿は\nはるかなる墓碑\n鳥渡る"
        );
        assert_eq!(
            detect_senryu("三月の甘納豆のうふふふふ", dictionary).unwrap(),
            "三月の\n甘納豆の\nうふふふふ"
        );

        assert_eq!(
            detect_senryu(
                "古池や :frog: 蛙飛びこむ 水の音 https://example.com",
                dictionary
            )
            .unwrap(),
            "古池や\n蛙飛びこむ\n水の音"
        );
        assert_eq!(
            detect_senryu(
                "!!ネタバレ!! 柿くへば 鐘が鳴るなり $$ E=mc^2 $$ 法隆寺",
                dictionary
            )
            .unwrap(),
            "柿くへば\n鐘が鳴るなり\n法隆寺"
        );
        assert_eq!(
            detect_senryu(
                "雪とけて `const SNOW = 0;` 村いっぱいの子どもかな",
                dictionary
            )
            .unwrap(),
            "雪とけて\n村いっぱいの\n子どもかな"
        );

        assert_eq!(
            detect_senryu("@BOT_lachite 古池や 蛙飛びこむ 水の音 @someone", dictionary).unwrap(),
            "古池や\n蛙飛びこむ\n水の音"
        );

        assert_eq!(
            detect_senryu("@BOT_lachite !!ネタバレ!! 柿くへば :stamp: 鐘が鳴るなり $$ E=mc^2 $$ 法隆寺 https://example.com", dictionary).unwrap(),
            "柿くへば\n鐘が鳴るなり\n法隆寺"
        );
    }
}
