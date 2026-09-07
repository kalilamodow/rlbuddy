use std::time::Duration;

static NO_DURATION: Duration = Duration::ZERO;
static ONE_SECOND: Duration = Duration::from_secs(1);
static ONE_MINUTE: Duration = Duration::from_mins(1);
static ONE_HOUR: Duration = Duration::from_hours(1);

#[derive(Debug, Clone, Copy)]
enum TimeUnit {
    Second,
    Minute,
    Hour,
}

impl TimeUnit {
    fn normal(&self) -> &'static str {
        match self {
            Self::Second => "second",
            Self::Minute => "minute",
            Self::Hour => "hour",
        }
    }

    fn short(&self) -> &'static str {
        match self {
            Self::Second => "s",
            Self::Minute => "m",
            Self::Hour => "h",
        }
    }

    fn duration(&self) -> &'static Duration {
        match self {
            Self::Second => &ONE_SECOND,
            Self::Minute => &ONE_MINUTE,
            Self::Hour => &ONE_HOUR,
        }
    }
}

#[derive(Debug)]
struct TimePart {
    unit: TimeUnit,
    value: u64,
}

impl TimePart {
    fn new(value: u64, unit: TimeUnit) -> Self {
        Self { unit, value }
    }

    fn format(&self, short: bool) -> String {
        if short {
            self.format_short()
        } else {
            self.format_normal()
        }
    }

    fn format_normal(&self) -> String {
        let name = self.unit.normal();
        let plural = if self.value == 1 { "" } else { "s" };
        format!("{} {name}{plural}", self.value)
    }

    fn format_short(&self) -> String {
        format!("{}{}", self.value, self.unit.short())
    }
}

/// Returns a formatted string and how long itll take to need to update. Like
/// if it's in seconds, it'll tell you that you need to refresh in 1 second
/// (because eguis immediate mode and doesnt update itself)
pub fn format_seconds(total_seconds: u64, short: bool) -> (String, &'static Duration) {
    let hours = total_seconds / (60 * 60);
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;

    let parts: Vec<TimePart> = [
        (hours > 0).then(|| TimePart::new(hours, TimeUnit::Hour)),
        (minutes > 0).then(|| TimePart::new(minutes, TimeUnit::Minute)),
        (total_seconds < 60 && seconds > 0).then(|| TimePart::new(seconds, TimeUnit::Second)),
    ]
    .into_iter()
    .flatten()
    .collect();

    let Some(last) = parts.last() else {
        return ("Just now".into(), &NO_DURATION);
    };

    let output_duration = last.unit.duration();
    let output_string = parts
        .into_iter()
        .map(|t| t.format(short))
        .collect::<Vec<_>>()
        .join(" ");

    (output_string, output_duration)
}
