use std::borrow::Cow;

#[derive(Debug, Clone, Copy)]
enum TimeUnit {
    Second,
    Minute,
    Hour,
    Day,
    Week,
}

impl TimeUnit {
    fn normal(self) -> &'static str {
        match self {
            Self::Second => "second",
            Self::Minute => "minute",
            Self::Hour => "hour",
            Self::Day => "day",
            Self::Week => "week",
        }
    }

    fn short(self) -> &'static str {
        match self {
            Self::Second => "s",
            Self::Minute => "m",
            Self::Hour => "h",
            Self::Day => "d",
            Self::Week => "w",
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

pub fn format_seconds(total_seconds: u64, short: bool) -> Cow<'static, str> {
    let weeks = total_seconds / (60 * 60 * 24 * 7);
    let days = total_seconds / (60 * 60 * 24);
    let hours = total_seconds / (60 * 60);
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;

    let parts: Vec<TimePart> = [
        (weeks > 0).then(|| TimePart::new(weeks, TimeUnit::Week)),
        (days > 0).then(|| TimePart::new(days, TimeUnit::Day)),
        (days == 0 && hours > 0).then(|| TimePart::new(hours, TimeUnit::Hour)),
        (days == 0 && minutes > 0).then(|| TimePart::new(minutes, TimeUnit::Minute)),
        (total_seconds < 60 && seconds > 0).then(|| TimePart::new(seconds, TimeUnit::Second)),
    ]
    .into_iter()
    .flatten()
    .collect();

    if parts.is_empty() {
        return Cow::Borrowed(&"Just now");
    }

    let output_string = parts
        .into_iter()
        .map(|t| t.format(short))
        .collect::<Vec<_>>()
        .join(" ");

    Cow::Owned(output_string)
}
