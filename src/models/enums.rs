use serde::{Deserialize, Serialize};
use sqlx::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)] // sqlx::Type for database mapping
#[sqlx(type_name = "deadline_priority_level", rename_all = "lowercase")] // Match DB ENUM name and values
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum DeadlinePriorityLevel {
    #[default]
    Normal,
    Important,
    Urgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "workload_unit_type", rename_all = "lowercase")]
#[serde(rename_all = "camelCase")]
pub enum WorkloadUnitType {
    Minutes,
    Hours,
    Days,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "event_invitation_status", rename_all = "lowercase")]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum EventInvitationStatus {
    #[default]
    Pending,
    Accepted,
    Rejected,
    Maybe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "share_privacy_level", rename_all = "lowercase")]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum SharePrivacyLevel {
    #[default]
    FullDetails,
    BusyOnly,
}
