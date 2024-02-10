use super::Argument;

pub struct FloatRangeArgument(pub &'static str);

impl Argument for FloatRangeArgument {
    type ValueType = std::ops::RangeInclusive<f64>;

    fn name(&self) -> &'static str {
        self.0
    }

    fn try_parse(s: &str) -> Option<std::ops::RangeInclusive<f64>> {
        let mut parts = s.split("..");
        let Some(start) = parts.next() else {
            return None;
        };
        let Some(end) = parts.next() else {
            return None;
        };
        if parts.next().is_some() {
            return None;
        }
        let Ok(start) = start.parse::<f64>() else {
            return None;
        };
        let Ok(end) = end.parse::<f64>() else {
            return None;
        };
        Some(start..=end)
    }
}
