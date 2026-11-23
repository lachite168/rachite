use rquickjs::{
    Context, Function, Object, Runtime,
    function::{Constructor, This},
};
use std::time::{Duration, Instant};

#[derive(Debug, thiserror::Error)]
pub(super) enum MatchError {
    #[error("Invalid ECMAScript regular expression or flags")]
    InvalidPattern,
    #[error("Stamp regular expression matching timed out")]
    TimedOut,
    #[error("QuickJS stamp matching failed: {0}")]
    QuickJs(#[from] rquickjs::Error),
}

pub(super) fn matching_names(
    pattern: &str,
    flags: &str,
    names: Vec<String>,
    budget: Duration,
) -> Result<Vec<String>, MatchError> {
    let deadline = Instant::now() + budget;
    let runtime = Runtime::new()?;
    runtime.set_interrupt_handler(Some(Box::new(move || Instant::now() >= deadline)));
    let context = Context::full(&runtime)?;
    let result = context.with(|ctx| {
        let constructor: Constructor = ctx.globals().get("RegExp")?;
        let regexp: Object = constructor.construct((pattern, flags)).map_err(|error| {
            if error.is_exception()
                && ctx.catch().as_object().is_some_and(|object| {
                    object
                        .get::<_, String>("name")
                        .is_ok_and(|name| name == "SyntaxError")
                })
            {
                MatchError::InvalidPattern
            } else {
                error.into()
            }
        })?;
        let test: Function = regexp.get("test")?;
        let mut matches = Vec::new();
        for name in names {
            if Instant::now() >= deadline {
                return Err(MatchError::TimedOut);
            }
            regexp.set("lastIndex", 0)?;
            if test.call((This(regexp.clone()), name.as_str()))? {
                matches.push(name);
            }
        }
        Ok(matches)
    });
    if Instant::now() >= deadline {
        return Err(MatchError::TimedOut);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::matching_names;
    use std::time::Duration;

    #[test]
    fn resets_the_ecmascript_match_position_for_each_stamp() {
        for (flags, expected) in [
            ("gi", vec!["N3_fox", "n3_cat", "old_n3"]),
            ("yi", vec!["N3_fox", "n3_cat"]),
        ] {
            let names = ["N3_fox", "n3_cat", "old_n3"].map(str::to_owned).to_vec();
            assert_eq!(
                matching_names("n3", flags, names, Duration::from_secs(2)).unwrap(),
                expected
            );
        }
    }
}
