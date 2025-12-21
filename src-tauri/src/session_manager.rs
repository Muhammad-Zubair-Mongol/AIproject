use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub transcripts: Vec<TranscriptEntry>,
    pub graph_nodes: Vec<GraphNode>,
    pub graph_edges: Vec<GraphEdge>,
    pub metadata: SessionMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptEntry {
    pub timestamp: String,
    pub speaker_id: String,
    pub text: String,
    pub tone: Option<String>,
    pub category: Option<Vec<String>>,
    pub confidence: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub weight: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMetadata {
    pub title: String,
    pub duration_seconds: u64,
    pub total_transcripts: usize,
    pub total_speakers: usize,
    pub tags: Vec<String>,
}

impl SessionData {
    pub fn new(title: String) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            transcripts: Vec::new(),
            graph_nodes: Vec::new(),
            graph_edges: Vec::new(),
            metadata: SessionMetadata {
                title,
                duration_seconds: 0,
                total_transcripts: 0,
                total_speakers: 0,
                tags: Vec::new(),
            },
        }
    }

    pub fn add_transcript(&mut self, entry: TranscriptEntry) {
        self.transcripts.push(entry);
        self.metadata.total_transcripts = self.transcripts.len();
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn add_graph_node(&mut self, node: GraphNode) {
        self.graph_nodes.push(node);
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn add_graph_edge(&mut self, edge: GraphEdge) {
        self.graph_edges.push(edge);
        self.updated_at = Utc::now().to_rfc3339();
    }
}

// Session Manager
pub struct SessionManager {
    sessions_dir: PathBuf,
}

impl SessionManager {
    pub fn new() -> Result<Self, String> {
        let sessions_dir = dirs::data_local_dir()
            .ok_or("Could not find local data directory")?
            .join("GOD-V8")
            .join("sessions");

        fs::create_dir_all(&sessions_dir)
            .map_err(|e| format!("Failed to create sessions directory: {}", e))?;

        Ok(Self { sessions_dir })
    }

    pub fn save_session(&self, session: &SessionData) -> Result<String, String> {
        let filename = format!("{}.json", session.id);
        let filepath = self.sessions_dir.join(&filename);

        let json = serde_json::to_string_pretty(session)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;

        fs::write(&filepath, json)
            .map_err(|e| format!("Failed to write session file: {}", e))?;

        Ok(filepath.to_string_lossy().to_string())
    }

    pub fn load_session(&self, session_id: &str) -> Result<SessionData, String> {
        let filename = format!("{}.json", session_id);
        let filepath = self.sessions_dir.join(&filename);

        let json = fs::read_to_string(&filepath)
            .map_err(|e| format!("Failed to read session file: {}", e))?;

        serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize session: {}", e))
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionData>, String> {
        let entries = fs::read_dir(&self.sessions_dir)
            .map_err(|e| format!("Failed to read sessions directory: {}", e))?;

        let mut sessions = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                if let Some(ext) = entry.path().extension() {
                    if ext == "json" {
                        if let Ok(json) = fs::read_to_string(entry.path()) {
                            if let Ok(session) = serde_json::from_str::<SessionData>(&json) {
                                sessions.push(session);
                            }
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let filename = format!("{}.json", session_id);
        let filepath = self.sessions_dir.join(&filename);

        fs::remove_file(&filepath)
            .map_err(|e| format!("Failed to delete session: {}", e))
    }
}

// Export functionality
pub struct ExportManager;

impl ExportManager {
    pub fn export_to_json(session: &SessionData) -> Result<String, String> {
        serde_json::to_string_pretty(session)
            .map_err(|e| format!("Failed to export to JSON: {}", e))
    }

    pub fn export_to_csv(session: &SessionData) -> Result<String, String> {
        let mut csv = String::from("Timestamp,Speaker,Text,Tone,Categories,Confidence\n");
        
        for transcript in &session.transcripts {
            let categories = transcript.category.as_ref()
                .map(|c| c.join(";"))
                .unwrap_or_default();
            
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",{}\n",
                transcript.timestamp,
                transcript.speaker_id,
                transcript.text.replace("\"", "\"\""),
                transcript.tone.as_deref().unwrap_or(""),
                categories,
                transcript.confidence
            ));
        }
        
        Ok(csv)
    }

    pub fn export_to_markdown(session: &SessionData) -> Result<String, String> {
        let mut md = format!("# {}\n\n", session.metadata.title);
        md.push_str(&format!("**Session ID**: {}\n", session.id));
        md.push_str(&format!("**Created**: {}\n", session.created_at));
        md.push_str(&format!("**Duration**: {} seconds\n", session.metadata.duration_seconds));
        md.push_str(&format!("**Total Transcripts**: {}\n\n", session.metadata.total_transcripts));
        
        md.push_str("## Transcripts\n\n");
        for transcript in &session.transcripts {
            md.push_str(&format!("### {} - {}\n", transcript.timestamp, transcript.speaker_id));
            if let Some(tone) = &transcript.tone {
                md.push_str(&format!("**Tone**: {}\n", tone));
            }
            if let Some(categories) = &transcript.category {
                md.push_str(&format!("**Categories**: {}\n", categories.join(", ")));
            }
            md.push_str(&format!("\n{}\n\n", transcript.text));
        }
        
        md.push_str("## Knowledge Graph\n\n");
        md.push_str(&format!("**Nodes**: {}\n", session.graph_nodes.len()));
        md.push_str(&format!("**Edges**: {}\n\n", session.graph_edges.len()));
        
        Ok(md)
    }
}

// Tauri commands
#[tauri::command]
pub fn save_session(session_json: String) -> Result<String, String> {
    let session: SessionData = serde_json::from_str(&session_json)
        .map_err(|e| format!("Invalid session data: {}", e))?;
    
    let manager = SessionManager::new()?;
    manager.save_session(&session)
}

#[tauri::command]
pub fn load_session(session_id: String) -> Result<String, String> {
    let manager = SessionManager::new()?;
    let session = manager.load_session(&session_id)?;
    serde_json::to_string(&session)
        .map_err(|e| format!("Failed to serialize session: {}", e))
}

#[tauri::command]
pub fn list_sessions() -> Result<String, String> {
    let manager = SessionManager::new()?;
    let sessions = manager.list_sessions()?;
    serde_json::to_string(&sessions)
        .map_err(|e| format!("Failed to serialize sessions: {}", e))
}

#[tauri::command]
pub fn delete_session(session_id: String) -> Result<(), String> {
    let manager = SessionManager::new()?;
    manager.delete_session(&session_id)
}

#[tauri::command]
pub fn export_session(session_json: String, format: String) -> Result<String, String> {
    let session: SessionData = serde_json::from_str(&session_json)
        .map_err(|e| format!("Invalid session data: {}", e))?;
    
    match format.as_str() {
        "json" => ExportManager::export_to_json(&session),
        "csv" => ExportManager::export_to_csv(&session),
        "markdown" | "md" => ExportManager::export_to_markdown(&session),
        _ => Err(format!("Unsupported export format: {}", format)),
    }
}
