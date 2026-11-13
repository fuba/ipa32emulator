/*
 * This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at https://mozilla.org/MPL/2.0/.
 */
//! `NSDateFormatter`.
//!
//! Resources:
//! - Apple's [Introduction to Data Formatting Programming Guide For Cocoa](https://developer.apple.com/library/archive/documentation/Cocoa/Conceptual/DataFormatting/DataFormatting.html)
//! - [Unicode Technical Standard #35](https://unicode.org/reports/tr35/tr35-10.html#Date_Format_Patterns)

use crate::frameworks::core_foundation::time::{
    gregorian_date_to_absolute_time, CFAbsoluteTimeGetGregorianDate, CFGregorianDate,
};
use crate::frameworks::foundation::{ns_string, NSTimeInterval, NSUInteger};
use crate::objc::{
    autorelease, id, msg, msg_class, nil, objc_classes, release, retain, ClassExports, HostObject,
    NSZonePtr,
};
use crate::Environment;

type NSDateFormatterStyle = NSUInteger;

struct NSDateFormatterHostObject {
    date_format: Option<id>,
    date_style: NSDateFormatterStyle,
    time_style: NSDateFormatterStyle,
    behavior: NSUInteger,
    locale: id,
    time_zone: id,
}
impl HostObject for NSDateFormatterHostObject {}

pub const CLASSES: ClassExports = objc_classes! {

(env, this, _cmd);

@implementation NSDateFormatter: NSObject

+ (id)allocWithZone:(NSZonePtr)_zone {
    let host_object = Box::new(NSDateFormatterHostObject {
        date_format: None,
        date_style: 0,
        time_style: 0,
        behavior: 0,
        locale: nil,
        time_zone: nil,
    });
    env.objc.alloc_object(this, host_object, &mut env.mem)
}

- (())setDateFormat:(id)format { // NSString *
    let date_format: id = msg![env; format copy];
    let host = env.objc.borrow_mut::<NSDateFormatterHostObject>(this);
    let old = std::mem::replace(&mut host.date_format, (date_format != nil).then_some(date_format));
    release(env, old.unwrap_or(nil));
}

- (())setTimeZone:(id)time_zone {
    retain(env, time_zone);
    let host = env.objc.borrow_mut::<NSDateFormatterHostObject>(this);
    let previous = std::mem::replace(&mut host.time_zone, time_zone);
    release(env, previous);
}
- (())setLocale:(id)locale {
    retain(env, locale);
    let host = env.objc.borrow_mut::<NSDateFormatterHostObject>(this);
    let previous = std::mem::replace(&mut host.locale, locale);
    release(env, previous);
}
- (id)locale {
    let locale = env.objc.borrow::<NSDateFormatterHostObject>(this).locale;
    if locale == nil { msg_class![env; NSLocale currentLocale] } else { locale }
}
- (id)timeZone {
    let zone = env.objc.borrow::<NSDateFormatterHostObject>(this).time_zone;
    if zone == nil { msg_class![env; NSTimeZone defaultTimeZone] } else { zone }
}

- (())setFormatterBehavior:(NSUInteger)behavior {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).behavior = behavior;
}
- (NSUInteger)formatterBehavior {
    env.objc.borrow::<NSDateFormatterHostObject>(this).behavior
}

// Setting a style replaces the format, as in Cocoa.
- (())setDateStyle:(NSDateFormatterStyle)style {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).date_style = style;
    set_format_from_styles(env, this);
}
- (())setTimeStyle:(NSDateFormatterStyle)style {
    env.objc.borrow_mut::<NSDateFormatterHostObject>(this).time_style = style;
    set_format_from_styles(env, this);
}

- (id)dateFromString:(id)string {
    if string == nil {
        return nil;
    }
    let Some(format) = env.objc.borrow::<NSDateFormatterHostObject>(this).date_format else {
        return nil;
    };
    let format = ns_string::to_rust_string(env, format);
    let input = ns_string::to_rust_string(env, string);
    let Some(date) = parse_numeric_date(&format, &input) else {
        return nil;
    };
    let zone: id = msg![env; this timeZone];
    let offset: i32 = msg![env; zone secondsFromGMT];
    let interval = gregorian_date_to_absolute_time(date) - f64::from(offset);
    msg_class![env; NSDate dateWithTimeIntervalSinceReferenceDate:interval]
}

- (id)stringFromDate:(id)date {
    let &NSDateFormatterHostObject {
        date_format, ..
    } = env.objc.borrow(this);
    let format = ns_string::to_rust_string(env, date_format.unwrap()).to_string();
    log_dbg!("date_format before: {:?}", format);

    let zone: id = msg![env; this timeZone];
    let offset: i32 = msg![env; zone secondsFromGMT];
    let ti: NSTimeInterval = msg![env; date timeIntervalSinceReferenceDate];
    let ti = ti + f64::from(offset);
    let greg_date = CFAbsoluteTimeGetGregorianDate(env, ti, nil);
    // The reference date (2001-01-01) was a Monday.
    let weekday = ((ti / 86400.0).floor() as i64 + 1).rem_euclid(7) as usize;
    let res = format_date(&format, &greg_date, weekday, offset);
    log_dbg!("date_format after: {:?}", res);

    let res = ns_string::from_rust_string(env, res);
    autorelease(env, res)
}

- (())dealloc {
    let &NSDateFormatterHostObject { date_format, locale, time_zone, .. } = env.objc.borrow(this);
    release(env, date_format.unwrap_or(nil));
    release(env, locale);
    release(env, time_zone);
    env.objc.dealloc_object(this, &mut env.mem);
}

@end

};

/// The en_US patterns for each style (none, short, medium, long, full).
fn set_format_from_styles(env: &mut Environment, this: id) {
    const DATE: [&str; 5] = [
        "",
        "M/d/yy",
        "MMM d, yyyy",
        "MMMM d, yyyy",
        "EEEE, MMMM d, yyyy",
    ];
    const TIME: [&str; 5] = ["", "h:mm a", "h:mm:ss a", "h:mm:ss a z", "h:mm:ss a zzzz"];
    let host_object = env.objc.borrow::<NSDateFormatterHostObject>(this);
    let date = DATE[host_object.date_style as usize];
    let time = TIME[host_object.time_style as usize];
    let format = [date, time]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let format = ns_string::from_rust_string(env, format);
    let old = env
        .objc
        .borrow_mut::<NSDateFormatterHostObject>(this)
        .date_format
        .replace(format);
    if let Some(old) = old {
        release(env, old);
    }
}

/// Parses numeric Gregorian fields and quoted literals in a UTC date pattern.
fn parse_numeric_date(format: &str, mut input: &str) -> Option<CFGregorianDate> {
    let mut date = CFGregorianDate {
        year: 2000,
        month: 1,
        day: 1,
        hours: 0,
        minutes: 0,
        seconds: 0.0,
    };
    let mut pattern = format.chars().peekable();
    let mut quoted = false;
    while let Some(field) = pattern.next() {
        if field == '\'' {
            if pattern.next_if_eq(&'\'').is_some() {
                input = input.strip_prefix('\'')?;
            } else {
                quoted = !quoted;
            }
            continue;
        }
        if quoted || !field.is_ascii_alphabetic() {
            input = input.strip_prefix(field)?;
            continue;
        }
        let mut width = 1;
        while pattern.next_if_eq(&field).is_some() {
            width += 1;
        }
        let digits = input.bytes().take_while(u8::is_ascii_digit).count();
        let width = match field {
            'y' if width >= 3 => digits.min(width.max(4)),
            'M' | 'd' | 'H' | 'm' | 's' if width == 1 => digits.min(2),
            _ => width,
        };
        if width == 0 || digits < width {
            return None;
        }
        let value: i32 = input[..width].parse().ok()?;
        input = &input[width..];
        match field {
            'y' if value > 0 && width >= 3 => date.year = value,
            'M' if width <= 2 && (1..=12).contains(&value) => date.month = value as i8,
            'd' if (1..=31).contains(&value) => date.day = value as i8,
            'H' if (0..24).contains(&value) => date.hours = value as i8,
            'm' if (0..60).contains(&value) => date.minutes = value as i8,
            's' if (0..60).contains(&value) => date.seconds = value.into(),
            _ => return None,
        }
    }
    let leap_year = date.year % 4 == 0 && (date.year % 100 != 0 || date.year % 400 == 0);
    let days_per_month = [
        31,
        28 + i8::from(leap_year),
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    (!quoted && input.is_empty() && date.day <= days_per_month[date.month as usize - 1])
        .then_some(date)
}

/// Formats a date with a Unicode date pattern (en_US, in UTC).
fn format_date(format: &str, date: &CFGregorianDate, weekday: usize, offset: i32) -> String {
    const MONTHS: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    const DAYS: [&str; 7] = [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ];
    let month = MONTHS[date.month as usize - 1];
    let hour12 = (date.hours as i32 + 11) % 12 + 1;
    let mut out = String::new();
    let mut chars = format.chars().peekable();
    while let Some(c) = chars.next() {
        if !c.is_ascii_alphabetic() {
            if c == '\'' {
                // Quoted text, where '' is a quote.
                while let Some(c) = chars.next() {
                    if c == '\'' && chars.next_if_eq(&'\'').is_none() {
                        break;
                    }
                    out.push(c);
                }
            } else {
                out.push(c);
            }
            continue;
        }
        let mut n = 1;
        while chars.next_if_eq(&c).is_some() {
            n += 1;
        }
        let num = |v: i32| format!("{v:0n$}");
        out += &match c {
            'y' | 'Y' if n == 2 => format!("{:02}", date.year % 100),
            'y' | 'Y' => num(date.year),
            'M' | 'L' if n >= 4 => month.to_string(),
            'M' | 'L' if n == 3 => month[..3].to_string(),
            'M' | 'L' => num(date.month.into()),
            'd' => num(date.day.into()),
            'H' => num(date.hours.into()),
            'h' => num(hour12),
            'm' => num(date.minutes.into()),
            's' => num(date.seconds as i32),
            'a' => (if date.hours < 12 { "AM" } else { "PM" }).to_string(),
            'E' | 'c' if n >= 4 => DAYS[weekday].to_string(),
            'E' | 'c' => DAYS[weekday][..3].to_string(),
            'Z' => {
                let sign = if offset < 0 { '-' } else { '+' };
                let minutes = offset.unsigned_abs() / 60;
                format!("{sign}{:02}{:02}", minutes / 60, minutes % 60)
            }
            // Dates are worked out in UTC (see CFAbsoluteTimeGetGregorianDate).
            'z' if n >= 4 => "Greenwich Mean Time".to_string(),
            'z' => "GMT".to_string(),
            _ => unimplemented!("date format pattern {}", c.to_string().repeat(n)),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openfeint_date_formats() {
        let expected = CFGregorianDate {
            year: 2010,
            month: 2,
            day: 28,
            hours: 15,
            minutes: 5,
            seconds: 9.0,
        };
        assert_eq!(
            parse_numeric_date("yyyy-MM-dd'T'HH:mm:ss'Z'", "2010-02-28T15:05:09Z"),
            Some(expected)
        );
        assert!(parse_numeric_date("yyyy-MM-dd", "2010-02-29").is_none());
        assert!(parse_numeric_date("yyyy-MM-dd", "2012-02-29").is_some());
    }

    #[test]
    fn test_format_date() {
        // 2009-07-04 15:05:09, a Saturday.
        let date = CFGregorianDate {
            year: 2009,
            month: 7,
            day: 4,
            hours: 15,
            minutes: 5,
            seconds: 9.0,
        };
        let f = |format| format_date(format, &date, 6, 0);
        assert_eq!(f("yyyy-MM-dd HH:mm:ss"), "2009-07-04 15:05:09");
        assert_eq!(f("M/d/yy h:mm a"), "7/4/09 3:05 PM");
        assert_eq!(f("MMM d, yyyy h:mm:ss a"), "Jul 4, 2009 3:05:09 PM");
        assert_eq!(f("EEEE, MMMM d, yyyy"), "Saturday, July 4, 2009");
        assert_eq!(f("'at' HH 'o''clock'"), "at 15 o'clock");
    }
}
