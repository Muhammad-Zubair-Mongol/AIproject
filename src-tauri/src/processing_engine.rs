use serde::{Deserialize, Serialize};
use serde_json::Value;

// GOD PROMPT V8 Schema Validator
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IntelligenceOutput {
    pub timestamp_ms: u64,
    pub speaker_id: String,
    pub transcript_chunk: String,
    pub is_final: bool,
    pub intelligence: Intelligence,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Intelligence {
    pub category: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone: Option<String>,
    pub confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entities: Option<Vec<Entity>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_updates: Option<Vec<GraphUpdate>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Entity {
    pub text: String,
    #[serde(rename = "type")]
    pub entity_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GraphUpdate {
    pub node_a: String,
    pub relation: String,
    pub node_b: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directional: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tone_modifier: Option<f32>,
}

// Validation functions
pub fn validate_intelligence_output(json: &str) -> Result<IntelligenceOutput, String> {
    serde_json::from_str::<IntelligenceOutput>(json)
        .map_err(|e| format!("Schema validation failed: {}", e))
}

pub fn validate_category(category: &[String]) -> bool {
    const VALID_CATEGORIES: &[&str] = &[
        "TASK", "DECISION", "DEADLINE", "QUERY", "ACTION_ITEM", "RISK",
        "SENTIMENT", "URGENCY", "INTERRUPTION", "AGREEMENT", "DISAGREEMENT",
        "OFF_TOPIC", "EMOTION_SHIFT", "DOMINANCE_SHIFT", "EMPATHY_GAP", "TOPIC_DRIFT"
    ];
    
    category.iter().all(|c| VALID_CATEGORIES.contains(&c.as_str()))
}

pub fn validate_tone(tone: &Option<String>) -> bool {
    const VALID_TONES: &[&str] = &[
        "URGENT", "FRUSTRATED", "EXCITED", "POSITIVE", "NEGATIVE",
        "HESITANT", "DOMINANT", "EMPATHETIC", "NEUTRAL"
    ];
    
    match tone {
        Some(t) => VALID_TONES.contains(&t.as_str()),
        None => true,
    }
}

// Graph State Management
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub weight: f32,
    pub directional: bool,
    pub tone_modifier: Option<f32>,
}

pub struct KnowledgeGraph {
    nodes: Arc<Mutex<HashMap<String, GraphNode>>>,
    edges: Arc<Mutex<Vec<GraphEdge>>>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(Mutex::new(HashMap::new())),
            edges: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn add_node(&self, id: String, node_type: String) {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.insert(id.clone(), GraphNode {
            id,
            node_type,
            metadata: HashMap::new(),
        });
    }

    pub fn add_edge(&self, update: GraphUpdate) {
        let mut edges = self.edges.lock().unwrap();
        edges.push(GraphEdge {
            from: update.node_a,
            to: update.node_b,
            relation: update.relation,
            weight: update.weight.unwrap_or(1.0),
            directional: update.directional.unwrap_or(true),
            tone_modifier: update.tone_modifier,
        });
    }

    pub fn get_graph_data(&self) -> (Vec<GraphNode>, Vec<GraphEdge>) {
        let nodes = self.nodes.lock().unwrap();
        let edges = self.edges.lock().unwrap();
        (
            nodes.values().cloned().collect(),
            edges.clone(),
        )
    }
}

// Optimistic Updates
pub struct OptimisticTranscript {
    pub partial_text: String,
    pub confidence: f32,
    pub timestamp: u64,
}

impl OptimisticTranscript {
    pub fn new(text: String) -> Self {
        Self {
            partial_text: text,
            confidence: 0.5, // Low confidence for optimistic updates
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub fn update(&mut self, new_text: String, confidence: f32) {
        self.partial_text = new_text;
        self.confidence = confidence;
    }
}

#[tauri::command]
pub fn validate_json_schema(json_str: String) -> Result<bool, String> {
    match validate_intelligence_output(&json_str) {
        Ok(output) => {
            // Additional validation
            if !validate_category(&output.intelligence.category) {
                return Err("Invalid category".to_string());
            }
            if !validate_tone(&output.intelligence.tone) {
                return Err("Invalid tone".to_string());
            }
            Ok(true)
        }
        Err(e) => Err(e),
    }
}
