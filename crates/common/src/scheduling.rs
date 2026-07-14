use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// Frequency for recurrence rules
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Frequency {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

/// Day of week
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DayOfWeek {
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

/// Recurrence rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub frequency: Frequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateTime<Utc>>,
    pub by_day: Vec<DayOfWeek>,
    pub by_month: Vec<u32>,
    pub by_month_day: Vec<u32>,
}

impl RecurrenceRule {
    /// Parse RRULE string
    pub fn parse(rrule: &str) -> Result<Self, String> {
        let mut frequency = Frequency::Daily;
        let mut interval = 1;
        let mut count = None;
        let mut until = None;
        let mut by_day = Vec::new();
        let mut by_month = Vec::new();
        let mut by_month_day = Vec::new();

        for part in rrule.split(';') {
            let mut kv = part.splitn(2, '=');
            if let (Some(key), Some(value)) = (kv.next(), kv.next()) {
                match key {
                    "FREQ" => {
                        frequency = match value {
                            "DAILY" => Frequency::Daily,
                            "WEEKLY" => Frequency::Weekly,
                            "MONTHLY" => Frequency::Monthly,
                            "YEARLY" => Frequency::Yearly,
                            _ => return Err(format!("Invalid frequency: {}", value)),
                        };
                    }
                    "INTERVAL" => {
                        interval = value.parse().map_err(|e| format!("Invalid interval: {}", e))?;
                    }
                    "COUNT" => {
                        count = Some(value.parse().map_err(|e| format!("Invalid count: {}", e))?);
                    }
                    "UNTIL" => {
                        until = Some(
                            DateTime::parse_from_rfc3339(value)
                                .map_err(|e| format!("Invalid until: {}", e))?
                                .with_timezone(&Utc),
                        );
                    }
                    "BYDAY" => {
                        for day in value.split(',') {
                            by_day.push(match day {
                                "SU" => DayOfWeek::Sunday,
                                "MO" => DayOfWeek::Monday,
                                "TU" => DayOfWeek::Tuesday,
                                "WE" => DayOfWeek::Wednesday,
                                "TH" => DayOfWeek::Thursday,
                                "FR" => DayOfWeek::Friday,
                                "SA" => DayOfWeek::Saturday,
                                _ => return Err(format!("Invalid day: {}", day)),
                            });
                        }
                    }
                    "BYMONTH" => {
                        for month in value.split(',') {
                            by_month.push(month.parse().map_err(|e| format!("Invalid month: {}", e))?);
                        }
                    }
                    "BYMONTHDAY" => {
                        for day in value.split(',') {
                            by_month_day.push(day.parse().map_err(|e| format!("Invalid day: {}", e))?);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(Self {
            frequency,
            interval,
            count,
            until,
            by_day,
            by_month,
            by_month_day,
        })
    }

    /// Generate instances of a recurring event
    pub fn generate_instances(
        &self,
        start: DateTime<Utc>,
        range_start: DateTime<Utc>,
        range_end: DateTime<Utc>,
    ) -> Vec<DateTime<Utc>> {
        let mut instances = Vec::new();
        let mut current = start;
        let mut count = 0;

        while current <= range_end {
            if current >= range_start {
                instances.push(current);
            }

            if let Some(max_count) = self.count {
                count += 1;
                if count >= max_count {
                    break;
                }
            }

            if let Some(until) = self.until
                && current > until
            {
                break;
            }

            current = self.next_occurrence(current);
        }

        instances
    }

    fn next_occurrence(&self, current: DateTime<Utc>) -> DateTime<Utc> {
        match self.frequency {
            Frequency::Daily => current + chrono::Duration::days(self.interval as i64),
            Frequency::Weekly => current + chrono::Duration::weeks(self.interval as i64),
            Frequency::Monthly => {
                let mut year = current.year();
                let mut month = current.month() + self.interval;
                if month > 12 {
                    year += (month / 12) as i32;
                    month %= 12;
                    if month == 0 {
                        month = 12;
                        year -= 1;
                    }
                }
                let day = current.day().min(28);
                Utc.with_ymd_and_hms(year, month, day, current.hour(), current.minute(), current.second())
                    .unwrap()
            }
            Frequency::Yearly => {
                let year = current.year() + self.interval as i32;
                Utc.with_ymd_and_hms(
                    year,
                    current.month(),
                    current.day(),
                    current.hour(),
                    current.minute(),
                    current.second(),
                )
                .unwrap()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_daily() {
        let rule = RecurrenceRule::parse("FREQ=DAILY;COUNT=10").unwrap();
        assert_eq!(rule.frequency, Frequency::Daily);
        assert_eq!(rule.count, Some(10));
    }

    #[test]
    fn test_parse_weekly() {
        let rule = RecurrenceRule::parse("FREQ=WEEKLY;BYDAY=MO,WE,FR").unwrap();
        assert_eq!(rule.frequency, Frequency::Weekly);
        assert_eq!(rule.by_day.len(), 3);
    }

    #[test]
    fn test_next_occurrence_daily() {
        let rule = RecurrenceRule::parse("FREQ=DAILY;INTERVAL=1").unwrap();
        let start = Utc.with_ymd_and_hms(2024, 1, 1, 10, 0, 0).unwrap();
        let next = rule.next_occurrence(start);
        assert_eq!(next, Utc.with_ymd_and_hms(2024, 1, 2, 10, 0, 0).unwrap());
    }
}
