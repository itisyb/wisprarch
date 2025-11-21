use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VoiceToTextData {
    pub text: String,
    pub audio_path: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WorkflowData {
    VoiceToText(VoiceToTextData),
    //will support more types later
}

#[derive(Debug)]
pub enum WorkflowType {
    VoiceToText,
}

impl WorkflowType {
    pub fn from_str(s: &str) -> Result<WorkflowType> {
        match s {
            "VoiceToText" => Ok(WorkflowType::VoiceToText),
            _ => Err(rusqlite::Error::InvalidQuery),
        }
    }
    pub fn to_str(&self) -> &str {
        match self {
            WorkflowType::VoiceToText => "VoiceToText",
        }
    }
}

#[derive(Debug)]
pub struct Workflow {
    WorkflowType: WorkflowType,
    data: WorkflowData,
}

impl Workflow {
    pub fn to_row(&self) -> Result<(String, String)> {
        Ok((
            self.WorkflowType.to_str().to_string(),
            serde_json::to_string(&self.data)?,
        ))
    }

    pub fn from_row(workflow_type: String, json: String) -> Result<Workflow> {
        Ok(Workflow {
            workflow_type: WorkflowType::from_str(&workflow_type)?,
            data: serde_json::from_str(&json)?,
        })
    }
}

// boostrap function will
// create a connection to the database
//  run migrations
// anything else?

pub fn migrate(conn: &Connection) -> Result<()> {
    // runs migrations ideompodently
}
