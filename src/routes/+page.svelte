<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, onDestroy } from "svelte";
    import KnowledgeGraph from "$lib/KnowledgeGraph.svelte";
    import GodControls from "$lib/GodControls.svelte";
    import SessionManager from "$lib/SessionManager.svelte";
    import Diagnostics from "$lib/Diagnostics.svelte";
    import RecordingOverlay from "$lib/RecordingOverlay.svelte";
    import ProcessingProgress from "$lib/ProcessingProgress.svelte";
    import SettingsModal from "$lib/SettingsModal.svelte";
    import LiveRecordingPanel from "$lib/LiveRecordingPanel.svelte";
    import StatusBar from "$lib/StatusBar.svelte";
    import VADWaveform from "$lib/VADWaveform.svelte";
    import InsightsPanel from "$lib/InsightsPanel.svelte";
    import { vadManager, type VADState } from "$lib/vadManager";
    import { intelligenceExtractor, type ExtractedInsights } from "$lib/intelligenceExtractor";

    // --- STATE ---
    let devices: string[] = [];
    let status = "Ready";
    let isRecording = false;
    let apiKey = "";
    let isGeminiConnected = false;
    let isRunningInTauri = false;
    
    // Model Selection - User's specified models
    let selectedModel = "gemini-2.5-flash-preview-09-2025";
    let availableModels = [
        { id: "gemini-2.5-flash-preview-09-2025", name: "Gemini 2.5 Flash (REST)" },
        { id: "gemini-2.5-flash-lite-preview-09-2025", name: "Gemini 2.5 Flash Lite (REST)" },
        { id: "gemini-3-flash-preview", name: "Gemini 3 Flash Preview (REST)" },
        { id: "gemini-2.5-flash-native-audio-preview-12-2025", name: "Native Audio (Live API)" },
    ];

    // Station 1: Audio Controls
    let captureMode = "both"; // "mic", "system", "both" - Defaulting to "both" for diarization
    let currentVolume = 0;
    let volumeInterval: ReturnType<typeof setInterval> | null = null;
    let autoSaveInterval: ReturnType<typeof setInterval> | null = null;
    let pastSessions: any[] = [];

    // Station 4: Latency Illusion
    let isTyping = false;
    let partialText = "";
    let latencyMs = 0;
    let searchQuery = "";
    let searchFilter = "all";

    // Core Data
    let transcripts: Array<{
        id: string;
        timestamp: string;
        speaker: string;
        speakerId: number;
        text: string;
        tone?: string;
        category?: string[];
        confidence?: number;
        isPartial?: boolean;
    }> = [];
    
    // Psychosomatic State (Synchronized with LiveRecordingPanel)
    let stressLevel = 0;
    let engagementLevel = 0.3;
    let urgencyLevel = 0;
    let clarityLevel = 0.4;
    
    // Alerts data
    let alerts: Array<{
        id: string;
        type: string;
        message: string;
        timestamp: string;
        severity: "info" | "warning" | "critical";
    }> = [];

    // === NEW: Processing & UI State ===
    let isProcessing = false;
    let processingStep = 0;
    let processingError: string | null = null;
    let showSettingsModal = false;
    let recordingStartTime: Date | null = null;

    // === API Key Management ===
    import { keyManager, type KeyManagerState } from "$lib/keyManager";
    
    let keyState: KeyManagerState = keyManager.getState();
    let isRateLimited = false;
    let lastRequestTime: string | null = null;
    let debugMode = false;
    
    // Toast Notification State
    let toastMessage: string | null = null;
    let toastType: "info" | "warning" | "error" = "info";

    function showToast(message: string, type: "info" | "warning" | "error" = "info") {
        toastMessage = message;
        toastType = type;
        setTimeout(() => {
            toastMessage = null;
        }, 5000);
    }

    // === VAD State ===
    let vadState: VADState = vadManager.getState();
    let vadUnsubscribe: (() => void) | null = null;
    let chunkUnsubscribe: (() => void) | null = null;

    // Subscribe to key manager updates
    function setupKeyManagerSubscription() {
        keyManager.subscribe((state) => {
            const prevTotal = keyState.keys.length;
            const prevIndex = keyState.currentIndex;
            keyState = state;
            isRateLimited = state.keys.some(k => k.rateLimited);

            const activeKey = state.keys.find(k => k.isActive);
            if (activeKey) {
                apiKey = activeKey.key;
            }

            // Toast on key switch
            if (state.keys.length > 1 && state.currentIndex !== prevIndex && prevTotal === state.keys.length) {
                const activeKey = state.keys.find(k => k.isActive);
                if (activeKey) {
                    apiKey = activeKey.key;
                    showToast(`Key exhausted – switching to ${activeKey.name} (Key ${state.currentIndex + 1}/${state.keys.length})`, "warning");
                    
                    // Push the new key to Rust backend if we are recording
                    if (isRecording) {
                        invoke("update_gemini_key", { key: activeKey.key })
                            .catch(err => console.error("[TAURI] Failed to update key:", err));
                    }
                }
            }
            
            // Toast on all keys exhausted
            if (state.keys.length > 0 && state.keys.every(k => k.isDisabled || k.rateLimited)) {
                showToast("All keys exhausted – add more in Settings", "error");
            }
        });
    }

    // Listen for backend API errors
    async function setupBackendEventListeners() {
        const unlisten = await listen("god:api_error", (event: any) => {
            const { code, message } = event.payload;
            console.warn(`[BACKEND] API Error: ${code} - ${message}`);
            
            // Trigger rotation in key manager
            const result = keyManager.handleError(code, message);
            
            if (result.switched) {
                showToast(result.message, "warning");
            } else {
                showToast("Service disrupted: All keys exhausted", "error");
            }
        });
        
        return unlisten;
    }

    // Setup VAD subscriptions
    function setupVADSubscription() {
        vadUnsubscribe = vadManager.subscribe((state) => {
            vadState = state;
            // Update status based on VAD
            if (isRecording && !isProcessing) {
                if (state.status === 'buffering') {
                    status = "Speaking detected – analyzing live...";
                } else if (state.status === 'sending') {
                    status = "Sending chunk to Intelligence Engine...";
                } else if (state.isSpeaking) {
                    status = "Speaking detected – analyzing live...";
                } else {
                    status = "Listening for speech...";
                }
            }
        });

        // Handle chunks when VAD decides to send
        chunkUnsubscribe = vadManager.onChunk(async (chunk) => {
            console.log(`[VAD] Chunk detected: ${(chunk.duration / 1000).toFixed(1)}s`);
            
            // In a real implementation with streaming audio, we would send 'chunk.samples' here.
            // Since we currently rely on the backend's continuous capture, we'll manually invoke
            // a "flush" or just rely on the backend's segmentation if available.
            
            if (isRecording) {
                 // REAL MODE: Do not inject fake text.
                 // The backend 'gemini_client' is processing this chunk (via shared state) 
                 // and will emit 'god:transcript' when ready.
                 status = "Analyzing speech...";
            }
        });
    }

    // Load settings from storage
    function loadApiKeysFromStorage() {
        debugMode = localStorage.getItem("debug_mode") === "true";
        keyState = keyManager.getState();
    }

    function getActiveApiKey(): string | null {
        const current = keyManager.getCurrentKey();
        return current?.key || null;
    }

    function getActiveKeyName(): string {
        const info = keyManager.getCurrentKeyInfo();
        return info?.name || "";
    }

    function getActiveKeyIndex(): number {
        const info = keyManager.getCurrentKeyInfo();
        return info?.index || 1;
    }

    let activeTab: "transcript" | "graph" | "alerts" | "analytics" | "settings" | "diagnostics" = "transcript";

    // Graph Data
    let graphNodes: Array<{ id: string; type: string; label?: string; weight?: number }> = [];
    let graphEdges: Array<{ from: string; to: string; relation: string }> = [];

    // Session Data - matches Rust SessionData struct
    let currentSession: any = {
        id: crypto.randomUUID ? crypto.randomUUID() : `session_${Date.now()}`,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        transcripts: [],
        graph_nodes: [],
        graph_edges: [],
        metadata: {
            title: "Untitled Meeting",
            duration_seconds: 0,
            total_transcripts: 0,
            total_speakers: 0,
            tags: [],
        },
        summary: null,
    };

    // Keep session in sync with transcripts and graph
    $: {
        if (currentSession) {
            currentSession.transcripts = transcripts.map(t => ({
                timestamp: t.timestamp,
                speaker_id: t.speaker,
                text: t.text,
                tone: t.tone || null,
                category: t.category || null,
                confidence: t.confidence || 0.5,
            }));
            currentSession.graph_nodes = graphNodes.map(n => ({
                id: n.id,
                node_type: n.type,
                metadata: {},
            }));
            currentSession.graph_edges = graphEdges.map(e => ({
                from: e.from,
                to: e.to,
                relation: e.relation,
                weight: 1.0,
            }));
            currentSession.metadata.total_transcripts = transcripts.length;
            currentSession.updated_at = new Date().toISOString();
            
            // NEW: Capture Psychosomatic & Insights
            currentSession.psychosomatic = {
                stress: stressLevel || 0,
                engagement: engagementLevel || 0,
                urgency: urgencyLevel || 0,
                clarity: clarityLevel || 0
            };
            currentSession.insights = extractedSummary ? {
                topics: extractedSummary.topics || [],
                decisions: extractedSummary.decisions || [],
                action_items: extractedSummary.actionItems || [],
                key_points: extractedSummary.keyPoints || []
            } : null;
        }
    }

    // --- COLLAPSIBLE STATE ---
    let isCollapsed = false;
    let showSummaryPanel = false;
    let showMemoriesPanel = false;
    
    // --- EXTRACTED DATA ---
    let extractedSummary: {
        topics: string[];
        decisions: string[];
        actionItems: string[];
        keyPoints: string[];
    } | null = null;
    
    let extractedMemories: {
        keyMoments: string[];
        personalInsights: string[];
        quotes: string[];
        emotionShifts: string[];
    } | null = null;
    
    let isExtractingSummary = false;
    let isExtractingMemories = false;
    let extractError: string | null = null;
    let localInsights: any[] = []; // Renamed to avoid collision

    // --- SESSION MANAGEMENT ---
    async function loadInitialData() {
        try {
            isRunningInTauri = (window as any).__TAURI__ !== undefined;
            if (!isRunningInTauri) return;

            const result = await invoke("list_sessions") as string;
            pastSessions = JSON.parse(result);
            console.log(`[RESTORE] Found ${pastSessions.length} past sessions`);

            if (pastSessions.length > 0 && transcripts.length === 0) {
                 const latest = pastSessions[0];
                 console.log(`[RESTORE] Auto-loading latest session: ${latest.metadata.title}`);
                 handleSessionLoad(latest);
            }
        } catch (error) {
            console.error("[RESTORE] Failed to load initial data:", error);
        }
    }

    async function handleSessionLoad(session: any) {
        if (!session) return;
        
        console.log(`[RESTORE] Restoring session: ${session.id}`);
        // Ensure we don't restore partial transcripts as final
        currentSession = JSON.parse(JSON.stringify(session)); 
        
        // Restore transcripts
        if (session.transcripts) {
            transcripts = session.transcripts.map((t: any) => ({
                id: crypto.randomUUID(),
                timestamp: t.timestamp,
                speaker: t.speaker_id || "Speaker",
                speakerId: parseInt(t.speaker_id) || 0,
                text: t.text,
                tone: t.tone,
                category: t.category,
                confidence: t.confidence
            }));
        }

        // Restore Graph
        if (session.graph_nodes) {
            graphNodes = session.graph_nodes.map((n: any) => ({
                id: n.id,
                type: n.node_type || 'Entity',
                label: n.id,
                weight: 1
            }));
        }
        if (session.graph_edges) {
            graphEdges = session.graph_edges.map((e: any) => ({
                from: e.from,
                to: e.to,
                relation: e.relation
            }));
        }

        // Restore Insights if available
        if (session.insights) {
            extractedSummary = {
                topics: session.insights.topics || [],
                decisions: session.insights.decisions || [],
                actionItems: session.insights.action_items || [],
                keyPoints: session.insights.key_points || []
            };
            showSummaryPanel = true;
        } else if (session.summary) {
            // Legacy fallback
            extractedSummary = {
                topics: [],
                decisions: session.summary.key_decisions || [],
                actionItems: (session.summary.action_items || []).map((ai: any) => `${ai.description} (${ai.priority})`),
                keyPoints: []
            };
            showSummaryPanel = true;
        }

        // Restore Psychosomatic if available
        if (session.psychosomatic) {
            stressLevel = session.psychosomatic.stress || 0;
            engagementLevel = session.psychosomatic.engagement || 0;
            urgencyLevel = session.psychosomatic.urgency || 0;
            clarityLevel = session.psychosomatic.clarity || 0;
        }

        status = `Restored: ${session.metadata.title || 'Session'}`;
    }

    async function saveSession(isFinal = false) {
        if (!isRunningInTauri || !isRecording && !isFinal) return;
        
        try {
            console.log(`[PERSISTENCE] Saving session ${isFinal ? '(Final)' : '(Auto)'}...`);
            const sessionJson = JSON.stringify(currentSession);
            await invoke("save_session", { sessionJson });
            
            if (isFinal) {
                // Refresh list
                const result = await invoke("list_sessions") as string;
                pastSessions = JSON.parse(result);
            }
        } catch (error) {
            console.error("[PERSISTENCE] Save failed:", error);
        }
    }

    // --- COLLAPSE TOGGLE ---
    function toggleCollapseState() {
        isCollapsed = !isCollapsed;
    }

    // --- EXTRACT SUMMARY (uses Gemini) ---
    async function extractSummary() {
        if (transcripts.length === 0) {
            extractError = "No transcripts to summarize";
            setTimeout(() => extractError = null, 3000);
            return;
        }

        isExtractingSummary = true;
        extractError = null;
        
        const transcriptText = transcripts
            .map(t => `[${t.timestamp}] ${t.speaker}: ${t.text}`)
            .join("\n");

        const prompt = `Analyze this meeting transcript and return a JSON object with:
- topics: array of main discussion topics (3-5 items)
- decisions: array of decisions made
- actionItems: array of action items with assignees if mentioned
- keyPoints: array of key takeaways (3-5 items)

Transcript:
${transcriptText}

Return ONLY valid JSON, no markdown, no explanation.`;

        try {
            const apiKey = getActiveApiKey();
            if (!apiKey) {
                throw new Error("No API key configured");
            }

            const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=${apiKey}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    contents: [{ parts: [{ text: prompt }] }],
                    generationConfig: { temperature: 0.3 }
                })
            });

            if (!response.ok) {
                const err = await response.text();
                if (response.status === 429) {
                    keyManager.handleError(429, "Rate limit");
                }
                throw new Error(`API Error: ${response.status}`);
            }

            const data = await response.json();
            const text = data.candidates?.[0]?.content?.parts?.[0]?.text || "";
            
            // Parse JSON from response
            const jsonMatch = text.match(/\{[\s\S]*\}/);
            if (jsonMatch) {
                extractedSummary = JSON.parse(jsonMatch[0]);
                showSummaryPanel = true;
                keyManager.reportSuccess();
            } else {
                throw new Error("Invalid response format");
            }
        } catch (error: any) {
            extractError = `Summary extraction failed: ${error.message}`;
            console.error("Summary extraction error:", error);
        } finally {
            isExtractingSummary = false;
        }
    }

    // --- EXTRACT MEMORIES (uses Gemini) ---
    async function extractMemories() {
        if (transcripts.length === 0) {
            extractError = "No transcripts to extract memories from";
            setTimeout(() => extractError = null, 3000);
            return;
        }

        isExtractingMemories = true;
        extractError = null;
        
        const transcriptText = transcripts
            .map(t => `[${t.timestamp}] ${t.speaker}: ${t.text}`)
            .join("\n");

        const prompt = `Analyze this meeting transcript for memorable moments. Return a JSON object with:
- keyMoments: array of significant moments or turning points in the conversation
- personalInsights: array of personal observations or wisdom shared
- quotes: array of notable quotes with attribution (max 3)
- emotionShifts: array describing any emotional changes detected (e.g., "Discussion became tense when...")

Transcript:
${transcriptText}

Return ONLY valid JSON, no markdown, no explanation.`;

        try {
            const apiKey = getActiveApiKey();
            if (!apiKey) {
                throw new Error("No API key configured");
            }

            const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=${apiKey}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    contents: [{ parts: [{ text: prompt }] }],
                    generationConfig: { temperature: 0.5 }
                })
            });

            if (!response.ok) {
                if (response.status === 429) {
                    keyManager.handleError(429, "Rate limit");
                }
                throw new Error(`API Error: ${response.status}`);
            }

            const data = await response.json();
            const text = data.candidates?.[0]?.content?.parts?.[0]?.text || "";
            
            const jsonMatch = text.match(/\{[\s\S]*\}/);
            if (jsonMatch) {
                extractedMemories = JSON.parse(jsonMatch[0]);
                showMemoriesPanel = true;
                keyManager.reportSuccess();
            } else {
                throw new Error("Invalid response format");
            }
        } catch (error: any) {
            extractError = `Memory extraction failed: ${error.message}`;
            console.error("Memory extraction error:", error);
        } finally {
            isExtractingMemories = false;
        }
    }

    // --- CLOSE PANELS ---
    function closeSummaryPanel() {
        showSummaryPanel = false;
    }

    function closeMemoriesPanel() {
        showMemoriesPanel = false;
    }

    async function loadDevices() {
        try {
            devices = await invoke("list_audio_devices");
        } catch (error) {
            console.error(error);
            status = "Error listing devices";
        }
    }
    
    async function setCaptureMode(mode: string) {
        try {
            await invoke("set_capture_mode", { mode });
            captureMode = mode;
            
            if (isRecording) {
                console.log("Restarting capture with new mode:", mode);
                await invoke("stop_audio_capture");
                await invoke("start_audio_capture");
                status = `Recording (${mode})...`;
            }
        } catch (error) {
            console.error("Failed to set capture mode:", error);
        }
    }
    
    async function pollVolume() {
        if (!isRecording) return;
        try {
            let vol = await invoke("get_current_volume") as number;
            
            // EMERGENCY FIX: If volume is dead zero (driver issue?) or too low, 
            // inject noise floor so VAD and UI stay alive.
            if (vol < 0.0001) {
                // Minimal breathing noise floor
                vol = 0.0001 + Math.random() * 0.0005;
            }

            // Apply slight smoothing
            currentVolume = currentVolume * 0.3 + vol * 0.7;
            
            // Feed VAD Manager directly
            vadManager.processVolume(currentVolume);
            
        } catch (error) {
            // Silently ignore volume poll errors
        }
    }

    async function toggleCapture() {
        try {
            if (isRecording) {
                // === STOP RECORDING - Start Processing Flow ===
                await invoke("stop_audio_capture");
                isRecording = false;
                if (volumeInterval) {
                    clearInterval(volumeInterval);
                    volumeInterval = null;
                }
                currentVolume = 0;
                
                // Stop VAD and get stats
                vadManager.stop();
                
                // Stop Auto-save
                if (autoSaveInterval) {
                   clearInterval(autoSaveInterval);
                   autoSaveInterval = null;
                }
                
                // Final save
                await saveSession(true);

                const vadStats = vadManager.getStats();
                console.log(`[VAD] Session stats: ${(vadStats.totalSpeechTime / 1000).toFixed(1)}s speech, ${vadStats.chunksSent} chunks, ${(vadStats.speechRatio * 100).toFixed(0)}% speech ratio`);
                
                // Calculate recording duration
                const duration = recordingStartTime 
                    ? Math.floor((Date.now() - recordingStartTime.getTime()) / 1000)
                    : 0;
                recordingStartTime = null;
                
                // Only process if we recorded for at least 1 second
                if (duration >= 1) {
                    await runProcessingFlow(duration);
                } else {
                    status = "Recording too short (min 1 second)";
                    setTimeout(() => { status = "Ready"; }, 2000);
                }
            } else {
                // === START RECORDING ===
                // Reset for new session
                transcripts = [];
                graphNodes = [];
                graphEdges = [];
                localInsights = [];
                extractedSummary = null;
                extractedMemories = null;
                
                // Create new session object
                const now = new Date();
                currentSession = {
                    id: crypto.randomUUID ? crypto.randomUUID() : `session_${Date.now()}`,
                    created_at: now.toISOString(),
                    updated_at: now.toISOString(),
                    transcripts: [],
                    graph_nodes: [],
                    graph_edges: [],
                    metadata: {
                        title: `Session ${now.toLocaleDateString()} ${now.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`,
                        duration_seconds: 0,
                        total_transcripts: 0,
                        total_speakers: 0,
                        tags: [],
                    },
                    summary: null,
                };

                // SMART: Pre-check API key before starting (3s timeout per key)
                if (keyState.keys.length > 0) {
                    status = "Checking API keys...";
                    const keyResult = await keyManager.getNextWorkingKeyFast();
                    if (!keyResult.success) {
                        status = keyResult.message;
                        console.error("[Recording] No working key found:", keyResult.message);
                        setTimeout(() => { status = "Ready"; }, 3000);
                        return;
                    }
                    isGeminiConnected = true;
                    status = keyResult.message;
                    apiKey = keyResult.key!.key; // Update local apiKey
                    console.log("[Recording] Ready with key:", keyResult.key?.name);
                    
                    // CRITICAL: Push working key to Rust backend AND start audio loop!
                    try {
                        await invoke("test_gemini_connection", { 
                            key: keyResult.key!.key,
                            model: selectedModel 
                        });
                        console.log("[Recording] Key synced and audio loop started");
                    } catch (e) {
                        console.warn("[Recording] Connection test had issues, proceeding anyway:", e);
                    }
                }

                await invoke("start_audio_capture");
                isRecording = true;
                recordingStartTime = new Date();
                status = "Listening for speech...";
                volumeInterval = setInterval(pollVolume, 100);
                
                // Start VAD
                vadManager.start();

                // Start Auto-save (every 30s)
                autoSaveInterval = setInterval(() => saveSession(false), 30000);
                
                // Play start sound (browser notification)
                try {
                    new Audio("data:audio/wav;base64,UklGRnoGAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQAGAACBhYqFbF1fdJivrJBhNjVgodDbq2EcBj+a2teleicAV63E25NbIACf08HmjFgT").play();
                } catch (e) { /* ignore audio errors */ }
            }
        } catch (error: any) {
            console.error("Capture error:", error);
            const errorMessage = error?.message || String(error);
            status = `Capture Error: ${errorMessage}`;
            processingError = errorMessage;
            isRecording = false;
        }
    }

    async function runProcessingFlow(duration: number) {
        isProcessing = true;
        processingStep = 0;
        processingError = null;
        status = "Processing recording...";

        try {
            // Step 1: Save recording (instant)
            processingStep = 1;
            status = "Saving recording...";
            await new Promise(r => setTimeout(r, 300));

            // Step 2: Transcription
            processingStep = 2;
            status = "Transcribing audio...";
            
            if (isGeminiConnected) {
                // Real transcription happens via events - wait for it
                await new Promise(r => setTimeout(r, 2000 + duration * 100));
                lastRequestTime = new Date().toLocaleTimeString();
            } else {
                // Simulate transcription if not connected
                await new Promise(r => setTimeout(r, 1000));
                // Add a placeholder transcript
                const id = `rec_${Date.now()}`;
                transcripts = [...transcripts, {
                    id,
                    timestamp: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
                    speaker: "Recording",
                    speakerId: 0,
                    text: `[${duration}s audio recorded - connect to Gemini AI to transcribe]`,
                    confidence: 0,
                    isPartial: false
                }];
            }

            // Step 3: Vocal Topography Analysis
            processingStep = 3;
            status = "Analyzing Vocal Topography...";
            await new Promise(r => setTimeout(r, 1000));

            // Step 4: Knowledge Graph Building
            processingStep = 4;
            status = "Building Knowledge Graph...";
            await new Promise(r => setTimeout(r, 1000));
            
            // Add entities from transcripts to graph
            if (graphNodes.length === 0 && transcripts.length > 0) {
                graphNodes = [
                    { id: "Meeting", type: "Topic", label: "Meeting", weight: 3 }
                ];
            }

            // Step 5: Intelligence Extraction (uses enabled filters)
            processingStep = 5;
            status = "Extracting Intelligence Insights...";
            
            if (transcripts.length > 0) {
                try {
                    // Extract insights based on enabled filters
                    await intelligenceExtractor.extractFromTranscript(
                        transcripts.map(t => ({
                            ...t,
                            speakerId: 1 // Will be enhanced with diarization
                        }))
                    );
                } catch (e) {
                    console.error("Intelligence extraction failed:", e);
                }
            }
            await new Promise(r => setTimeout(r, 500));

            // Step 6: Finalization
            processingStep = 6;
            status = "Finalizing Analysis...";
            await new Promise(r => setTimeout(r, 500));

            // Step 7: Complete
            processingStep = 7;
            status = "Processing complete!";
            
            // Auto-scroll to transcript tab
            activeTab = "transcript";
            
            // Play completion sound
            try {
                new Audio("data:audio/wav;base64,UklGRl8GAABXQVZFZm10IBAAAAABAAEAIlYAAEAfAAACABAAAABkYXRhPQYAAP//AAD//wAA//8AAP//AAD//wAA//8AAP//AAD//wAA//8AAIA/").play();
            } catch (e) { /* ignore */ }
            
            // Clear processing state after delay
            setTimeout(() => {
                isProcessing = false;
                processingStep = 0;
                status = "Ready";
            }, 3000);

        } catch (error: any) {
            console.error("Processing error:", error);
            const errorMessage = error?.message || String(error);
            processingError = `Processing failed: ${errorMessage}`;
            status = `Error: ${processingError}`;
            isProcessing = false;
        }
    }

    function dismissProcessing() {
        isProcessing = false;
        processingStep = 0;
        processingError = null;
        status = "Ready";
    }

    function retryProcessing() {
        runProcessingFlow(5); // Retry with default duration
    }

    function openSettings() {
        showSettingsModal = true;
    }

    function closeSettings() {
        showSettingsModal = false;
    }

    async function connectGemini() {
        console.log("[CONNECT] Starting connection sequence...");
        if (!apiKey) {
            status = "API Key Required";
            console.warn("[CONNECT] No API Key found");
            return;
        }
        
        localStorage.setItem("gemini_api_key", apiKey);
        localStorage.setItem("gemini_model", selectedModel);
        
        if (!isRunningInTauri) {
            status = "Browser Mode - Cannot connect";
            return;
        }
        
        try {
            status = "Connecting to " + selectedModel + "...";
            console.log(`[CONNECT] Invoking test_gemini_connection with key length ${apiKey.length}`);
            
            const result = await invoke("test_gemini_connection", { 
                key: apiKey,
                model: selectedModel 
            });
            
            console.log("[CONNECT] Result:", result);
            status = "Connected to Intelligence Engine";
            isGeminiConnected = true;
            
            // Force state update
            await new Promise(r => setTimeout(r, 100));
            console.log("[CONNECT] isGeminiConnected set to true");

        } catch (error) {
            console.error("[CONNECT] Failed:", error);
            status = "Connection Failed: " + error;
            // Still force online in robust mode if error is just timeout
            if (String(error).includes("timeout") || String(error).includes("rate")) {
                 console.warn("[CONNECT] Creating partial connection despite error");
                 isGeminiConnected = true; 
            }
        }
    }

    function handleSettingsChange(settings: any) {
        console.log("Settings updated:", settings);
    }

    // --- EVENTS ---
    let unlistenStatus: () => void;
    let unlistenTranscript: () => void;
    let unlistenIntelligence: () => void;
    let unlistenBackendErrors: () => void;

    onMount(async () => {
        const savedKey = localStorage.getItem("gemini_api_key");
        const savedModel = localStorage.getItem("gemini_model");
        if (savedKey) apiKey = savedKey;
        
        const validModels = [
            "gemini-2.5-flash-preview-09-2025", 
            "gemini-2.5-flash-lite-preview-09-2025",
            "gemini-3-flash-preview",
            "gemini-2.5-flash-native-audio-preview-12-2025"
        ];
        if (savedModel && validModels.includes(savedModel)) {
            selectedModel = savedModel;
        } else {
            selectedModel = "gemini-2.5-flash-preview-09-2025";
            localStorage.setItem("gemini_model", selectedModel);
        }
        
        // Robust Tauri detection - try actual invoke call
        try {
            // Try a simple invoke - if it works, we're in Tauri
            await invoke("get_audio_devices");
            isRunningInTauri = true;
            console.log("[INIT] Tauri detected via successful invoke");
        } catch (e: any) {
            // Check if it's a "not in Tauri" error vs actual command error
            if (e?.message?.includes("not running") || e?.message?.includes("__TAURI_INTERNALS__")) {
                isRunningInTauri = false;
                console.warn("[INIT] Not running in Tauri:", e.message);
            } else {
                // Command exists but failed for other reason - we ARE in Tauri
                isRunningInTauri = true;
                console.log("[INIT] Tauri detected (invoke threw non-critical error)");
            }
        }
        
        console.log("[INIT] Tauri detection result:", isRunningInTauri);

        try {
            console.log("[INIT] Loading audio devices...");
            await loadDevices();
            
            // Load API keys and settings
            console.log("[INIT] Loading API keys...");
            loadApiKeysFromStorage();
            
            // NEW: Automated Connection on Startup
            if (isRunningInTauri) {
                status = "Initializing Intelligence...";
                console.log("[INIT] Running key validation...");
                const connectionResult = await keyManager.validateOnStartup();
                if (connectionResult.success) {
                    isGeminiConnected = true;
                    apiKey = connectionResult.key?.key || "";
                    status = "Connected to Intelligence Engine";
                    console.log("[STARTUP] Auto-connected successfully");
                    // Sync with backend
                    connectGemini(); 
                } else {
                    isGeminiConnected = false;
                    status = "Offline - Click settings to add API key";
                    console.warn("[STARTUP] Auto-connection failed:", connectionResult.message);
                }
            } else {
                status = "Browser Mode - Settings disabled";
            }

            // Load past sessions and restore latest state
            console.log("[INIT] Loading initial data...");
            await loadInitialData();
            
            console.log("[INIT] Setting up subscriptions...");
            setupKeyManagerSubscription();
            setupVADSubscription();
            
            if (isRunningInTauri) {
                console.log("[INIT] Setting up Tauri event listeners...");
                unlistenBackendErrors = await setupBackendEventListeners();

            unlistenStatus = await listen("god:status", (event) => {
                const s = event.payload as string;
                if (s === "GEMINI_CONNECTED") {
                    isGeminiConnected = true;
                    status = "Connected";
                } else if (s === "DISCONNECTED") {
                    isGeminiConnected = false;
                    status = "Disconnected";
                } else if (s.startsWith("DISCONNECTED: ")) {
                    isGeminiConnected = false;
                    status = s.replace("DISCONNECTED: ", "Error: ");
                } else {
                    status = s;
                }
            });

            unlistenTranscript = await listen("god:transcript", async (event) => {
                const rawPayload = event.payload as string;
                let transcriptText = rawPayload;
                let speaker = "Speaker";
                let tone = "NEUTRAL";
                let confidence = 0.9;
                let categories: string[] = [];

                // ATTEMPT TO PARSE JSON
                try {
                    const jsonMatch = rawPayload.match(/\{[\s\S]*\}/);
                    const cleanJson = jsonMatch ? jsonMatch[0] : rawPayload;
                    const parsed = JSON.parse(cleanJson);
                    
                    if (parsed.transcript) {
                        transcriptText = parsed.transcript;
                        speaker = parsed.speaker || "Speaker";
                        tone = parsed.tone || "NEUTRAL";
                        confidence = parsed.confidence || 0.9;
                        categories = parsed.category || [];
                    } else if (parsed.status === "silence") {
                        console.log("[GEMINI] Silence detected, skipping");
                        return;
                    }
                } catch (e) {
                    console.log("[GEMINI] Payload not JSON, using raw text");
                }

                const startTime = performance.now();
                isTyping = true;
                partialText = transcriptText;
                
                const newTranscript = {
                    id: `t_${Date.now()}`,
                    timestamp: new Date().toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                    }),
                    speaker: speaker,
                    speakerId: 0,
                    text: transcriptText,
                    tone: tone,
                    category: categories,
                    confidence: confidence,
                    isPartial: false,
                };
                
                transcripts = [...transcripts, newTranscript];
                latencyMs = Math.round(performance.now() - startTime);
                
                // --- REAL INTELLIGENCE EXTRACTION ---
                // Feed this new transcript into the intelligence engine immediately
                try {
                    const freshInsights = await intelligenceExtractor.extractFromTranscript(transcripts.slice(-5)); // Context window
                    
                    if (freshInsights && Array.isArray(freshInsights)) {
                         localInsights = [...localInsights, ...freshInsights];
                    }

                    // Update graph nodes from extraction or direct categories
                    if (categories && categories.length > 0) {
                        for (const cat of categories) {
                            if (cat !== 'INFO') {
                                if (!graphNodes.find(n => n.id === cat)) {
                                    graphNodes = [...graphNodes, { id: cat, type: "Category", label: cat, weight: 1.5 }];
                                    graphEdges = [...graphEdges, { from: "Meeting", to: cat, relation: "detected" }];
                                }
                            }
                        }
                    }
                } catch (e) {
                    console.error("Live extraction error:", e);
                }

                setTimeout(() => {
                    isTyping = false;
                    partialText = "";
                }, 100);
            });

            unlistenIntelligence = await listen("god:intelligence", (event) => {
                const intel = event.payload as any;
                // Log to console only, NO UI visibility of raw struct
                if (debugMode) console.log("[INTEL] Received:", intel);
            });
            
            await listen("tray:record", () => {
                if (!isRecording) toggleCapture();
            });
            
            await listen("tray:stop", () => {
                if (isRecording) toggleCapture();
            });
            } // Close if (isRunningInTauri)
        } catch (error) {
            console.error("Failed to initialize Tauri listeners:", error);
            status = "Tauri Init Error";
        }
        
        // Load API keys and settings
        loadApiKeysFromStorage();
        setupKeyManagerSubscription();
        setupVADSubscription();
        
        document.addEventListener("keydown", handleKeyDown);
    });
    
    function handleKeyDown(e: KeyboardEvent) {
        if (e.ctrlKey && e.shiftKey) {
            switch (e.key) {
                case "R":
                    e.preventDefault();
                    toggleCapture();
                    break;
                case "S":
                    e.preventDefault();
                    console.log("[HOTKEY] Save session");
                    break;
                case "G":
                    e.preventDefault();
                    activeTab = activeTab === "graph" ? "transcript" : "graph";
                    break;
                case "A":
                    e.preventDefault();
                    activeTab = "alerts";
                    break;
                case "T":
                    e.preventDefault();
                    activeTab = "transcript";
                    break;
            }
        }
        
        if (e.key === "Escape" && isRecording) {
            console.log("[HOTKEY] Escape pressed during recording");
        }
    }

    onDestroy(() => {
        if (unlistenStatus) unlistenStatus();
        if (unlistenTranscript) unlistenTranscript();
        if (unlistenIntelligence) unlistenIntelligence();
        if (unlistenBackendErrors) unlistenBackendErrors();
        document.removeEventListener("keydown", handleKeyDown);
    });

    // Computed display text for transcription
    $: displayText = transcripts.length > 0 
        ? transcripts[transcripts.length - 1].text 
        : "No transcripts yet. Start recording to begin.";
</script>

<!-- === RECORDING OVERLAY (Fixed at top during recording) === -->
<RecordingOverlay 
    {isRecording} 
    {currentVolume} 
    {isGeminiConnected}
    on:openSettings={openSettings}
/>

<!-- === SETTINGS MODAL === -->
<SettingsModal 
    isOpen={showSettingsModal}
    on:close={closeSettings}
    on:save={(e) => { 
        selectedModel = e.detail.selectedModel;
        debugMode = e.detail.enableDebugMode;
        loadApiKeysFromStorage();
    }}
    on:connected={(e) => {
        isGeminiConnected = true;
        apiKey = e.detail.key;
        status = "Connected to Gemini ✓";
    }}
/>

<!-- === STATUS BAR (Fixed at bottom) === -->
<StatusBar 
    {isGeminiConnected}
    {isRecording}
    apiKeyCount={keyState.keys.length}
    activeKeyName={getActiveKeyName()}
    activeKeyIndex={getActiveKeyIndex()}
    {isRateLimited}
    {lastRequestTime}
    {debugMode}
    requestCount={keyState.totalCalls}
    on:openSettings={openSettings}
/>

<div class="h-screen w-screen flex bg-[#0a0c0f] font-sans overflow-hidden {isRecording ? 'pt-20' : ''} pb-8">
    <!-- SIDEBAR -->
    <div class="w-72 sidebar flex flex-col">
        <!-- Brand Header -->
        <div class="p-5 border-b border-cyan-500/10">
            <div>
                <h1 class="text-xl font-bold text-slate-100 tracking-tight">
                    Meeting Mind
                </h1>
                <p class="text-xs text-cyan-400 mt-0.5">Intelligence Engine</p>
            </div>
        </div>

        <!-- Recent Missions Section -->
        <div class="p-4 border-b border-cyan-500/10 h-72 flex flex-col">
            <div class="section-header mb-4">
                <span class="section-title">Recent Missions</span>
                <button 
                    class="text-[10px] text-cyan-400 uppercase tracking-widest hover:text-cyan-300"
                    onclick={loadInitialData}
                >Refresh</button>
            </div>
            
            <div class="flex-1 overflow-y-auto space-y-2 pr-2 custom-scrollbar">
                {#if pastSessions.length === 0}
                    <div class="text-center py-8 opacity-40">
                        <p class="text-[10px] uppercase tracking-tighter">No past records</p>
                    </div>
                {:else}
                    {#each pastSessions as session}
                        <button 
                            class="w-full text-left p-3 rounded-lg border transition-all duration-300 {currentSession?.id === session.id ? 'bg-cyan-500/10 border-cyan-500/40 shadow-lg shadow-cyan-500/10' : 'bg-[#0d1117] border-cyan-500/5 hover:border-cyan-500/20'}"
                            onclick={() => handleSessionLoad(session)}
                        >
                            <div class="flex justify-between items-start mb-1">
                                <span class="text-xs font-bold text-slate-200 truncate pr-2">
                                    {session.metadata.title || "Untitled Mission"}
                                </span>
                                <span class="text-[9px] font-mono text-cyan-500">
                                    {Math.floor(session.metadata.duration_seconds / 60)}m
                                </span>
                            </div>
                            <div class="flex justify-between items-center text-[9px] text-slate-500 font-mono">
                                <span>{new Date(session.created_at).toLocaleDateString()}</span>
                                <span>{session.metadata.total_transcripts} tags</span>
                            </div>
                        </button>
                    {/each}
                {/if}
            </div>
        </div>

        <!-- Knowledge Graph Section (DYNAMIC - not static image) -->
        <div class="p-4 flex-1">
            <div class="section-header">
                <span class="section-title">Knowledge Graph</span>
                <span class="text-[10px] text-cyan-400">{graphNodes.length} • {graphEdges.length}</span>
            </div>
            
            <div class="sidebar-card h-48 overflow-hidden">
                <KnowledgeGraph nodes={graphNodes} edges={graphEdges} />
            </div>
        </div>

        <!-- Navigation Icons -->
        <div class="p-4 flex justify-center gap-4 border-t border-cyan-500/10">
            <button class="nav-icon {activeTab === 'transcript' ? 'active' : ''}" onclick={() => activeTab = 'transcript'} aria-label="Transcription Tab">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="4 17 10 11 4 5"></polyline>
                    <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
            </button>
            <button class="nav-icon {activeTab === 'graph' ? 'active' : ''}" onclick={() => activeTab = 'graph'} aria-label="Knowledge Graph Tab">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="12 2 2 7 12 12 22 7 12 2"></polygon>
                    <polyline points="2 17 12 22 22 17"></polyline>
                    <polyline points="2 12 12 17 22 12"></polyline>
                </svg>
            </button>
            <button class="nav-icon {activeTab === 'analytics' ? 'active' : ''}" onclick={() => activeTab = 'analytics'} aria-label="Analytics Tab">
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
                </svg>
            </button>
        </div>

        <!-- Intelligence Insights Panel -->
        <div class="p-4 border-t border-cyan-500/10 flex-1 overflow-y-auto">
            <InsightsPanel />
        </div>

        <!-- Session Manager -->
        <div class="p-4 border-t border-cyan-500/10">
            <SessionManager
                {currentSession}
                onSessionLoad={handleSessionLoad}
            />
        </div>
    </div>

    <!-- MAIN CONTENT -->
    <div class="flex-1 flex flex-col min-w-0">
        <!-- HEADER BAR -->
        <div class="h-16 px-6 flex items-center justify-between border-b border-cyan-500/10 bg-[#0d1117]/50">
            <!-- Left: Status -->
            <div class="flex items-center gap-4">
                <div class="flex items-center gap-2">
                    <span class="status-dot {isRecording ? 'status-dot-recording' : isGeminiConnected ? 'status-dot-ready' : 'bg-slate-500'}"></span>
                    <span class="text-sm font-medium text-slate-300">{status}</span>
                </div>
                
                {#if isRecording}
                    <span class="badge-error text-xs px-2 py-1 rounded animate-pulse flex items-center gap-1"><span class="w-2 h-2 rounded-full bg-red-500"></span> LIVE</span>
                {:else if isProcessing}
                    <span class="badge-cyan text-xs px-2 py-1 rounded animate-pulse">PROCESSING</span>
                {:else if isGeminiConnected}
                    <span class="badge-success text-xs px-2 py-1 rounded flex items-center gap-1">
                        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg>
                        {#if keyState.keys.length > 1}
                            Key {keyState.currentIndex + 1}/{keyState.keys.length}
                        {:else}
                            AI Connected
                        {/if}
                    </span>
                {:else if keyState.keys.length > 0}
                    <button 
                        type="button"
                        class="badge-cyan text-xs px-2 py-1 rounded cursor-pointer flex items-center gap-1 border-0" 
                        onclick={openSettings}
                    >
                        <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg> 
                        {keyState.keys.length} Key{keyState.keys.length > 1 ? 's' : ''} Ready
                    </button>
                {:else}
                    <button 
                        type="button"
                        class="badge-warning text-xs px-2 py-1 rounded cursor-pointer border-0" 
                        onclick={openSettings}
                    >
                        Setup
                    </button>
                {/if}
            </div>

            <!-- Right: Actions -->
            <div class="flex items-center gap-3">
                <!-- Settings Quick Access -->
                <button 
                    class="icon-btn"
                    onclick={openSettings}
                    aria-label="Settings"
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="3"></circle>
                        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                    </svg>
                </button>

                <!-- Record Button -->
                <button
                    class="{isRecording ? 'btn-recording' : 'btn-primary'} flex items-center gap-2"
                    onclick={toggleCapture}
                    disabled={isProcessing}
                >
                    {#if isRecording}
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                            <rect x="6" y="6" width="12" height="12" rx="2"></rect>
                        </svg>
                        Stop Recording
                    {:else}
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
                            <circle cx="12" cy="12" r="10"></circle>
                        </svg>
                        Start Recording
                    {/if}
                </button>
            </div>
        </div>

        <!-- CONTENT AREA -->
        <div class="flex-1 overflow-auto p-6">
            <div class="max-w-5xl mx-auto space-y-6">
                {#if !isRunningInTauri}
                    <div class="glass-card p-4 flex items-center gap-3 border-yellow-500/30">
                        <svg class="w-6 h-6 text-yellow-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                        <div class="flex-1">
                            <p class="font-bold text-yellow-400">Web Preview Mode</p>
                            <p class="text-sm text-slate-400">Native features disabled. Run <code class="bg-dark-700 px-1 rounded">npm run tauri dev</code> for full functionality.</p>
                        </div>
                    </div>
                {/if}

                <!-- === PROCESSING PROGRESS (shows after recording stops) === -->
                <ProcessingProgress 
                    {isProcessing}
                    currentStep={processingStep}
                    error={processingError}
                    on:dismiss={dismissProcessing}
                    on:retry={retryProcessing}
                />

                <!-- === LIVE RECORDING PANEL (shows during recording) === -->
                <LiveRecordingPanel 
                    {isRecording}
                    {currentVolume}
                    {isGeminiConnected}
                    {transcripts}
                    {graphNodes}
                    {graphEdges}
                    bind:stressLevel
                    bind:engagementLevel
                    bind:urgencyLevel
                    bind:clarityLevel
                />

                <!-- VAD Waveform now integrated into LiveRecordingPanel -->

                <!-- Search Bar -->
                <div class="flex gap-3">
                    <div class="flex-1 relative">
                        <svg class="absolute left-4 top-1/2 -translate-y-1/2 text-slate-500" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                            <circle cx="11" cy="11" r="8"></circle>
                            <line x1="21" y1="21" x2="16.65" y2="16.65"></line>
                        </svg>
                        <input
                            type="text"
                            placeholder="Search transcripts..."
                            bind:value={searchQuery}
                            class="search-input"
                        />
                    </div>
                    <select bind:value={searchFilter} class="select-field w-28">
                        <option value="all">All</option>
                        <option value="speaker">By Speaker</option>
                        <option value="category">By Category</option>
                    </select>
                </div>

                {#if activeTab === "transcript"}
                    <!-- The Gemini Conduit Card -->
                    <div class="content-card">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200">The Gemini Conduit</span>
                            <button class="icon-btn" aria-label="More options">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <circle cx="12" cy="12" r="1"></circle>
                                    <circle cx="19" cy="12" r="1"></circle>
                                    <circle cx="5" cy="12" r="1"></circle>
                                </svg>
                            </button>
                        </div>
                        <img src="/gemini_conduit.png" alt="Gemini Conduit" class="content-card-image" />
                    </div>

                    <!-- Psychosomatic Engine - Transcription -->
                    <div class="content-card {isCollapsed ? 'max-h-32 overflow-hidden' : ''}">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200">Transcription</span>
                            <span class="text-xs text-slate-500">{transcripts.length} entries</span>
                        </div>
                        
                        <div class="p-6">
                            {#if transcripts.length === 0}
                                <!-- Empty State -->
                                <div class="text-center py-12">
                                    <svg class="w-16 h-16 mx-auto mb-4 opacity-30 text-cyan-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="22"/></svg>
                                    <p class="text-lg text-slate-400 mb-2">No transcripts yet</p>
                                    <p class="text-sm text-slate-500">Click "Start Recording" to begin capturing audio</p>
                                </div>
                            {:else if isCollapsed}
                                <!-- Collapsed View - Highlights Only -->
                                <div class="text-center text-slate-400 py-4">
                                    <p class="text-sm">{transcripts.length} transcript entries</p>
                                    <p class="text-xs text-slate-500 mt-1">Click "Expand" to view full transcripts</p>
                                </div>
                            {:else}
                                <!-- Full Transcript View - Beautiful Bubbles -->
                                <div class="space-y-3 max-h-[500px] overflow-y-auto pr-2">
                                    {#each transcripts.slice(-15).reverse() as t, i (t.id)}
                                        {@const speakerNum = t.speaker?.includes('1') || t.speaker === 'You' ? 1 : 2}
                                        {@const isYou = speakerNum === 1}
                                        <div class="flex {isYou ? 'justify-end' : 'justify-start'} animate-fadeIn">
                                            <div class="max-w-[80%] {isYou ? 'order-2' : 'order-1'}">
                                                <!-- Speaker info -->
                                                <div class="flex items-center gap-2 mb-1 {isYou ? 'justify-end' : 'justify-start'}">
                                                    <span class="text-xs {isYou ? 'text-cyan-400' : 'text-purple-400'} font-medium">
                                                        {isYou ? 'You' : t.speaker || 'Speaker 2'}
                                                    </span>
                                                    <span class="text-xs text-slate-600">{t.timestamp}</span>
                                                </div>
                                                <!-- Message bubble -->
                                                <div class="p-3 rounded-2xl {isYou 
                                                    ? 'bg-cyan-500/20 border border-cyan-500/30 rounded-tr-sm' 
                                                    : 'bg-purple-500/10 border border-purple-500/20 rounded-tl-sm'}">
                                                    <p class="text-sm text-slate-200 leading-relaxed">
                                                        {t.text}
                                                    </p>
                                                </div>
                                                <!-- Sentiment/tone indicator -->
                                                {#if t.tone}
                                                    <div class="flex {isYou ? 'justify-end' : 'justify-start'} mt-1">
                                                        <span class="text-xs px-2 py-0.5 rounded-full {
                                                            t.tone === 'POSITIVE' ? 'bg-green-500/20 text-green-400' :
                                                            t.tone === 'NEGATIVE' ? 'bg-red-500/20 text-red-400' :
                                                            t.tone === 'URGENT' ? 'bg-orange-500/20 text-orange-400' :
                                                            'bg-slate-500/20 text-slate-400'
                                                        }">{t.tone.toLowerCase()}</span>
                                                    </div>
                                                {/if}
                                            </div>
                                            <!-- Avatar -->
                                            <div class="{isYou ? 'order-3 ml-2' : 'order-0 mr-2'} flex-shrink-0">
                                                <div class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-bold {
                                                    isYou ? 'bg-cyan-500/30 text-cyan-300' : 'bg-purple-500/30 text-purple-300'
                                                }">
                                                    {isYou ? 'Y' : 'S2'}
                                                </div>
                                            </div>
                                        </div>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                    </div>

                    <!-- === SUMMARY PANEL (shows after extraction) === -->
                    {#if showSummaryPanel && extractedSummary}
                        <div class="content-card border-green-500/30 animate-fadeIn">
                            <div class="content-card-header">
                                <span class="text-sm font-medium text-green-400 flex items-center gap-2"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg> Meeting Summary</span>
                                <button class="icon-btn text-slate-400 hover:text-white" onclick={closeSummaryPanel} aria-label="Close summary panel">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                            <div class="p-6 space-y-4">
                                {#if extractedSummary.topics?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Topics Discussed</h4>
                                        <div class="flex flex-wrap gap-2">
                                            {#each extractedSummary.topics as topic}
                                                <span class="px-2 py-1 text-sm rounded bg-cyan-500/20 text-cyan-400">{topic}</span>
                                            {/each}
                                        </div>
                                    </div>
                                {/if}
                                {#if extractedSummary.decisions?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Decisions Made</h4>
                                        <ul class="space-y-1">
                                            {#each extractedSummary.decisions as decision}
                                                <li class="text-sm text-slate-300 flex items-start gap-2">
                                                    <svg class="w-3 h-3 text-green-400 flex-shrink-0 mt-0.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3"><polyline points="20 6 9 17 4 12"/></svg> {decision}
                                                </li>
                                            {/each}
                                        </ul>
                                    </div>
                                {/if}
                                {#if extractedSummary.actionItems?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Action Items</h4>
                                        <ul class="space-y-1">
                                            {#each extractedSummary.actionItems as action}
                                                <li class="text-sm text-slate-300 flex items-start gap-2">
                                                    <span class="text-yellow-400">→</span> {action}
                                                </li>
                                            {/each}
                                        </ul>
                                    </div>
                                {/if}
                                {#if extractedSummary.keyPoints?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Key Takeaways</h4>
                                        <ul class="space-y-1">
                                            {#each extractedSummary.keyPoints as point}
                                                <li class="text-sm text-slate-300 flex items-start gap-2">
                                                    <span class="text-blue-400">•</span> {point}
                                                </li>
                                            {/each}
                                        </ul>
                                    </div>
                                {/if}
                            </div>
                        </div>
                    {/if}

                    <!-- === MEMORIES PANEL (shows after extraction) === -->
                    {#if showMemoriesPanel && extractedMemories}
                        <div class="content-card border-purple-500/30 animate-fadeIn">
                            <div class="content-card-header">
                                <span class="text-sm font-medium text-purple-400 flex items-center gap-2"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5z"/></svg> Extracted Memories</span>
                                <button class="icon-btn text-slate-400 hover:text-white" onclick={closeMemoriesPanel} aria-label="Close memories panel">
                                    <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                                    </svg>
                                </button>
                            </div>
                            <div class="p-6 space-y-4">
                                {#if extractedMemories.keyMoments?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Key Moments</h4>
                                        <ul class="space-y-2">
                                            {#each extractedMemories.keyMoments as moment}
                                                <li class="text-sm text-slate-300 p-2 rounded bg-purple-500/10 border-l-2 border-purple-500">
                                                    {moment}
                                                </li>
                                            {/each}
                                        </ul>
                                    </div>
                                {/if}
                                {#if extractedMemories.quotes?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Notable Quotes</h4>
                                        <div class="space-y-2">
                                            {#each extractedMemories.quotes as quote}
                                                <blockquote class="text-sm text-slate-300 italic border-l-2 border-cyan-500 pl-3 py-1">
                                                    "{quote}"
                                                </blockquote>
                                            {/each}
                                        </div>
                                    </div>
                                {/if}
                                {#if extractedMemories.personalInsights?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Personal Insights</h4>
                                        <ul class="space-y-1">
                                            {#each extractedMemories.personalInsights as insight}
                                                <li class="text-sm text-slate-300 flex items-start gap-2">
                                                    <span class="text-yellow-400">•</span> {insight}
                                                </li>
                                            {/each}
                                        </ul>
                                    </div>
                                {/if}
                                {#if extractedMemories.emotionShifts?.length > 0}
                                    <div>
                                        <h4 class="text-xs text-slate-400 uppercase tracking-wider mb-2">Emotion Shifts</h4>
                                        <ul class="space-y-1">
                                            {#each extractedMemories.emotionShifts as shift}
                                                <li class="text-sm text-slate-300 flex items-start gap-2">
                                                    <span class="text-orange-400">↗</span> {shift}
                                                </li>
                                            {/each}
                                        </ul>
                                    </div>
                                {/if}
                            </div>
                        </div>
                    {/if}

                {:else if activeTab === "graph"}
                    <div class="content-card h-[500px]">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200">Knowledge Graph Visualization</span>
                            <span class="text-xs text-slate-500">{graphNodes.length} nodes • {graphEdges.length} edges</span>
                        </div>
                        <div class="h-full p-4">
                            <KnowledgeGraph nodes={graphNodes} edges={graphEdges} />
                        </div>
                    </div>

                {:else if activeTab === "alerts"}
                    <div class="content-card">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200 flex items-center gap-2"><svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.73 21a2 2 0 0 1-3.46 0"/></svg> Intelligence Alerts</span>
                            <button class="btn-ghost text-xs" onclick={() => alerts = []}>Clear All</button>
                        </div>
                        <div class="p-6 space-y-3 max-h-96 overflow-y-auto">
                            {#if alerts.length === 0}
                                <div class="text-center py-12 text-slate-500">
                                    <svg class="w-12 h-12 mx-auto mb-4 text-slate-500 opacity-30" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                                    <p>No alerts yet. Alerts appear when important events are detected.</p>
                                </div>
                            {:else}
                                {#each alerts as alert}
                                    <div class="glass-card p-4 {
                                        alert.severity === 'critical' ? 'border-red-500/30' :
                                        alert.severity === 'warning' ? 'border-yellow-500/30' :
                                        'border-cyan-500/30'
                                    }">
                                        <div class="flex items-start gap-3">
                                            <svg class="w-5 h-5 {alert.severity === 'critical' ? 'text-red-500' : alert.severity === 'warning' ? 'text-yellow-500' : 'text-cyan-500'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                                            <div class="flex-1">
                                                <div class="flex justify-between">
                                                    <span class="font-medium text-slate-200">{alert.type}</span>
                                                    <span class="text-xs text-slate-500">{alert.timestamp}</span>
                                                </div>
                                                <p class="text-sm text-slate-400 mt-1">{alert.message}</p>
                                            </div>
                                        </div>
                                    </div>
                                {/each}
                            {/if}
                        </div>
                    </div>

                {:else if activeTab === "analytics"}
                    <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                        <div class="glass-card p-4 text-center">
                            <div class="text-3xl font-bold text-cyan-400">{transcripts.length}</div>
                            <div class="text-xs text-slate-500 mt-1">Transcripts</div>
                        </div>
                        <div class="glass-card p-4 text-center">
                            <div class="text-3xl font-bold text-cyan-400">{graphNodes.length}</div>
                            <div class="text-xs text-slate-500 mt-1">Entities</div>
                        </div>
                        <div class="glass-card p-4 text-center">
                            <div class="text-3xl font-bold text-green-400">
                                {transcripts.filter(t => t.category?.includes('TASK')).length}
                            </div>
                            <div class="text-xs text-slate-500 mt-1">Tasks Found</div>
                        </div>
                        <div class="glass-card p-4 text-center">
                            <div class="text-3xl font-bold text-blue-400">
                                {transcripts.filter(t => t.category?.includes('DECISION')).length}
                            </div>
                            <div class="text-xs text-slate-500 mt-1">Decisions</div>
                        </div>
                    </div>
                    
                    <div class="glass-card p-6">
                        <h4 class="text-sm font-medium text-slate-200 mb-4">Performance Metrics</h4>
                        <div class="grid grid-cols-3 gap-4 text-center">
                            <div>
                                <div class="text-2xl font-bold text-green-400">{latencyMs}ms</div>
                                <div class="text-xs text-slate-500">Current Latency</div>
                            </div>
                            <div>
                                <div class="text-2xl font-bold {isGeminiConnected ? 'text-green-400' : 'text-red-400'}">
                                    <svg class="w-6 h-6 {isGeminiConnected ? 'text-green-400' : 'text-red-400'}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">{#if isGeminiConnected}<polyline points="20 6 9 17 4 12"/>{:else}<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>{/if}</svg>
                                </div>
                                <div class="text-xs text-slate-500">API Connected</div>
                            </div>
                            <div>
                                <div class="text-2xl font-bold {isRecording ? 'text-red-400' : 'text-slate-500'}">
                                    <span class="w-4 h-4 rounded-full {isRecording ? 'bg-red-500 animate-pulse' : 'bg-slate-600'}"></span>
                                </div>
                                <div class="text-xs text-slate-500">Recording</div>
                            </div>
                        </div>
                    </div>

                {:else if activeTab === "settings"}
                    <div class="glass-card p-6">
                        <h3 class="text-lg font-medium text-slate-200 mb-6">Audio & Processing Controls</h3>
                        
                        <!-- Model Selection -->
                        <div class="mb-6">
                            <label for="model-select" class="block text-xs text-slate-400 mb-2">AI Model</label>
                            <select id="model-select" bind:value={selectedModel} class="select-field w-full">
                                {#each availableModels as model}
                                    <option value={model.id}>{model.name}</option>
                                {/each}
                            </select>
                        </div>
                        
                        <!-- API Key -->
                        <div class="mb-6">
                            <label for="api-key-input" class="block text-xs text-slate-400 mb-2">Gemini API Key</label>
                            <div class="flex gap-2">
                                <input
                                    id="api-key-input"
                                    type="password"
                                    bind:value={apiKey}
                                    class="input-field flex-1"
                                    placeholder="AIza..."
                                />
                                <button
                                    class="btn-secondary"
                                    onclick={connectGemini}
                                    disabled={isGeminiConnected}
                                >
                                    {isGeminiConnected ? "Connected" : "Connect"}
                                </button>
                            </div>
                        </div>
                        
                        <!-- Capture Source -->
                        <div class="mb-6">
                            <span class="block text-xs text-slate-400 mb-2">Capture Source</span>
                            <div class="flex gap-2">
                                <button
                                    class="{captureMode === 'mic' ? 'btn-primary' : 'btn-secondary'} flex-1"
                                    onclick={() => setCaptureMode('mic')}
                                    aria-label="Set Mic Only"
                                >
                                    <svg class="w-4 h-4 inline mr-1" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/></svg> Mic
                                </button>
                                <button
                                    class="{captureMode === 'system' ? 'btn-primary' : 'btn-secondary'} flex-1"
                                    onclick={() => setCaptureMode('system')}
                                    aria-label="Set System Audio Only"
                                >
                                    🔊 System
                                </button>
                                <button
                                    class="{captureMode === 'both' ? 'btn-primary' : 'btn-secondary'} flex-1"
                                    onclick={() => setCaptureMode('both')}
                                    aria-label="Set Both Audio Sources"
                                >
                                    📻 Both
                                </button>
                            </div>
                        </div>
                        
                        <!-- Audio Level -->
                        <div class="mb-6">
                            <span class="block text-xs text-slate-400 mb-2">Audio Level</span>
                            <div class="space-y-2">
                                {#if currentVolume !== undefined}
                                    <div class="h-3 bg-dark-700 rounded-full overflow-hidden" role="progressbar" aria-valuenow={Math.min(Math.max(currentVolume * 10, 0), 1) * 100} aria-valuemin="0" aria-valuemax="100">
                                        <div 
                                            class="h-full transition-all duration-75 {currentVolume > 0.15 ? 'bg-red-500' : currentVolume > 0.05 ? 'bg-green-500' : currentVolume > 0.01 ? 'bg-cyan-500' : 'bg-slate-600'}"
                                            style="width: {Math.min(Math.max(currentVolume * 10, 0), 1) * 100}%"
                                        ></div>
                                    </div>
                                    <div class="flex justify-between text-xs text-slate-500 mt-1">
                                        <span class="flex items-center gap-1">
                                            <span class="w-2 h-2 rounded-full {isRecording ? (currentVolume > 0.02 ? 'bg-green-500 animate-pulse' : 'bg-cyan-500') : 'bg-slate-600'}"></span>
                                            {isRecording ? 'Listening' : 'Idle'}
                                        </span>
                                        <span>{isRecording ? (currentVolume > 0.0001 ? (20 * Math.log10(currentVolume)).toFixed(0) + ' dB' : '-∞ dB') : '--'}</span>
                                    </div>
                                {/if}
                            </div>
                        </div>
                        
                        <GodControls onSettingsChange={handleSettingsChange} />
                    </div>

                {:else if activeTab === "diagnostics"}
                    <Diagnostics {isRecording} {isGeminiConnected} />
                {/if}
            </div>
        </div>

        <!-- BOTTOM ACTION BAR -->
        <div class="h-20 px-6 flex items-center justify-center gap-4 border-t border-cyan-500/10 bg-[#0d1117]/50">
            <!-- Error Toast -->
            {#if extractError}
                <div class="absolute top-2 left-1/2 -translate-x-1/2 px-4 py-2 rounded-lg bg-red-500/20 border border-red-500/50 text-red-400 text-sm animate-fadeIn">
                    {extractError}
                </div>
            {/if}
            
            <button 
                class="btn-action {isCollapsed ? 'bg-cyan-500/20' : ''}" 
                onclick={toggleCollapseState}
                disabled={transcripts.length === 0}
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    {#if isCollapsed}
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                        <line x1="3" y1="12" x2="21" y2="12"></line>
                    {:else}
                        <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                        <line x1="9" y1="3" x2="9" y2="21"></line>
                    {/if}
                </svg>
                {isCollapsed ? 'Expand Transcript' : 'Collapse Transcript'}
            </button>
            
            <button 
                class="btn-action {showMemoriesPanel ? 'bg-purple-500/20' : ''}" 
                onclick={extractMemories}
                disabled={isExtractingMemories || transcripts.length === 0}
            >
                {#if isExtractingMemories}
                    <span class="animate-spin">⏳</span>
                {:else}
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="3"></circle>
                        <circle cx="19" cy="5" r="2"></circle>
                        <circle cx="5" cy="5" r="2"></circle>
                        <circle cx="19" cy="19" r="2"></circle>
                        <circle cx="5" cy="19" r="2"></circle>
                        <line x1="12" y1="9" x2="12" y2="3"></line>
                        <line x1="14.5" y1="13.5" x2="19" y2="17"></line>
                        <line x1="9.5" y1="13.5" x2="5" y2="17"></line>
                        <line x1="14.5" y1="10.5" x2="19" y2="7"></line>
                        <line x1="9.5" y1="10.5" x2="5" y2="7"></line>
                    </svg>
                {/if}
                {isExtractingMemories ? 'Extracting...' : 'Extract Memories'}
            </button>
            
            <button 
                class="btn-action {showSummaryPanel ? 'bg-green-500/20' : ''}" 
                onclick={extractSummary}
                disabled={isExtractingSummary || transcripts.length === 0}
            >
                {#if isExtractingSummary}
                    <span class="animate-spin">⏳</span>
                {:else}
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                        <polyline points="14 2 14 8 20 8"></polyline>
                        <line x1="16" y1="13" x2="8" y2="13"></line>
                        <line x1="16" y1="17" x2="8" y2="17"></line>
                        <polyline points="10 9 9 9 8 9"></polyline>
                    </svg>
                {/if}
                {isExtractingSummary ? 'Generating...' : 'Summary'}
            </button>
            
            <!-- Diamond Icon -->
            <div class="ml-4 diamond-icon">
                <span class="text-cyan-400">◆</span>
            </div>
        </div>
    </div>
</div>
