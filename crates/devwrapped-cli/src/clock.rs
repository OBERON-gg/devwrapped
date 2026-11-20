use std::ffi::OsString;

use devwrapped_card::Clock;

// regions whose common time format is 12-hour; Canada only for English
const TWELVE_HOUR_REGIONS: [&str; 11] = [
    "AU", "BD", "EG", "IN", "KR", "NZ", "PH", "PK", "SA", "TW", "US",
];

/// guesses the clock from the system settings, and falls back to 24-hour
pub fn detect(var: impl Fn(&str) -> Option<OsString>) -> Clock {
    system_setting()
        .or_else(|| from_locale(&locale(var)?))
        .unwrap_or(Clock::H24)
}

fn locale(var: impl Fn(&str) -> Option<OsString>) -> Option<String> {
    ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .filter_map(|name| var(name)?.into_string().ok())
        .find(|value| !value.is_empty())
}

/// reads a locale such as `en_US.UTF-8`, and gives `None` when it names no region
fn from_locale(locale: &str) -> Option<Clock> {
    let name = locale.split(['.', '@']).next()?;
    let (language, region) = name.split_once(['_', '-'])?;
    let twelve = TWELVE_HOUR_REGIONS.contains(&region)
        || (region == "CA" && language.eq_ignore_ascii_case("en"));
    Some(if twelve { Clock::H12 } else { Clock::H24 })
}

#[cfg(windows)]
fn system_setting() -> Option<Clock> {
    let output = std::process::Command::new("reg")
        .args(["query", r"HKCU\Control Panel\International", "/v", "iTime"])
        .output()
        .ok()?;
    from_windows_setting(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
fn system_setting() -> Option<Clock> {
    let forced = |key: &str| {
        std::process::Command::new("defaults")
            .args(["read", "-g", key])
            .output()
            .is_ok_and(|output| output.stdout.trim_ascii() == b"1")
    };
    if forced("AppleICUForce24HourTime") {
        Some(Clock::H24)
    } else if forced("AppleICUForce12HourTime") {
        Some(Clock::H12)
    } else {
        None
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
fn system_setting() -> Option<Clock> {
    None
}

// windows stores 0 for a 12-hour clock and 1 for a 24-hour clock
#[cfg(any(windows, test))]
fn from_windows_setting(output: &str) -> Option<Clock> {
    let line = output.lines().find(|line| line.contains("iTime"))?;
    match line.split_whitespace().last()? {
        "0" => Some(Clock::H12),
        "1" => Some(Clock::H24),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_region_of_the_locale() {
        assert_eq!(from_locale("en_US.UTF-8"), Some(Clock::H12));
        assert_eq!(from_locale("en_GB.UTF-8"), Some(Clock::H24));
        assert_eq!(from_locale("de_DE@euro"), Some(Clock::H24));
        assert_eq!(from_locale("en_CA"), Some(Clock::H12));
        assert_eq!(from_locale("fr_CA.UTF-8"), Some(Clock::H24));
        assert_eq!(from_locale("en-AU"), Some(Clock::H12));
        assert_eq!(from_locale("C.UTF-8"), None);
        assert_eq!(from_locale("POSIX"), None);
    }

    #[test]
    fn prefers_lc_all_then_lc_time_then_lang() {
        let vars = |pairs: &'static [(&'static str, &'static str)]| {
            move |name: &str| {
                pairs
                    .iter()
                    .find(|(key, _)| *key == name)
                    .map(|(_, value)| OsString::from(value))
            }
        };
        assert_eq!(
            locale(vars(&[("LANG", "en_US.UTF-8"), ("LC_TIME", "en_GB.UTF-8")])),
            Some("en_GB.UTF-8".to_string())
        );
        assert_eq!(
            locale(vars(&[("LC_ALL", ""), ("LANG", "en_US.UTF-8")])),
            Some("en_US.UTF-8".to_string())
        );
        assert_eq!(locale(vars(&[])), None);
    }

    #[test]
    fn reads_the_windows_clock_setting() {
        let output = |value| {
            format!(
                "\r\nHKEY_CURRENT_USER\\Control Panel\\International\r\n    iTime    REG_SZ    {value}\r\n\r\n"
            )
        };
        assert_eq!(from_windows_setting(&output("0")), Some(Clock::H12));
        assert_eq!(from_windows_setting(&output("1")), Some(Clock::H24));
        assert_eq!(from_windows_setting("ERROR: not found"), None);
    }
}
