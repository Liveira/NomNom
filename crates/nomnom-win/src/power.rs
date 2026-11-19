//! The battery, as Windows reports it.

use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    /// 0 to 100.
    pub percent: u8,
    /// The charger is plugged in.
    pub plugged: bool,
    /// It is taking charge right now.
    pub charging: bool,
    /// Minutes of battery left, when Windows can tell (only while it is running on the battery).
    pub minutes_left: Option<u32>,
}

/// `None` on a computer with no battery.
pub fn battery() -> Option<Battery> {
    let mut s = SYSTEM_POWER_STATUS::default();
    // SAFETY: the struct is valid for the call and Windows fills it.
    unsafe { GetSystemPowerStatus(&mut s).ok()? };
    // 128 means no battery at all; 255 means unknown.
    if s.BatteryFlag & 128 != 0 || s.BatteryLifePercent > 100 {
        return None;
    }
    Some(Battery {
        percent: s.BatteryLifePercent,
        plugged: s.ACLineStatus == 1,
        charging: s.BatteryFlag & 8 != 0,
        minutes_left: (s.BatteryLifeTime != u32::MAX).then_some(s.BatteryLifeTime / 60),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn asking_for_the_battery_does_not_fail() {
        // Either there is one (a laptop) or there is not; both are fine.
        if let Some(b) = super::battery() {
            assert!(b.percent <= 100);
        }
    }
}
