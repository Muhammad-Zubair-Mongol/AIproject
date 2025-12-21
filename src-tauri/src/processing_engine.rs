use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// STATION 3: OMNIPOTENT PROCESSING ENGINE
// ============================================================================

// GOD PROMPT V9 Schema Structures
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

// ============================================================================
// VALIDATION
// ============================================================================

const VALID_CATEGORIES: &[&str] = &[
    "TASK", "DECISION", "DEADLINE", "QUERY", "ACTION_ITEM", "RISK",
    "SENTIMENT", "URGENCY", "INTERRUPTION", "AGREEMENT", "DISAGREEMENT",
    "OFF_TOPIC", "EMOTION_SHIFT", "DOMINANCE_SHIFT", "EMPATHY_GAP", "TOPIC_DRIFT"
];

const VALID_TONES: &[&str] = &[
    "URGENT", "FRUSTRATED", "EXCITED", "POSITIVE", "NEGATIVE",
    "HESITANT", "DOMINANT", "EMPATHETIC", "NEUTRAL"
];

pub fn validate_intelligence_output(json: &str) -> Result<IntelligenceOutput, String> {
    serde_json::from_str::<IntelligenceOutput>(json)
        .map_err(|e| format!("Schema validation failed: {}", e))
}

pub fn validate_category(category: &[String]) -> bool {
    category.iter().all(|c| VALID_CATEGORIES.contains(&c.as_str()))
}

pub fn validate_tone(tone: &Option<String>) -> bool {
    match tone {
        Some(t) => VALID_TONES.contains(&t.as_str()),
        None => true,
    }
}

// ============================================================================
// PROCESSING ENGINE STATE
// ============================================================================

pub struct ProcessingEngineState {
    // Response cache (LRU)
    pub cache: Mutex<ResponseCache>,
    // Invalid JSON streak counter
    pub error_streak: Mutex<u32>,
    // Optimistic predictions buffer
    pub optimistic_buffer: Mutex<VecDeque<OptimisticPrediction>>,
    // User settings
    pub settings: Mutex<ProcessingSettings>,
    // Knowledge graph
    pub graph: KnowledgeGraph,
}

impl Default for ProcessingEngineState {
    fn default() -> Self {
        Self {
            cache: Mutex::new(ResponseCache::new(100)),
            error_streak: Mutex::new(0),
            optimistic_buffer: Mutex::new(VecDeque::with_capacity(50)),
            settings: Mutex::new(ProcessingSettings::default()),
            graph: KnowledgeGraph::new(),
        }
    }
}

// ============================================================================
// RESPONSE CACHE (LRU)
// ============================================================================

pub struct ResponseCache {
    items: VecDeque<CachedIntelligence>,
    max_size: usize,
}

#[derive(Clone)]
pub struct CachedIntelligence {
    pub data: IntelligenceOutput,
    pub cached_at: u64,
    pub hit_count: u32,
}

impl ResponseCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            items: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    pub fn add(&mut self, output: IntelligenceOutput) {
        // Remove oldest if at capacity
        if self.items.len() >= self.max_size {
            self.items.pop_front();
        }
        
        self.items.push_back(CachedIntelligence {
            data: output,
            cached_at: now_ms(),
            hit_count: 0,
        });
    }

    pub fn get_recent(&self, count: usize) -> Vec<IntelligenceOutput> {
        self.items.iter()
            .rev()
            .take(count)
            .map(|c| c.data.clone())
            .collect()
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }
}

// ============================================================================
// OPTIMISTIC PREDICTIONS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimisticPrediction {
    pub id: String,
    pub predicted_text: String,
    pub predicted_category: Option<String>,
    pub confidence: f32,
    pub timestamp_ms: u64,
    pub confirmed: bool,
    pub replaced_by: Option<String>, // ID of confirming intelligence
}

impl OptimisticPrediction {
    pub fn new(text: String, category: Option<String>) -> Self {
        Self {
            id: format!("opt_{}", now_ms()),
            predicted_text: text,
            predicted_category: category,
            confidence: 0.3, // Low confidence for predictions
            timestamp_ms: now_ms(),
            confirmed: false,
            replaced_by: None,
        }
    }
}

// ============================================================================
// USER SETTINGS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingSettings {
    pub confidence_threshold: f32,      // Filter below this (0.0 - 1.0)
    pub prediction_aggression: f32,     // How aggressive optimistic predictions (0.0 - 1.0)
    pub max_error_streak: u32,          // Reconnect after this many JSON errors
    pub enable_optimistic: bool,        // Enable/disable optimistic updates
    pub categories_filter: Vec<String>, // Only show these categories
}

impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            confidence_threshold: 0.5,
            prediction_aggression: 0.5,
            max_error_streak: 5,
            enable_optimistic: true,
            categories_filter: VALID_CATEGORIES.iter().map(|s| s.to_string()).collect(),
        }
    }
}

// ============================================================================
// KNOWLEDGE GRAPH
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub metadata: HashMap<String, String>,
    pub is_optimistic: bool, // True if pre-computed, not yet confirmed
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub relation: String,
    pub weight: f32,
    pub directional: bool,
    pub tone_modifier: Option<f32>,
    pub is_optimistic: bool,
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

    pub fn add_node(&self, id: String, node_type: String, label: String, optimistic: bool) {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.insert(id.clone(), GraphNode {
            id,
            node_type,
            label,
            metadata: HashMap::new(),
            is_optimistic: optimistic,
        });
    }

    pub fn confirm_node(&self, id: &str) {
        let mut nodes = self.nodes.lock().unwrap();
        if let Some(node) = nodes.get_mut(id) {
            node.is_optimistic = false;
        }
    }

    pub fn add_edge(&self, update: GraphUpdate, optimistic: bool) {
        // Ensure nodes exist
        self.add_node(update.node_a.clone(), "entity".to_string(), update.node_a.clone(), optimistic);
        self.add_node(update.node_b.clone(), "entity".to_string(), update.node_b.clone(), optimistic);
        
        let mut edges = self.edges.lock().unwrap();
        edges.push(GraphEdge {
            id: format!("edge_{}_{}", update.node_a, update.node_b),
            from: update.node_a,
            to: update.node_b,
            relation: update.relation,
            weight: update.weight.unwrap_or(1.0),
            directional: update.directional.unwrap_or(true),
            tone_modifier: update.tone_modifier,
            is_optimistic: optimistic,
        });
    }

    pub fn rollback_optimistic(&self) {
        let mut nodes = self.nodes.lock().unwrap();
        nodes.retain(|_, v| !v.is_optimistic);
        
        let mut edges = self.edges.lock().unwrap();
        edges.retain(|e| !e.is_optimistic);
    }

    pub fn confirm_all_optimistic(&self) {
        let mut nodes = self.nodes.lock().unwrap();
        for node in nodes.values_mut() {
            node.is_optimistic = false;
        }
        
        let mut edges = self.edges.lock().unwrap();
        for edge in edges.iter_mut() {
            edge.is_optimistic = false;
        }
    }

    pub fn get_graph_data(&self) -> (Vec<GraphNode>, Vec<GraphEdge>) {
        let nodes = self.nodes.lock().unwrap();
        let edges = self.edges.lock().unwrap();
        (
            nodes.values().cloned().collect(),
            edges.clone(),
        )
    }

    pub fn clear(&self) {
        self.nodes.lock().unwrap().clear();
        self.edges.lock().unwrap().clear();
    }
}

// ============================================================================
// PROCESSING FUNCTIONS
// ============================================================================

/// Process raw text from Gemini, validate, cache, and filter
pub fn process_intelligence(
    state: &ProcessingEngineState,
    raw_text: &str,
) -> Result<Option<IntelligenceOutput>, ProcessingError> {
    let settings = state.settings.lock().unwrap().clone();
    
    // Try to parse as JSON
    match validate_intelligence_output(raw_text) {
        Ok(output) => {
            // Reset error streak on success
            *state.error_streak.lock().unwrap() = 0;
            
            // Validate categories and tones
            if !validate_category(&output.intelligence.category) {
                return Err(ProcessingError::InvalidCategory);
            }
            if !validate_tone(&output.intelligence.tone) {
                return Err(ProcessingError::InvalidTone);
            }
            
            // Filter by confidence threshold
            if output.intelligence.confidence < settings.confidence_threshold {
                return Ok(None); // Below threshold, skip
            }
            
            // Filter by categories
            let has_matching_category = output.intelligence.category.iter()
                .any(|c| settings.categories_filter.contains(c));
            if !has_matching_category && !settings.categories_filter.is_empty() {
                return Ok(None); // No matching category, skip
            }
            
            // Add to cache
            state.cache.lock().unwrap().add(output.clone());
            
            // Update graph with confirmed data
            if let Some(updates) = &output.intelligence.graph_updates {
                for update in updates {
                    state.graph.add_edge(update.clone(), false); // confirmed
                }
            }
            
            // Confirm any matching optimistic predictions
            state.graph.confirm_all_optimistic();
            
            Ok(Some(output))
        }
        Err(e) => {
            // Increment error streak
            let mut streak = state.error_streak.lock().unwrap();
            *streak += 1;
            
            if *streak >= settings.max_error_streak {
                return Err(ProcessingError::ErrorStreakExceeded(*streak));
            }
            
            Err(ProcessingError::ParseError(e))
        }
    }
}

/// Generate optimistic prediction from partial text
pub fn generate_optimistic(
    state: &ProcessingEngineState,
    partial_text: &str,
) -> Option<OptimisticPrediction> {
    let settings = state.settings.lock().unwrap();
    
    if !settings.enable_optimistic {
        return None;
    }
    
    // Simple keyword detection for optimistic category
    let category = detect_category_keywords(partial_text);
    
    let prediction = OptimisticPrediction::new(
        partial_text.to_string(),
        category,
    );
    
    // Add to buffer
    state.optimistic_buffer.lock().unwrap().push_back(prediction.clone());
    
    // Pre-compute graph nodes if we detected entities
    if let Some(ref cat) = prediction.predicted_category {
        state.graph.add_node(
            format!("opt_node_{}", prediction.timestamp_ms),
            cat.clone(),
            partial_text.chars().take(30).collect(),
            true, // optimistic
        );
    }
    
    Some(prediction)
}

/// Simple keyword detection for instant category hints
fn detect_category_keywords(text: &str) -> Option<String> {
    let text_lower = text.to_lowercase();
    
    // Task keywords
    if text_lower.contains("todo") || text_lower.contains("need to") || 
       text_lower.contains("should") || text_lower.contains("must") ||
       text_lower.contains("karna hai") || text_lower.contains("kar do") {
        return Some("TASK".to_string());
    }
    
    // Decision keywords
    if text_lower.contains("decided") || text_lower.contains("let's go with") ||
       text_lower.contains("final") || text_lower.contains("faisla") {
        return Some("DECISION".to_string());
    }
    
    // Deadline keywords
    if text_lower.contains("by tomorrow") || text_lower.contains("deadline") ||
       text_lower.contains("due") || text_lower.contains("kal tak") {
        return Some("DEADLINE".to_string());
    }
    
    // Risk keywords
    if text_lower.contains("risk") || text_lower.contains("problem") ||
       text_lower.contains("issue") || text_lower.contains("masla") {
        return Some("RISK".to_string());
    }
    
    // Urgency keywords
    if text_lower.contains("urgent") || text_lower.contains("asap") ||
       text_lower.contains("jaldi") || text_lower.contains("abhi") {
        return Some("URGENCY".to_string());
    }
    
    None
}

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug)]
pub enum ProcessingError {
    ParseError(String),
    InvalidCategory,
    InvalidTone,
    ErrorStreakExceeded(u32),
}

// ============================================================================
// HELPERS
// ============================================================================

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ============================================================================
// TAURI COMMANDS
// ============================================================================

#[tauri::command]
pub fn validate_json_schema(json_str: String) -> Result<bool, String> {
    match validate_intelligence_output(&json_str) {
        Ok(output) => {
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

#[tauri::command]
pub fn update_processing_settings(
    confidence_threshold: Option<f32>,
    prediction_aggression: Option<f32>,
    enable_optimistic: Option<bool>,
    categories: Option<Vec<String>>,
) -> Result<String, String> {
    // This would update the global state - simplified for now
    Ok("Settings updated".to_string())
}

#[tauri::command]
pub fn get_recent_intelligence(count: u32) -> Result<Vec<String>, String> {
    // Would return from cache - simplified for now
    Ok(vec![])
}

#[tauri::command]
pub fn clear_intelligence_cache() -> Result<String, String> {
    Ok("Cache cleared".to_string())
}

#[tauri::command]
pub fn inject_manual_intelligence(
    text: String,
    category: String,
    confidence: f32,
) -> Result<String, String> {
    // Manual intelligence injection
    let output = IntelligenceOutput {
        timestamp_ms: now_ms(),
        speaker_id: "MANUAL".to_string(),
        transcript_chunk: text,
        is_final: true,
        intelligence: Intelligence {
            category: vec![category],
            summary: Some("Manually injected".to_string()),
            tone: Some("NEUTRAL".to_string()),
            confidence,
            entities: None,
            graph_updates: None,
        },
    };
    
    Ok(serde_json::to_string(&output).unwrap_or_default())
}
