use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::str::FromStr; // For parsing strings into enums

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

// Implement FromStr for EventInvitationStatus to parse query parameters like ?status=pending
impl FromStr for EventInvitationStatus {
    type Err = String; // Use String for simple error message

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(EventInvitationStatus::Pending),
            "accepted" => Ok(EventInvitationStatus::Accepted),
            "rejected" => Ok(EventInvitationStatus::Rejected),
            "maybe" => Ok(EventInvitationStatus::Maybe),
            _ => Err(format!("Invalid event invitation status: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Type, Serialize, Deserialize)]
#[sqlx(type_name = "share_privacy_level", rename_all = "lowercase")]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub enum SharePrivacyLevel {
    #[default]
    Full,
    Limited,
}
