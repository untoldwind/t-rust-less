use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::ops;
use zeroize::Zeroize;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Ord, PartialOrd)]
#[serde(transparent)]
pub struct ZeroizeDateTime(DateTime<Utc>);

impl ZeroizeDateTime {
  pub fn timestamp_millis(&self) -> i64 {
    self.0.timestamp_millis()
  }

  pub fn format(&self, fmt: &str) -> String {
    self.0.format(fmt).to_string()
  }
}

impl Zeroize for ZeroizeDateTime {
  fn zeroize(&mut self) {
    self.0 = Utc.timestamp_millis(0)
  }
}

impl From<DateTime<Utc>> for ZeroizeDateTime {
  fn from(date_time: DateTime<Utc>) -> Self {
    ZeroizeDateTime(date_time)
  }
}

impl From<std::time::SystemTime> for ZeroizeDateTime {
  fn from(system_time: std::time::SystemTime) -> Self {
    ZeroizeDateTime(DateTime::from(system_time))
  }
}

impl<T: Into<ZeroizeDateTime>> ops::Sub<T> for ZeroizeDateTime {
  type Output = Duration;

  fn sub(self, rhs: T) -> Self::Output {
    self.0 - rhs.into().0
  }
}
