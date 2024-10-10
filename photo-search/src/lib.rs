use chrono::{DateTime, FixedOffset, Utc};
use photogrid::PhotoLayoutData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Default)]
pub struct SearchFilter {
    pub before: Option<u64>,
    pub after: Option<u64>,
    pub rating: Option<u8>,
}

pub trait PhotoAccess {
    fn get_timestamp(&self) -> DateTime<Utc>;
    fn get_rating(&self) -> u8;
}

impl PhotoAccess for PhotoLayoutData {
    fn get_timestamp(&self) -> DateTime<Utc> {
        self.metadata
            .get("timestamp")
            .and_then(|x| x.parse::<DateTime<FixedOffset>>().ok())
            .map(|x| x.into())
            .unwrap_or_default()
    }

    fn get_rating(&self) -> u8 {
        self.metadata
            .get("rating")
            .and_then(|x| x.parse().ok())
            .unwrap_or_default()
    }
}

impl SearchFilter {
    pub fn matches(&self, photo_data: &PhotoLayoutData) -> bool {
        self.before
            .is_none_or(|timestamp| photo_data.get_timestamp().timestamp() <= timestamp as i64)
            && self
                .after
                .is_none_or(|timestamp| photo_data.get_timestamp().timestamp() >= timestamp as i64)
            && self
                .rating
                .is_none_or(|rating| photo_data.get_rating() >= rating)
    }
}
