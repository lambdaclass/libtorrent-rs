use super::stats_updater::StatsUpdater;
use crate::tracker_status::current_tracker_stats::CurrentTrackerStats;
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

/// Struct that represents the response of the stats request.
///
/// ## Fields
/// * `bucket_size_in_minutes`: The time interval in minutes of the bucket.
/// * `content`: A `Vec<CurrentTrackerStats>` containing the history of the stats.
#[derive(Debug, Serialize, Deserialize)]
pub struct StatsResponse {
    pub bucket_size_in_minutes: i64,
    pub content: Vec<CurrentTrackerStats>,
}

/// Posible stats request errors.
pub enum StatsResponseError {
    InvalidQueryParamError,
}

impl StatsResponse {
    /// Creates a new `StatsResponse` from the query parameters and a StatsUpdater. If the query parameters are invalid, an `InvalidQueryParamError` is returned.
    ///
    /// ## Returns
    /// * `Result<StatsResponse, StatsResponseError>`: The response of the stats request.
    pub fn from(
        query_params: HashMap<String, String>,
        stats_updater: Arc<StatsUpdater>,
    ) -> Result<Self, StatsResponseError> {
        let since_in_hours = query_params
            .get("since")
            .ok_or(StatsResponseError::InvalidQueryParamError)?
            .parse::<u64>()
            .map_err(|_| StatsResponseError::InvalidQueryParamError)?;

        let history = stats_updater.get_history(Duration::hours(since_in_hours as i64));

        Ok(Self {
            bucket_size_in_minutes: stats_updater.get_timeout().num_minutes(),
            content: history,
        })
    }
}
