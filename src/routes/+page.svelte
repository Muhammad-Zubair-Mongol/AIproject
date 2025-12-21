<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { listen } from "@tauri-apps/api/event";
    import { onMount, onDestroy } from "svelte";
    import KnowledgeGraph from "$lib/KnowledgeGraph.svelte";
    import GodControls from "$lib/GodControls.svelte";
    import SessionManager from "$lib/SessionManager.svelte";
    import Diagnostics from "$lib/Diagnostics.svelte";

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
        { id: "gemini-2.5-flash-preview-09-2025", name: "⚡ Gemini 2.5 Flash (REST)" },
        { id: "gemini-2.5-flash-lite-preview-09-2025", name: "🔥 Gemini 2.5 Flash Lite (REST)" },
        { id: "gemini-3-flash-preview", name: "💎 Gemini 3 Flash Preview (REST)" },
        { id: "gemini-2.5-flash-native-audio-preview-12-2025", name: "🎤 Native Audio (Live API)" },
    ];

    // Station 1: Audio Controls
    let captureMode = "mic"; // "mic", "system", "both"
    let currentVolume = 0;
    let volumeInterval: ReturnType<typeof setInterval> | null = null;

    // Station 4: Latency Illusion
    let isTyping = false; // Show typing indicator
    let partialText = ""; // Progressive render buffer
    let latencyMs = 0; // Current latency metric
    let searchQuery = ""; // Full-text search
    let searchFilter = "all"; // speaker, category, time

    // Core Data
    let transcripts: Array<{
        id: string;
        timestamp: string;
        speaker: string;
        text: string;
        tone?: string;
        category?: string[];
        confidence?: number;
        isPartial?: boolean; // For progressive renders
    }> = [];
    
    // Alerts data
    let alerts: Array<{
        id: string;
        type: string;
        message: string;
        timestamp: string;
        severity: "info" | "warning" | "critical";
    }> = [];

    let activeTab: "transcript" | "graph" | "alerts" | "analytics" | "settings" | "diagnostics" = "transcript";

    // Graph Data
    let graphNodes: Array<{ id: string; type: string; label?: string; weight?: number }> = [];
    let graphEdges: Array<{ from: string; to: string; relation: string }> = [];

    // Session Data
    let currentSession: any = {
        id: "",
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        transcripts: [],
        graph_nodes: [],
        graph_edges: [],
        metadata: {
            title: "Untitled Meeting",
        },
    };

    // --- ACTIONS ---
    function simulateIntelligence() {
        const id = `sim_${Date.now()}`;
        const timestamp = new Date().toLocaleTimeString([], {
            hour: "2-digit",
            minute: "2-digit",
        });

        // 1. Add Transcript
        const mockTranscripts = [
            "We should definitely prioritize the database migration next sprint.",
            "I agree, but we need to ensure the backup strategy is solid.",
            "Let's assign Sarah to lead the migration task.",
            "The latency on the current API is around 250ms, which is too high."
        ];
        const text = mockTranscripts[Math.floor(Math.random() * mockTranscripts.length)];
        
        transcripts = [...transcripts, {
            id,
            timestamp,
            speaker: "Simulated User",
            text,
            tone: ["URGENT", "POSITIVE", "NEUTRAL", "DOMINANT"][Math.floor(Math.random() * 4)],
            category: ["TASK", "DECISION", "METRIC"].slice(0, Math.floor(Math.random() * 3) + 1),
            confidence: 0.85 + Math.random() * 0.1,
            isPartial: false
        }];

        // 2. Add Graph Node/Edge
        const entityNames = ["Database", "Migration", "Sarah", "API", "Latency", "Backup"];
        const entity = entityNames[Math.floor(Math.random() * entityNames.length)];
        
        if (!graphNodes.find(n => n.id === entity)) {
            graphNodes = [...graphNodes, { id: entity, type: "Entity", label: entity, weight: 1 }];
        } else {
            graphNodes = graphNodes.map(n => n.id === entity ? { ...n, weight: (n.weight || 1) + 1 } : n);
        }

        if (graphNodes.length > 1) {
            const from = graphNodes[graphNodes.length - 2].id;
            const to = entity;
            if (from !== to) {
                graphEdges = [...graphEdges, { from, to, relation: "related" }];
            }
        }

        // 3. Add Alert
        if (Math.random() > 0.5) {
            alerts = [{
                id: `alert_${Date.now()}`,
                type: "Insight",
                message: `New entity detected: ${entity}`,
                timestamp,
                severity: Math.random() > 0.8 ? "critical" : "info"
            }, ...alerts];
        }
        
        status = "Simulated Event Processed";
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
        } catch (error) {
            console.error("Failed to set capture mode:", error);
        }
    }
    
    async function pollVolume() {
        if (!isRecording) return;
        try {
            currentVolume = await invoke("get_current_volume");
        } catch (error) {
            // Silently ignore volume poll errors
        }
    }

    async function toggleCapture() {
        try {
            if (isRecording) {
                await invoke("stop_audio_capture");
                isRecording = false;
                status = "Idle";
                // Stop volume polling
                if (volumeInterval) {
                    clearInterval(volumeInterval);
                    volumeInterval = null;
                }
                currentVolume = 0;
            } else {
                await invoke("start_audio_capture");
                isRecording = true;
                status = "Recording...";
                // Start volume polling (10 times per second)
                volumeInterval = setInterval(pollVolume, 100);
            }
        } catch (error) {
            console.error(error);
            status = "Capture Error";
        }
    }

    async function connectGemini() {
        if (!apiKey) {
            status = "API Key Required";
            return;
        }
        
        // Save to localStorage immediately (before connect attempt)
        localStorage.setItem("gemini_api_key", apiKey);
        localStorage.setItem("gemini_model", selectedModel);
        
        // Check if running in Tauri
        if (!isRunningInTauri) {
            status = "Browser Mode - Cannot connect (use npm run tauri dev)";
            return;
        }
        
        try {
            status = "Connecting to " + selectedModel + "...";
            const result = await invoke("test_gemini_connection", { 
                key: apiKey,
                model: selectedModel 
            });
            console.log("Connection result:", result);
        } catch (error) {
            console.error(error);
            status = "Connection Failed: " + error;
        }
    }

    function handleSettingsChange(settings: any) {
        console.log("Settings updated:", settings);
    }

    function handleSessionLoad(session: any) {
        currentSession = session;
        transcripts = session.transcripts
            ? session.transcripts.map((t: any) => ({
                  timestamp: t.timestamp,
                  speaker: t.speaker_id,
                  text: t.text,
                  tone: t.tone,
                  category: t.category,
                  confidence: t.confidence,
              }))
            : [];
        graphNodes = session.graph_nodes || [];
        graphEdges = session.graph_edges || [];
        status = "Session Loaded";
    }

    // --- EVENTS ---
    let unlistenStatus: () => void;
    let unlistenTranscript: () => void;
    let unlistenIntelligence: () => void;

    onMount(async () => {
        // Load saved settings from localStorage
        const savedKey = localStorage.getItem("gemini_api_key");
        const savedModel = localStorage.getItem("gemini_model");
        if (savedKey) apiKey = savedKey;
        
        // Check if saved model is valid
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
        
        // Detect if running in Tauri
        isRunningInTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__ !== undefined;
        
        if (!isRunningInTauri) {
            console.warn("Tauri API not detected. App is likely running in a web browser. Native features will be disabled.");
            status = "Web Preview (Native Disabled)";
            return;
        }

        try {
            await loadDevices();

            unlistenStatus = await listen("god:status", (event) => {
            const s = event.payload as string;
            // Clean up status messages for UI
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

        unlistenTranscript = await listen("god:transcript", (event) => {
            const text = event.payload as string;
            const startTime = performance.now();
            
            // Station 4: Progressive rendering with typing illusion
            isTyping = true;
            partialText = text;
            
            const newTranscript = {
                id: `t_${Date.now()}`,
                timestamp: new Date().toLocaleTimeString([], {
                    hour: "2-digit",
                    minute: "2-digit",
                }),
                speaker: "Speaker",
                text: text,
                confidence: 0.5,
                isPartial: true, // Will be updated when intelligence arrives
            };
            
            transcripts = [...transcripts, newTranscript];
            
            // Track latency
            latencyMs = Math.round(performance.now() - startTime);
            
            // Clear typing indicator after short delay
            setTimeout(() => {
                isTyping = false;
                partialText = "";
            }, 100);
        });

        unlistenIntelligence = await listen("god:intelligence", (event) => {
            const intel = event.payload as any;
            console.log("Intelligence:", intel);
            // Logic to update existing transcript or add new structured item could go here
        });
        
        // Station 6: Tray icon events
        await listen("tray:record", () => {
            if (!isRecording) toggleCapture();
        });
        
        await listen("tray:stop", () => {
            if (isRecording) toggleCapture();
        });
        } catch (error) {
            console.error("Failed to initialize Tauri listeners:", error);
            status = "Tauri Init Error";
        }
        
        // Global keyboard shortcuts
        document.addEventListener("keydown", handleKeyDown);
    });
    
    function handleKeyDown(e: KeyboardEvent) {
        // Ctrl+Shift shortcuts for god controls
        if (e.ctrlKey && e.shiftKey) {
            switch (e.key) {
                case "R": // Ctrl+Shift+R: Toggle recording
                    e.preventDefault();
                    toggleCapture();
                    break;
                case "S": // Ctrl+Shift+S: Quick save session
                    e.preventDefault();
                    // Emit save event
                    console.log("[HOTKEY] Save session");
                    break;
                case "G": // Ctrl+Shift+G: Toggle graph view
                    e.preventDefault();
                    activeTab = activeTab === "graph" ? "transcript" : "graph";
                    break;
                case "A": // Ctrl+Shift+A: Toggle alerts
                    e.preventDefault();
                    activeTab = "alerts";
                    break;
                case "T": // Ctrl+Shift+T: Toggle transcript
                    e.preventDefault();
                    activeTab = "transcript";
                    break;
            }
        }
        
        // Escape to minimize
        if (e.key === "Escape" && isRecording) {
            // Could minimize to tray here
            console.log("[HOTKEY] Escape pressed during recording");
        }
    }

    onDestroy(() => {
        if (unlistenStatus) unlistenStatus();
        if (unlistenTranscript) unlistenTranscript();
        if (unlistenIntelligence) unlistenIntelligence();
        document.removeEventListener("keydown", handleKeyDown);
    });
</script>

<div
    class="h-screen w-screen flex bg-slate-50 text-slate-900 font-sans overflow-hidden"
>
    <!-- SIDEBAR -->
    <div
        class="w-64 bg-white border-r border-slate-200 flex flex-col justify-between"
    >
        <!-- Brand -->
        <div class="p-6 border-b border-slate-100">
            <h1 class="text-lg font-bold tracking-tight text-slate-800">
                Meeting Mind
            </h1>
            <p class="text-xs text-slate-500 mt-1">Intelligence Engine</p>
        </div>

        <!-- Navigation -->
        <div class="flex-1 p-4 space-y-1 overflow-y-auto">
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'transcript'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "transcript")}
            >
                Transcripts
            </button>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'graph'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "graph")}
            >
                🕸️ Knowledge Graph
            </button>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'alerts'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "alerts")}
            >
                🔔 Alerts
                {#if alerts.filter(a => a.severity === 'critical').length > 0}
                    <span class="ml-2 bg-red-500 text-white text-xs px-1.5 py-0.5 rounded-full">
                        {alerts.filter(a => a.severity === 'critical').length}
                    </span>
                {/if}
            </button>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'analytics'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "analytics")}
            >
                📊 Analytics
            </button>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'diagnostics'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "diagnostics")}
            >
                🔧 Diagnostics
            </button>
            <div class="pt-4 pb-2">
                <p
                    class="px-3 text-xs font-semibold text-slate-400 uppercase tracking-wider"
                >
                    Settings
                </p>
            </div>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'settings'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "settings")}
            >
                Configuration
            </button>

            <!-- Debug / Testing -->
            <div class="pt-4 pb-2">
                <p
                    class="px-3 text-xs font-semibold text-slate-400 uppercase tracking-wider"
                >
                    Testing
                </p>
            </div>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium text-blue-600 hover:bg-blue-50 transition-colors"
                onclick={simulateIntelligence}
            >
                🚀 Simulate Intelligence
            </button>

            <!-- Quick connection controls -->
            <div
                class="mt-6 p-3 bg-slate-50 rounded-md border border-slate-100"
            >
                <label
                    for="model"
                    class="block text-xs font-medium text-slate-500 mb-2"
                    >Model</label
                >
                <select
                    id="model"
                    bind:value={selectedModel}
                    class="w-full px-2 py-1.5 text-xs border border-slate-200 rounded-md bg-white mb-3"
                >
                    {#each availableModels as model}
                        <option value={model.id}>{model.name}</option>
                    {/each}
                </select>
                
                <label
                    for="apikey"
                    class="block text-xs font-medium text-slate-500 mb-2"
                    >Gemini API Key</label
                >
                <input
                    id="apikey"
                    type="password"
                    bind:value={apiKey}
                    class="input-field mb-2"
                    placeholder="AIza..."
                />

                <button
                    class="w-full btn-secondary text-xs"
                    onclick={connectGemini}
                    disabled={isGeminiConnected}
                >
                    {isGeminiConnected ? "✓ Connected" : "Connect"}
                </button>
            </div>

            <!-- Station 1: Audio Controls -->
            <div
                class="mt-4 p-3 bg-slate-50 rounded-md border border-slate-100"
            >
                <span class="block text-xs font-medium text-slate-500 mb-2"
                    >Capture Source</span
                >
                <div class="flex gap-1 mb-3">
                    <button
                        class="flex-1 text-xs px-2 py-1 rounded {captureMode === 'mic' ? 'bg-slate-900 text-white' : 'bg-slate-200 text-slate-700'}"
                        onclick={() => setCaptureMode('mic')}
                    >
                        🎤 Mic
                    </button>
                    <button
                        class="flex-1 text-xs px-2 py-1 rounded {captureMode === 'system' ? 'bg-slate-900 text-white' : 'bg-slate-200 text-slate-700'}"
                        onclick={() => setCaptureMode('system')}
                    >
                        🔊 System
                    </button>
                    <button
                        class="flex-1 text-xs px-2 py-1 rounded {captureMode === 'both' ? 'bg-slate-900 text-white' : 'bg-slate-200 text-slate-700'}"
                        onclick={() => setCaptureMode('both')}
                    >
                        📻 Both
                    </button>
                </div>
                
                <!-- Volume Meter -->
                <span class="block text-xs font-medium text-slate-500 mb-1"
                    >Audio Level</span
                >
                <div class="h-2 bg-slate-200 rounded-full overflow-hidden">
                    <div 
                        class="h-full transition-all duration-75 {currentVolume > 0.5 ? 'bg-red-500' : currentVolume > 0.1 ? 'bg-green-500' : 'bg-slate-400'}"
                        style="width: {Math.min(currentVolume * 500, 100)}%"
                    ></div>
                </div>
                <div class="text-xs text-slate-400 mt-1 text-right">
                    {isRecording ? (currentVolume * 100).toFixed(1) + ' dB' : 'Idle'}
                </div>
            </div>
        </div>

        <!-- Session List (Mini) -->
        <div class="p-4 border-t border-slate-200 bg-slate-50/50">
            <SessionManager
                {currentSession}
                onSessionLoad={handleSessionLoad}
            />
        </div>
    </div>

    <!-- MAIN CONTENT -->
    <div class="flex-1 flex flex-col min-w-0">
        <!-- HEADER -->
        <div
            class="h-16 px-6 bg-white border-b border-slate-200 flex items-center justify-between"
        >
            <div class="flex items-center gap-3">
                <span
                    class="inline-flex h-2.5 w-2.5 rounded-full {isRecording ||
                    isGeminiConnected
                        ? 'bg-green-500'
                        : 'bg-slate-300'}"
                ></span>
                <span class="text-sm font-medium text-slate-700">{status}</span>
                {#if isRecording}
                    <span
                        class="ml-2 badge bg-red-100 text-red-700 border border-red-200"
                        >LIVE</span
                    >
                {/if}
            </div>

            <div class="flex items-center gap-4">
                <button
                    class="{isRecording
                        ? 'bg-red-600 hover:bg-red-700 text-white'
                        : 'btn-primary'} px-4 py-2 rounded-md font-medium transition-colors"
                    onclick={toggleCapture}
                >
                    {isRecording ? "Stop Recording" : "Start Recording"}
                </button>
            </div>
        </div>

        <!-- CONTENT AREA -->
        <div class="flex-1 overflow-auto bg-slate-50 p-6">
            <div class="max-w-4xl mx-auto h-full">
                {#if !isRunningInTauri}
                    <div class="mb-6 p-4 bg-amber-50 border border-amber-200 rounded-lg flex items-center gap-3 text-amber-800 shadow-sm">
                        <span class="text-2xl">⚠️</span>
                        <div class="flex-1">
                            <p class="font-bold">Running in Browser Mode</p>
                            <p class="text-sm opacity-90">Native features like audio capture and system integration are disabled. Use the simulation tool or open the app via <code>npm run tauri dev</code>.</p>
                        </div>
                    </div>
                {/if}

                {#if activeTab === "transcript"}
                    <!-- Search Bar -->
                    <div class="mb-4 flex gap-2">
                        <input
                            type="text"
                            placeholder="🔍 Search transcripts..."
                            bind:value={searchQuery}
                            class="flex-1 px-4 py-2 rounded-lg border border-slate-200 focus:border-slate-400 focus:ring-1 focus:ring-slate-400 outline-none"
                        />
                        <select
                            bind:value={searchFilter}
                            class="px-3 py-2 rounded-lg border border-slate-200 bg-white"
                        >
                            <option value="all">All</option>
                            <option value="speaker">By Speaker</option>
                            <option value="category">By Category</option>
                        </select>
                    </div>
                    
                    <!-- Latency Indicator -->
                    {#if latencyMs > 0}
                        <div class="mb-2 text-xs text-slate-400 flex items-center gap-2">
                            <span class="inline-block w-2 h-2 bg-green-500 rounded-full animate-pulse"></span>
                            Latency: {latencyMs}ms
                        </div>
                    {/if}
                    
                    <!-- Typing Indicator -->
                    {#if isTyping}
                        <div class="mb-4 p-3 bg-slate-100 rounded-lg border border-slate-200 animate-pulse">
                            <div class="flex items-center gap-2 text-slate-500">
                                <span class="flex gap-1">
                                    <span class="w-2 h-2 bg-slate-400 rounded-full animate-bounce"></span>
                                    <span class="w-2 h-2 bg-slate-400 rounded-full animate-bounce" style="animation-delay: 0.1s"></span>
                                    <span class="w-2 h-2 bg-slate-400 rounded-full animate-bounce" style="animation-delay: 0.2s"></span>
                                </span>
                                <span class="text-sm">Processing...</span>
                            </div>
                        </div>
                    {/if}

                    {#if transcripts.length === 0}
                        <div
                            class="h-full flex flex-col items-center justify-center text-slate-400"
                        >
                            <span class="mb-2 text-4xl font-light opacity-50"
                                >Transcription</span
                            >
                            <p>No transcripts yet. Start recording to begin.</p>
                        </div>
                    {:else}
                        <div class="space-y-3 pb-12">
                            {#each transcripts.filter(t => 
                                searchQuery === "" || 
                                t.text.toLowerCase().includes(searchQuery.toLowerCase()) ||
                                t.speaker.toLowerCase().includes(searchQuery.toLowerCase())
                            ) as t (t.id)}
                                {@const toneColor = {
                                    'URGENT': 'border-l-red-500 bg-red-50',
                                    'FRUSTRATED': 'border-l-orange-500 bg-orange-50',
                                    'EXCITED': 'border-l-yellow-500 bg-yellow-50',
                                    'POSITIVE': 'border-l-green-500 bg-green-50',
                                    'NEGATIVE': 'border-l-red-400 bg-red-50',
                                    'HESITANT': 'border-l-purple-500 bg-purple-50',
                                    'DOMINANT': 'border-l-blue-700 bg-blue-50',
                                    'EMPATHETIC': 'border-l-pink-500 bg-pink-50',
                                    'NEUTRAL': 'border-l-slate-300 bg-white',
                                }[t.tone || 'NEUTRAL'] || 'border-l-slate-300 bg-white'}
                                <div
                                    class="p-4 rounded-lg border border-slate-200 shadow-sm hover:shadow-md transition-all border-l-4 {toneColor} {t.isPartial ? 'opacity-75' : ''}"
                                >
                                    <div
                                        class="flex items-start justify-between mb-1"
                                    >
                                        <div class="flex items-center gap-2">
                                            <span
                                                class="font-bold text-slate-800"
                                                >{t.speaker}</span
                                            >
                                            <span class="text-xs text-slate-400"
                                                >{t.timestamp}</span
                                            >
                                            {#if t.isPartial}
                                                <span class="text-xs bg-yellow-100 text-yellow-700 px-1.5 py-0.5 rounded">
                                                    Processing...
                                                </span>
                                            {/if}
                                        </div>
                                        <div class="flex gap-1">
                                            {#if t.tone}
                                                <span
                                                    class="text-xs px-2 py-0.5 rounded-full bg-slate-100 text-slate-600"
                                                    >{t.tone}</span
                                                >
                                            {/if}
                                            {#if t.confidence}
                                                <span
                                                    class="text-xs px-2 py-0.5 rounded-full {t.confidence > 0.8 ? 'bg-green-100 text-green-700' : t.confidence > 0.5 ? 'bg-yellow-100 text-yellow-700' : 'bg-red-100 text-red-700'}"
                                                    >{(t.confidence * 100).toFixed(0)}%</span
                                                >
                                            {/if}
                                        </div>
                                    </div>
                                    <p
                                        class="text-slate-700 leading-relaxed text-sm lg:text-base"
                                    >
                                        {t.text}
                                    </p>
                                    {#if t.category && t.category.length > 0}
                                        <div class="mt-2 flex gap-1 flex-wrap">
                                            {#each t.category as cat}
                                                <span class="text-xs px-2 py-0.5 rounded bg-slate-200 text-slate-600">
                                                    {cat}
                                                </span>
                                            {/each}
                                        </div>
                                    {/if}
                                </div>
                            {/each}
                        </div>
                    {/if}
                {:else if activeTab === "graph"}
                    <div
                        class="h-full bg-white rounded-lg border border-slate-200 shadow-sm overflow-hidden relative"
                    >
                        <KnowledgeGraph nodes={graphNodes} edges={graphEdges} />
                        {#if graphNodes.length === 0}
                            <div
                                class="absolute inset-0 flex items-center justify-center pointer-events-none text-slate-400"
                            >
                                Graph Empty
                            </div>
                        {/if}
                    </div>
                {:else if activeTab === "alerts"}
                    <div class="space-y-4">
                        <div class="flex justify-between items-center mb-4">
                            <h3 class="text-lg font-medium text-slate-800">🔔 Intelligence Alerts</h3>
                            <button 
                                class="text-sm px-3 py-1 rounded bg-slate-100 hover:bg-slate-200 transition-colors"
                                onclick={() => alerts = []}
                            >
                                Clear All
                            </button>
                        </div>
                        
                        {#if alerts.length === 0}
                            <div class="text-center py-12 text-slate-400">
                                <span class="text-4xl mb-4 block">🔕</span>
                                <p>No alerts yet. Alerts will appear here when important events are detected.</p>
                            </div>
                        {:else}
                            {#each alerts as alert}
                                <div class="p-4 rounded-lg border {
                                    alert.severity === 'critical' ? 'bg-red-50 border-red-200' :
                                    alert.severity === 'warning' ? 'bg-yellow-50 border-yellow-200' :
                                    'bg-blue-50 border-blue-200'
                                }">
                                    <div class="flex items-start gap-3">
                                        <span class="text-lg">
                                            {alert.severity === 'critical' ? '🚨' : alert.severity === 'warning' ? '⚠️' : 'ℹ️'}
                                        </span>
                                        <div class="flex-1">
                                            <div class="flex justify-between">
                                                <span class="font-medium text-slate-800">{alert.type}</span>
                                                <span class="text-xs text-slate-400">{alert.timestamp}</span>
                                            </div>
                                            <p class="text-sm text-slate-600 mt-1">{alert.message}</p>
                                        </div>
                                    </div>
                                </div>
                            {/each}
                        {/if}
                    </div>
                {:else if activeTab === "analytics"}
                    <div class="space-y-6">
                        <h3 class="text-lg font-medium text-slate-800">📊 Meeting Analytics</h3>
                        
                        <!-- Summary Stats -->
                        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
                            <div class="bg-white p-4 rounded-lg border border-slate-200 text-center">
                                <div class="text-2xl font-bold text-slate-800">{transcripts.length}</div>
                                <div class="text-xs text-slate-500 mt-1">Transcripts</div>
                            </div>
                            <div class="bg-white p-4 rounded-lg border border-slate-200 text-center">
                                <div class="text-2xl font-bold text-slate-800">{graphNodes.length}</div>
                                <div class="text-xs text-slate-500 mt-1">Entities</div>
                            </div>
                            <div class="bg-white p-4 rounded-lg border border-slate-200 text-center">
                                <div class="text-2xl font-bold text-green-600">
                                    {transcripts.filter(t => t.category?.includes('TASK')).length}
                                </div>
                                <div class="text-xs text-slate-500 mt-1">Tasks Found</div>
                            </div>
                            <div class="bg-white p-4 rounded-lg border border-slate-200 text-center">
                                <div class="text-2xl font-bold text-blue-600">
                                    {transcripts.filter(t => t.category?.includes('DECISION')).length}
                                </div>
                                <div class="text-xs text-slate-500 mt-1">Decisions</div>
                            </div>
                        </div>
                        
                        <!-- Tone Distribution -->
                        <div class="bg-white p-4 rounded-lg border border-slate-200">
                            <h4 class="text-sm font-medium text-slate-700 mb-3">Tone Distribution</h4>
                            <div class="space-y-2">
                                {#each ['NEUTRAL', 'POSITIVE', 'URGENT', 'FRUSTRATED'] as tone}
                                    {@const count = transcripts.filter(t => t.tone === tone).length}
                                    {@const pct = transcripts.length > 0 ? (count / transcripts.length * 100) : 0}
                                    <div class="flex items-center gap-2">
                                        <span class="text-xs w-20 text-slate-600">{tone}</span>
                                        <div class="flex-1 h-2 bg-slate-100 rounded-full overflow-hidden">
                                            <div 
                                                class="h-full bg-slate-500 rounded-full transition-all"
                                                style="width: {pct}%"
                                            ></div>
                                        </div>
                                        <span class="text-xs text-slate-400 w-8">{count}</span>
                                    </div>
                                {/each}
                            </div>
                        </div>
                        
                        <!-- Latency Stats -->
                        <div class="bg-white p-4 rounded-lg border border-slate-200">
                            <h4 class="text-sm font-medium text-slate-700 mb-3">Performance Metrics</h4>
                            <div class="grid grid-cols-3 gap-4 text-center">
                                <div>
                                    <div class="text-lg font-bold text-green-600">{latencyMs}ms</div>
                                    <div class="text-xs text-slate-500">Current Latency</div>
                                </div>
                                <div>
                                    <div class="text-lg font-bold text-slate-600">
                                        {isGeminiConnected ? '✓' : '✗'}
                                    </div>
                                    <div class="text-xs text-slate-500">API Connected</div>
                                </div>
                                <div>
                                    <div class="text-lg font-bold text-slate-600">
                                        {isRecording ? '🔴' : '⚪'}
                                    </div>
                                    <div class="text-xs text-slate-500">Recording</div>
                                </div>
                            </div>
                        </div>
                    </div>
                {:else if activeTab === "settings"}
                    <div class="space-y-6">
                        <section>
                            <h3 class="text-lg font-medium text-slate-800 mb-4">
                                Audio & Processing Controls
                            </h3>
                            <GodControls
                                onSettingsChange={handleSettingsChange}
                            />
                        </section>
                    </div>
                {:else if activeTab === "diagnostics"}
                    <Diagnostics {isRecording} {isGeminiConnected} />
                {/if}
            </div>
        </div>
    </div>
</div>
