use crate::db::{self, WorkflowData};
use anyhow::{anyhow, Result};
use arboard::Clipboard;
use dialoguer::FuzzySelect;

use super::args::HistoryCliArgs;

pub fn handle_history_command(args: HistoryCliArgs) -> Result<()> {
    let conn = db::init_db()?;

    // If copy flag is provided, copy that specific workflow to clipboard
    if let Some(id) = args.copy {
        return handle_copy_by_id(&conn, id);
    }

    // Check if no filters are specified (use interactive mode)
    let no_filters = args.query.is_none() && args.from.is_none() && args.to.is_none();

    if no_filters {
        // Enter interactive mode
        handle_interactive_mode(&conn, args.limit)
    } else {
        // Use non-interactive mode with search
        handle_non_interactive_mode(&conn, args)
    }
}

fn handle_copy_by_id(conn: &rusqlite::Connection, id: i64) -> Result<()> {
    let workflows = db::search_workflows(conn, None, None, None, 1000)?;

    if let Some(workflow) = workflows.iter().find(|w| w.id == Some(id)) {
        let text = match &workflow.data {
            WorkflowData::VoiceToText(data) => &data.text,
        };

        let mut clipboard =
            Clipboard::new().map_err(|e| anyhow!("Failed to initialize clipboard: {}", e))?;
        clipboard
            .set_text(text)
            .map_err(|e| anyhow!("Failed to copy to clipboard: {}", e))?;

        println!(
            "Copied transcription #{} to clipboard ({} chars)",
            id,
            text.len()
        );
        Ok(())
    } else {
        Err(anyhow!("Workflow with ID {} not found", id))
    }
}

fn handle_interactive_mode(conn: &rusqlite::Connection, limit: usize) -> Result<()> {
    // Fetch recent workflows
    let workflows = db::get_recent_workflows(conn, limit)?;

    if workflows.is_empty() {
        println!("No transcriptions found in history.");
        return Ok(());
    }

    // Create display items for FuzzySelect
    let items: Vec<String> = workflows
        .iter()
        .map(|workflow| {
            let id = workflow.id.unwrap_or(0);
            let created_at = workflow.created_at.as_deref().unwrap_or("Unknown");
            let text = match &workflow.data {
                WorkflowData::VoiceToText(data) => &data.text,
            };

            // Truncate long text for display
            let display_text = if text.len() > 80 {
                format!("{}...", &text[..80])
            } else {
                text.to_string()
            };

            format!("[{}] {} - {}", id, created_at, display_text)
        })
        .collect();

    // Show fuzzy select
    let selection = FuzzySelect::new()
        .with_prompt("Search and select a transcription to copy")
        .items(&items)
        .default(0)
        .interact_opt()?;

    // Handle selection
    if let Some(index) = selection {
        let workflow = &workflows[index];
        let text = match &workflow.data {
            WorkflowData::VoiceToText(data) => &data.text,
        };

        let mut clipboard =
            Clipboard::new().map_err(|e| anyhow!("Failed to initialize clipboard: {}", e))?;
        clipboard
            .set_text(text)
            .map_err(|e| anyhow!("Failed to copy to clipboard: {}", e))?;

        println!("\n✓ Copied to clipboard ({} chars)", text.len());
        println!("\nFull text:");
        println!("{}", text);
    } else {
        println!("Selection cancelled.");
    }

    Ok(())
}

fn handle_non_interactive_mode(conn: &rusqlite::Connection, args: HistoryCliArgs) -> Result<()> {
    let workflows = db::search_workflows(
        conn,
        args.query.as_deref(),
        args.from.as_deref(),
        args.to.as_deref(),
        args.limit,
    )?;

    if workflows.is_empty() {
        println!("No transcriptions found matching your criteria.");
        return Ok(());
    }

    println!("Found {} transcription(s):\n", workflows.len());

    for workflow in workflows {
        let id = workflow.id.unwrap_or(0);
        let created_at = workflow.created_at.as_deref().unwrap_or("Unknown");
        let text = match &workflow.data {
            WorkflowData::VoiceToText(data) => &data.text,
        };

        // Truncate long text for display
        let display_text = if text.len() > 100 {
            format!("{}...", &text[..100])
        } else {
            text.to_string()
        };

        println!("ID: {}", id);
        println!("Date: {}", created_at);
        println!("Text: {}", display_text);
        println!("---");
    }

    println!("\nTo copy a transcription to clipboard, use: audetic history --copy <ID>");

    Ok(())
}
