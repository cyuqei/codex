use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    pub turn: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConversationHistory {
    pub thread_id: String,
    pub messages: Vec<ConversationMessage>,
    pub turn_count: i32,
}

impl ConversationHistory {
    fn new(thread_id: &str) -> Self {
        Self {
            thread_id: thread_id.to_string(),
            messages: Vec::new(),
            turn_count: 0,
        }
    }

    pub fn add_message(&mut self, role: &str, content: &str) {
        self.turn_count += 1;
        self.messages.push(ConversationMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0),
            turn: self.turn_count,
        });
    }

    /// Format conversation history as a string for injecting into LLM prompts.
    pub fn format_as_context(&self, max_turns: usize) -> String {
        if self.messages.is_empty() {
            return "# 对话历史\n无（这是本轮对话的第一次交互）".to_string();
        }

        let recent = if self.messages.len() > max_turns {
            &self.messages[self.messages.len() - max_turns..]
        } else {
            &self.messages
        };

        let mut output = String::from("# 之前的对话历史（按时间顺序）\n\n");
        for msg in recent {
            let role_label = match msg.role.as_str() {
                "user" => "用户",
                "assistant" => "助手",
                "system" => "系统",
                _ => &msg.role,
            };
            output.push_str(&format!(
                "**Turn {} - {}:** {}\n\n",
                msg.turn, role_label, msg.content
            ));
        }

        output.push_str("---\n请在回复时考虑上述对话历史。\n");
        output
    }
}

/// Thread-safe conversation history manager.
///
/// Stores conversations in memory with optional disk persistence.
pub struct ConversationStore {
    conversations: Arc<RwLock<HashMap<String, ConversationHistory>>>,
    storage_dir: Option<PathBuf>,
}

impl ConversationStore {
    pub fn new(storage_dir: Option<PathBuf>) -> Self {
        let store = Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
        };

        // Load existing conversations from disk if storage_dir is set
        if let Some(ref dir) = store.storage_dir {
            store.load_from_disk(dir);
        }

        store
    }

    /// Get or create a conversation history for the given thread.
    pub fn get_or_create(&self, thread_id: &str) -> ConversationHistory {
        let guard = self.conversations.read().unwrap();
        if let Some(history) = guard.get(thread_id) {
            history.clone()
        } else {
            drop(guard);
            let _history = ConversationHistory::new(thread_id);
            let mut guard = self.conversations.write().unwrap();
            guard.entry(thread_id.to_string())
                .or_insert_with(|| ConversationHistory::new(thread_id))
                .clone()
        }
    }

    /// Add a message to the conversation and persist.
    pub fn add_message(&self, thread_id: &str, role: &str, content: &str) {
        let mut guard = self.conversations.write().unwrap();
        let history = guard
            .entry(thread_id.to_string())
            .or_insert_with(|| ConversationHistory::new(thread_id));
        history.add_message(role, content);

        if let Some(ref dir) = self.storage_dir {
            store_to_disk(dir, thread_id, history);
        }
    }

    /// Get formatted conversation context for LLM prompt injection.
    pub fn get_context(&self, thread_id: &str, max_turns: usize) -> Option<String> {
        let guard = self.conversations.read().unwrap();
        guard
            .get(thread_id)
            .map(|h| h.format_as_context(max_turns))
    }

    fn load_from_disk(&self, dir: &PathBuf) {
        if !dir.exists() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&entry.path()) {
                        if let Ok(history) =
                            serde_json::from_str::<ConversationHistory>(&content)
                        {
                            let mut guard = self.conversations.write().unwrap();
                            guard.insert(history.thread_id.clone(), history);
                        }
                    }
                }
            }
        }
    }

    /// Clear conversation history for a thread.
    pub fn clear(&self, thread_id: &str) {
        let mut guard = self.conversations.write().unwrap();
        if let Some(dir) = &self.storage_dir {
            let file_path = dir.join(format!("{}.json", thread_id));
            let _ = std::fs::remove_file(file_path);
        }
        guard.remove(thread_id);
    }
}

fn store_to_disk(dir: &PathBuf, thread_id: &str, history: &ConversationHistory) {
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("Failed to create conversation store dir: {}", e);
        return;
    }
    let file_path = dir.join(format!("{}.json", thread_id));
    if let Ok(json) = serde_json::to_string_pretty(history) {
        if let Err(e) = std::fs::write(&file_path, json) {
            eprintln!("Failed to save conversation {}: {}", thread_id, e);
        }
    }
}
