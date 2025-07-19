//! Toy Notes Server - Library exports for testing
//!
//! This module exports the common types and implementations for testing.

use {
    anyhow::Result,
    async_trait::async_trait,
    solidmcp::{
        framework::ResourceProvider,
        handler::{ResourceContent, ResourceInfo},
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
