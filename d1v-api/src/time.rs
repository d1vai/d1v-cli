use jiff::{Timestamp, civil::DateTime, tz::TimeZone};
use serde::{Deserialize, Deserializer};

pub fn deserialize_timestamp<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    parse_timestamp(&value).map_err(serde::de::Error::custom)
}

pub fn deserialize_optional_timestamp<'de, D>(
    deserializer: D,
) -> Result<Option<Timestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    value
        .map(|value| parse_timestamp(&value).map_err(serde::de::Error::custom))
        .transpose()
}

fn parse_timestamp(value: &str) -> Result<Timestamp, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("timestamp is empty".to_string());
    }

    trimmed
        .parse::<Timestamp>()
        .or_else(|_| {
            trimmed
                .parse::<DateTime>()
                .and_then(|datetime| datetime.to_zoned(TimeZone::UTC).map(|zdt| zdt.timestamp()))
        })
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_timestamp;

    #[test]
    fn parses_rfc3339_timestamp() {
        let parsed = parse_timestamp("2026-02-12T06:30:55Z").unwrap();
        assert_eq!(parsed.to_string(), "2026-02-12T06:30:55Z");
    }

    #[test]
    fn parses_naive_timestamp_as_utc() {
        let parsed = parse_timestamp("2026-02-12T06:30:55").unwrap();
        assert_eq!(parsed.to_string(), "2026-02-12T06:30:55Z");
    }
}
