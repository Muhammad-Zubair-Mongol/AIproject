use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use std::collections::HashMap;

// ============================================================================
// STATION 5: COSMIC POST-PROCESSING & EMPIRE
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub transcripts: Vec<TranscriptEntry>,
    pub graph_nodes: Vec<GraphNode>,
    pub graph_edges: Vec<GraphEdge>,
    pub metadata: SessionMetadata,
    #[serde(default)]
    pub summary: Option<SessionSummary>,
    #[serde(default)]
    pub psychosomatic: Option<PsychosomaticState>,
    #[serde(default)]
    pub insights: Option<ExtractedInsights>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PsychosomaticState {
    pub stress: f32,
    pub engagement: f32,
    pub urgency: f32,
    pub clarity: f32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ExtractedInsights {
    pub topics: Vec<String>,
    pub decisions: Vec<String>,
    pub action_items: Vec<String>,
    pub key_points: Vec<String>,
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
    pub metadata: HashMap<String, String>,
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

// Station 5: Auto-generated summary
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SessionSummary {
    pub executive_summary: String,
    pub key_decisions: Vec<String>,
    pub action_items: Vec<ActionItem>,
    pub risks_identified: Vec<String>,
    pub next_steps: Vec<String>,
    pub generated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionItem {
    pub description: String,
    pub assignee: Option<String>,
    pub deadline: Option<String>,
    pub priority: String,
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
            summary: None,
            psychosomatic: None,
            insights: None,
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
    
    // Station 5: Generate local summary (without API)
    pub fn generate_local_summary(&mut self) {
        let mut decisions = Vec::new();
        let mut tasks = Vec::new();
        let mut risks = Vec::new();
        
        for t in &self.transcripts {
            if let Some(cats) = &t.category {
                for cat in cats {
                    match cat.as_str() {
                        "DECISION" => decisions.push(t.text.clone()),
                        "TASK" | "ACTION_ITEM" => tasks.push(ActionItem {
                            description: t.text.clone(),
                            assignee: Some(t.speaker_id.clone()),
                            deadline: None,
                            priority: "MEDIUM".to_string(),
                        }),
                        "RISK" => risks.push(t.text.clone()),
                        _ => {}
                    }
                }
            }
        }
        
        self.summary = Some(SessionSummary {
            executive_summary: format!(
                "Meeting with {} transcripts, {} entities discussed.",
                self.transcripts.len(),
                self.graph_nodes.len()
            ),
            key_decisions: decisions.into_iter().take(5).collect(),
            action_items: tasks.into_iter().take(10).collect(),
            risks_identified: risks.into_iter().take(5).collect(),
            next_steps: vec!["Review action items".to_string(), "Schedule follow-up".to_string()],
            generated_at: Utc::now().to_rfc3339(),
        });
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

        let tmp_filepath = filepath.with_extension("tmp");

        fs::write(&tmp_filepath, json)
            .map_err(|e| format!("Failed to write temp session file: {}", e))?;

        fs::rename(&tmp_filepath, &filepath)
            .map_err(|e| format!("Failed to commit session file (atomic rename): {}", e))?;

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

// ============================================================================
// EXPORT MANAGER - Station 5 Enhanced
// ============================================================================

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
        
        // Add summary if available
        if let Some(summary) = &session.summary {
            md.push_str("## Executive Summary\n\n");
            md.push_str(&format!("{}\n\n", summary.executive_summary));
            
            if !summary.key_decisions.is_empty() {
                md.push_str("### Key Decisions\n\n");
                for decision in &summary.key_decisions {
                    md.push_str(&format!("- {}\n", decision));
                }
                md.push_str("\n");
            }
            
            if !summary.action_items.is_empty() {
                md.push_str("### Action Items\n\n");
                for item in &summary.action_items {
                    md.push_str(&format!("- [ ] {} ({})\n", item.description, item.priority));
                }
                md.push_str("\n");
            }
            
            if !summary.risks_identified.is_empty() {
                md.push_str("### Risks Identified\n\n");
                for risk in &summary.risks_identified {
                    md.push_str(&format!("- ⚠️ {}\n", risk));
                }
                md.push_str("\n");
            }
        }
        
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
    
    // Station 5: GraphML Export
    pub fn export_to_graphml(session: &SessionData) -> Result<String, String> {
        let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<graphml xmlns="http://graphml.graphdrawing.org/xmlns">
  <key id="label" for="node" attr.name="label" attr.type="string"/>
  <key id="type" for="node" attr.name="type" attr.type="string"/>
  <key id="relation" for="edge" attr.name="relation" attr.type="string"/>
  <key id="weight" for="edge" attr.name="weight" attr.type="double"/>
  <graph id="G" edgedefault="directed">
"#);
        
        // Add nodes
        for node in &session.graph_nodes {
            xml.push_str(&format!(
                r#"    <node id="{}">
      <data key="label">{}</data>
      <data key="type">{}</data>
    </node>
"#,
                node.id,
                node.id,
                node.node_type
            ));
        }
        
        // Add edges
        for (i, edge) in session.graph_edges.iter().enumerate() {
            xml.push_str(&format!(
                r#"    <edge id="e{}" source="{}" target="{}">
      <data key="relation">{}</data>
      <data key="weight">{}</data>
    </edge>
"#,
                i,
                edge.from,
                edge.to,
                edge.relation,
                edge.weight
            ));
        }
        
        xml.push_str("  </graph>\n</graphml>");
        Ok(xml)
    }
    
    // Station 5: Entities CSV Export
    pub fn export_entities_csv(session: &SessionData) -> Result<String, String> {
        let mut csv = String::from("EntityID,Type,Label,Metadata\n");
        
        for node in &session.graph_nodes {
            let metadata_str = node.metadata.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(";");
            
            csv.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\"\n",
                node.id,
                node.node_type,
                node.id,
                metadata_str
            ));
        }
        
        Ok(csv)
    }
}

// ============================================================================
// WEBHOOK MANAGER - Station 5
// ============================================================================

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebhookConfig {
    pub url: String,
    pub events: Vec<String>, // "session_end", "task_detected", "risk_detected"
    pub headers: HashMap<String, String>,
    pub enabled: bool,
}

pub struct WebhookManager {
    configs: Vec<WebhookConfig>,
}

impl WebhookManager {
    pub fn new() -> Self {
        Self { configs: Vec::new() }
    }
    
    pub fn add_webhook(&mut self, config: WebhookConfig) {
        self.configs.push(config);
    }
    
    pub async fn trigger_webhook(&self, event: &str, payload: &serde_json::Value) {
        for config in &self.configs {
            if config.enabled && config.events.contains(&event.to_string()) {
                // In production, use reqwest to POST
                println!("[WEBHOOK] Would POST to {}: {:?}", config.url, payload);
            }
        }
    }
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

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
        "graphml" => ExportManager::export_to_graphml(&session),
        "entities" => ExportManager::export_entities_csv(&session),
        _ => Err(format!("Unsupported export format: {}", format)),
    }
}

#[tauri::command]
pub fn generate_session_summary(session_json: String) -> Result<String, String> {
    let mut session: SessionData = serde_json::from_str(&session_json)
        .map_err(|e| format!("Invalid session data: {}", e))?;
    
    session.generate_local_summary();
    
    serde_json::to_string(&session)
        .map_err(|e| format!("Failed to serialize session: {}", e))
}

#[tauri::command]
pub fn get_session_summary(session_json: String) -> Result<String, String> {
    let session: SessionData = serde_json::from_str(&session_json)
        .map_err(|e| format!("Invalid session data: {}", e))?;
    
    if let Some(summary) = session.summary {
        serde_json::to_string(&summary)
            .map_err(|e| format!("Failed to serialize summary: {}", e))
    } else {
        Ok("null".to_string())
    }
}
