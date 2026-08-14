use std::time::Duration;

fn pluralize_ago(count: u64, word: &str, suffix: &str, minimal: bool) -> String {
    format!(
        "{count}{}{word}{} {suffix}",
        if minimal { "" } else { " " },
        if minimal || count == 1 { "" } else { "s" }
    )
}

const ONE_SECOND: Duration = Duration::from_secs(1);
const ONE_MINUTE: Duration = Duration::from_mins(1);

/// Returns a formatted string and how long itll take to need to update. Like
/// if it's in seconds, it'll tell you that you need to refresh in 1 second
/// (because eguis immediate mode and doesnt update itself)
pub fn format_seconds(seconds: u64, short: bool) -> (String, Duration) {
    match seconds {
        ..60 => (
            pluralize_ago(seconds, if short { "s" } else { "second" }, "ago", short),
            ONE_SECOND,
        ),
        60..3600 => (
            pluralize_ago(
                seconds / 60,
                if short { "m" } else { "minute" },
                "ago",
                short,
            ),
            ONE_MINUTE,
        ),
        3600.. => (
            format!(
                "{}{}",
                pluralize_ago(seconds / 3600, if short { "h" } else { "hour" }, "", short),
                pluralize_ago(
                    (seconds % 3600) / 60,
                    if short { "m" } else { "minute" },
                    "ago",
                    short
                )
            ),
            ONE_MINUTE,
        ),
    }
}
