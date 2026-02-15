use crate::errors::AppError;
use chrono::{Datelike, Duration, NaiveDateTime, NaiveTime, Weekday};

pub fn business_hours_between(
    start: NaiveDateTime,
    end: NaiveDateTime,
    work_start_hour: u32,
    work_end_hour: u32,
) -> Result<f64, AppError> {
    // Validate work hours are in valid range (0-23)
    if work_start_hour > 23 || work_end_hour > 23 {
        return Err(AppError::Internal(format!(
            "Invalid work hours: start={}, end={} (must be 0-23)",
            work_start_hour, work_end_hour
        )));
    }

    if work_start_hour >= work_end_hour {
        return Err(AppError::Internal(format!(
            "Work start hour ({}) must be less than work end hour ({})",
            work_start_hour, work_end_hour
        )));
    }

    if end <= start {
        return Ok(0.0);
    }

    let work_start = NaiveTime::from_hms_opt(work_start_hour, 0, 0).ok_or_else(|| {
        AppError::Internal(format!("Invalid work start hour: {}", work_start_hour))
    })?;
    let work_end = NaiveTime::from_hms_opt(work_end_hour, 0, 0)
        .ok_or_else(|| AppError::Internal(format!("Invalid work end hour: {}", work_end_hour)))?;
    let mut total_minutes: i64 = 0;

    let mut current_date = start.date();
    let end_date = end.date();

    while current_date <= end_date {
        let weekday = current_date.weekday();
        if weekday != Weekday::Sat && weekday != Weekday::Sun {
            let day_start = if current_date == start.date() {
                start.time().max(work_start)
            } else {
                work_start
            };
            let day_end = if current_date == end_date {
                end.time().min(work_end)
            } else {
                work_end
            };

            if day_end > day_start {
                let diff = day_end - day_start;
                total_minutes += diff.num_minutes();
            }
        }
        current_date += Duration::days(1);
    }

    Ok(total_minutes as f64 / 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_business_hours_same_day() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap();
        let hours = business_hours_between(start, end, 9, 17).unwrap();
        assert_eq!(hours, 5.0);
    }

    #[test]
    fn test_business_hours_multi_day() {
        // Monday 4PM to Wednesday 10:30AM
        let start = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(16, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 8)
            .unwrap()
            .and_hms_opt(10, 30, 0)
            .unwrap();
        let hours = business_hours_between(start, end, 9, 17).unwrap();
        // Monday: 16:00->17:00 = 1h
        // Tuesday: 09:00->17:00 = 8h
        // Wednesday: 09:00->10:30 = 1.5h
        // Total: 10.5h
        assert_eq!(hours, 10.5);
    }

    #[test]
    fn test_business_hours_weekend_excluded() {
        // Friday 4PM to Monday 10AM
        let start = NaiveDate::from_ymd_opt(2025, 1, 10)
            .unwrap()
            .and_hms_opt(16, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 13)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let hours = business_hours_between(start, end, 9, 17).unwrap();
        // Friday: 16:00->17:00 = 1h
        // Sat/Sun: 0h (excluded)
        // Monday: 09:00->10:00 = 1h
        // Total: 2h
        assert_eq!(hours, 2.0);
    }

    #[test]
    fn test_business_hours_zero_if_reversed() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let hours = business_hours_between(start, end, 9, 17).unwrap();
        assert_eq!(hours, 0.0);
    }

    #[test]
    fn test_invalid_work_hours() {
        let start = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2025, 1, 6)
            .unwrap()
            .and_hms_opt(15, 0, 0)
            .unwrap();

        // Test invalid hour > 23
        assert!(business_hours_between(start, end, 25, 17).is_err());

        // Test start >= end
        assert!(business_hours_between(start, end, 17, 9).is_err());
    }
}
