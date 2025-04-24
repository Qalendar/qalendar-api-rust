use serde::Serialize;
use crate::models::event::Event;     // Import the Event model
use crate::models::deadline::Deadline; // Import the Deadline model

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")] // Ensure camelCase output for JSON
pub struct UserCalendarResponse {
    pub events: Vec<Event>,
    pub deadlines: Vec<Deadline>,
}