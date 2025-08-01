//! Toy Notes Server - Library exports for testing
//!
//! This module exports the common types and implementations for testing.

use {
    anyhow::Result,
    async_trait::async_trait,
    serde_json::Value,
    solidmcp::{
        PromptProvider, ResourceProvider,
        PromptArgument, PromptContent, PromptInfo, PromptMessage, ResourceContent, ResourceInfo,
    },
    std::{collections::HashMap, fs, path::PathBuf, sync::Arc},
    tokio::sync::RwLock,
};

/// Custom context for our notes server
#[derive(Debug)]
pub struct NotesContext {
    notes_dir: PathBuf,
    notes: RwLock<HashMap<String, String>>,
}

impl NotesContext {
    pub fn new(notes_dir: PathBuf) -> Self {
        Self {
            notes_dir,
            notes: RwLock::new(HashMap::new()),
        }
    }

    pub async fn load_notes(&self) -> Result<()> {
        if !self.notes_dir.exists() {
            fs::create_dir_all(&self.notes_dir)?;
        }

        let mut notes = self.notes.write().await;
        for entry in fs::read_dir(&self.notes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let content = fs::read_to_string(&path)?;
                    notes.insert(name.to_string(), content);
                }
            }
        }
        Ok(())
    }

    pub async fn save_note(&self, name: &str, content: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        fs::write(&file_path, content)?;
        self.notes
            .write()
            .await
            .insert(name.to_string(), content.to_string());
        Ok(())
    }

    pub async fn get_note(&self, name: &str) -> Option<String> {
        self.notes.read().await.get(name).cloned()
    }

    pub async fn list_notes(&self) -> Vec<String> {
        self.notes.read().await.keys().cloned().collect()
    }

    pub async fn delete_note(&self, name: &str) -> Result<()> {
        let file_path = self.notes_dir.join(format!("{}.md", name));
        if file_path.exists() {
            fs::remove_file(&file_path)?;
        }
        self.notes.write().await.remove(name);
        Ok(())
    }
}

/// Resource provider for notes - exposes notes as MCP resources
pub struct NotesResourceProvider;

#[async_trait]
impl ResourceProvider<NotesContext> for NotesResourceProvider {
    async fn list_resources(&self, context: Arc<NotesContext>) -> Result<Vec<ResourceInfo>> {
        let notes = context.list_notes().await;
        let mut resources = Vec::new();

        for note_name in notes {
            resources.push(ResourceInfo {
                uri: format!("note://{}", note_name),
                name: note_name.clone(),
                description: Some(format!("Markdown note: {}", note_name)),
                mime_type: Some("text/markdown".to_string()),
            });
        }

        Ok(resources)
    }

    async fn read_resource(
        &self,
        uri: &str,
        context: Arc<NotesContext>,
    ) -> Result<ResourceContent> {
        if let Some(note_name) = uri.strip_prefix("note://") {
            if let Some(content) = context.get_note(note_name).await {
                return Ok(ResourceContent {
                    uri: uri.to_string(),
                    mime_type: Some("text/markdown".to_string()),
                    content,
                });
            }
        }
        Err(anyhow::anyhow!("Resource not found: {}", uri))
    }
}

/// Prompt provider for notes - provides note templates and formatting prompts
pub struct NotesPromptProvider;

#[async_trait]
impl PromptProvider<NotesContext> for NotesPromptProvider {
    async fn list_prompts(&self, _context: Arc<NotesContext>) -> Result<Vec<PromptInfo>> {
        Ok(vec![
            PromptInfo {
                name: "meeting_notes".to_string(),
                description: Some("Template for creating structured meeting notes".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "meeting_title".to_string(),
                        description: Some("The title of the meeting".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "attendees".to_string(),
                        description: Some("List of meeting attendees".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "task_note".to_string(),
                description: Some("Template for creating task/todo notes".to_string()),
                arguments: vec![
                    PromptArgument {
                        name: "task_name".to_string(),
                        description: Some("Name of the task".to_string()),
                        required: true,
                    },
                    PromptArgument {
                        name: "priority".to_string(),
                        description: Some("Task priority (high, medium, low)".to_string()),
                        required: false,
                    },
                    PromptArgument {
                        name: "due_date".to_string(),
                        description: Some("When the task is due".to_string()),
                        required: false,
                    },
                ],
            },
            PromptInfo {
                name: "daily_journal".to_string(),
                description: Some("Template for daily journal entries".to_string()),
                arguments: vec![PromptArgument {
                    name: "date".to_string(),
                    description: Some("Date for the journal entry".to_string()),
                    required: false,
                }],
            },
        ])
    }

    async fn get_prompt(
        &self,
        name: &str,
        arguments: Option<Value>,
        _context: Arc<NotesContext>,
    ) -> Result<PromptContent> {
        let args = arguments.unwrap_or_default();

        match name {
            "meeting_notes" => {
                let meeting_title = args["meeting_title"].as_str().unwrap_or("Meeting");
                let attendees = args["attendees"].as_str().unwrap_or("TBD");

                Ok(PromptContent {
                    messages: vec![PromptMessage {
                        role: "user".to_string(),
                        content: format!(
                            "# {}\n\n## Attendees\n{}\n\n## Agenda\n- \n\n## Discussion\n\n\n## Action Items\n- [ ] \n\n## Next Steps\n\n",
                            meeting_title, attendees
                        ),
                    }],
                })
            }
            "task_note" => {
                let task_name = args["task_name"].as_str().unwrap_or("New Task");
                let priority = args["priority"].as_str().unwrap_or("medium");
                let due_date = args["due_date"].as_str().unwrap_or("TBD");

                Ok(PromptContent {
                    messages: vec![PromptMessage {
                        role: "user".to_string(),
                        content: format!(
                            "# Task: {}\n\n**Priority**: {}\n**Due Date**: {}\n\n## Description\n\n\n## Requirements\n- \n\n## Progress\n- [ ] \n\n## Notes\n\n",
                            task_name, priority, due_date
                        ),
                    }],
                })
            }
            "daily_journal" => {
                let default_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
                let date = args["date"].as_str().unwrap_or(&default_date);

                Ok(PromptContent {
                    messages: vec![PromptMessage {
                        role: "user".to_string(),
                        content: format!(
                            "# Daily Journal - {}\n\n## How I'm Feeling\n\n\n## What Happened Today\n\n\n## Accomplishments\n- \n\n## Challenges\n\n\n## Tomorrow's Goals\n- \n\n## Gratitude\n- \n\n",
                            date
                        ),
                    }],
                })
            }
            _ => Err(anyhow::anyhow!("Prompt not found: {}", name)),
        }
    }
}
