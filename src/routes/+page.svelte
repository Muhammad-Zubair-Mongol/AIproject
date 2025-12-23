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
        text: string;
        tone?: string;
        category?: string[];
        confidence?: number;
        isPartial?: boolean;
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
                status = "Ready";
                if (volumeInterval) {
                    clearInterval(volumeInterval);
                    volumeInterval = null;
                }
                currentVolume = 0;
            } else {
                await invoke("start_audio_capture");
                isRecording = true;
                status = "Recording...";
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
        
        localStorage.setItem("gemini_api_key", apiKey);
        localStorage.setItem("gemini_model", selectedModel);
        
        if (!isRunningInTauri) {
            status = "Browser Mode - Cannot connect";
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
        
        isRunningInTauri = typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__ !== undefined;
        
        if (!isRunningInTauri) {
            console.warn("Tauri API not detected.");
            status = "Web Preview Mode";
            return;
        }

        try {
            await loadDevices();

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

            unlistenTranscript = await listen("god:transcript", (event) => {
                const text = event.payload as string;
                const startTime = performance.now();
                
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
                    isPartial: true,
                };
                
                transcripts = [...transcripts, newTranscript];
                latencyMs = Math.round(performance.now() - startTime);
                
                setTimeout(() => {
                    isTyping = false;
                    partialText = "";
                }, 100);
            });

            unlistenIntelligence = await listen("god:intelligence", (event) => {
                const intel = event.payload as any;
                console.log("Intelligence:", intel);
            });
            
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
        document.removeEventListener("keydown", handleKeyDown);
    });

    // Computed display text for transcription
    $: displayText = transcripts.length > 0 
        ? transcripts[transcripts.length - 1].text 
        : "No transcripts yet. Start recording to begin.";
</script>

<div class="h-screen w-screen flex bg-[#0a0c0f] font-sans overflow-hidden">
    <!-- SIDEBAR -->
    <div class="w-72 sidebar flex flex-col">
        <!-- Brand Header -->
        <div class="p-5 border-b border-cyan-500/10">
            <div class="flex items-center justify-between">
                <div>
                    <h1 class="text-xl font-bold text-slate-100 tracking-tight">
                        Meeting Mind
                    </h1>
                    <p class="text-xs text-cyan-400 mt-0.5">Intelligence Engine</p>
                </div>
                <div class="nav-icon">
                    <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M12 19l7-7 3 3-7 7-3-3z"></path>
                        <path d="M18 13l-1.5-7.5L2 2l3.5 14.5L13 18l5-5z"></path>
                        <path d="M2 2l7.586 7.586"></path>
                        <circle cx="11" cy="11" r="2"></circle>
                    </svg>
                </div>
            </div>
        </div>

        <!-- Vocal Topography Section -->
        <div class="p-4">
            <div class="section-header">
                <span class="section-title">Vocal Topography</span>
                <div class="nav-icon w-8 h-8" style="font-size: 0.875rem;">
                    <span class="text-cyan-400">R</span>
                </div>
            </div>
            
            <div class="sidebar-card mb-3">
                <div class="relative">
                    <img src="/vocal_topography.png" alt="English Vocal" class="sidebar-card-image opacity-75" />
                    <div class="absolute inset-0 bg-gradient-to-t from-[#0d1117] via-transparent to-transparent"></div>
                    <div class="absolute top-3 left-1/2 -translate-x-1/2">
                        <span class="language-pill">English</span>
                    </div>
                </div>
            </div>
            
            <div class="sidebar-card">
                <div class="relative">
                    <img src="/vocal_topography.png" alt="Urdu Vocal" class="sidebar-card-image opacity-60 hue-rotate-30" />
                    <div class="absolute inset-0 bg-gradient-to-t from-[#0d1117] via-transparent to-transparent"></div>
                    <div class="absolute top-3 left-1/2 -translate-x-1/2">
                        <span class="language-pill">Urdu</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Knowledge Graph Section -->
        <div class="p-4 flex-1">
            <div class="section-header">
                <span class="section-title">Knowledge Graph</span>
                <button class="icon-btn" onclick={() => activeTab = 'settings'}>
                    <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <circle cx="12" cy="12" r="3"></circle>
                        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
                    </svg>
                </button>
            </div>
            
            <div class="sidebar-card h-48">
                <div class="relative h-full">
                    <img src="/knowledge_graph_bg.png" alt="Knowledge Graph" class="w-full h-full object-cover opacity-70" />
                    <div class="absolute inset-0 bg-gradient-to-t from-[#0d1117] via-transparent to-transparent"></div>
                    <div class="absolute top-3 left-3 right-3">
                        <span class="text-sm font-medium text-cyan-300">Digital Mycelium</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Navigation Icons -->
        <div class="p-4 flex justify-center gap-4 border-t border-cyan-500/10">
            <button class="nav-icon {activeTab === 'transcript' ? 'active' : ''}" onclick={() => activeTab = 'transcript'}>
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="4 17 10 11 4 5"></polyline>
                    <line x1="12" y1="19" x2="20" y2="19"></line>
                </svg>
            </button>
            <button class="nav-icon {activeTab === 'graph' ? 'active' : ''}" onclick={() => activeTab = 'graph'}>
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polygon points="12 2 2 7 12 12 22 7 12 2"></polygon>
                    <polyline points="2 17 12 22 22 17"></polyline>
                    <polyline points="2 12 12 17 22 12"></polyline>
                </svg>
            </button>
            <button class="nav-icon {activeTab === 'analytics' ? 'active' : ''}" onclick={() => activeTab = 'analytics'}>
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <polyline points="22 12 18 12 15 21 9 3 6 12 2 12"></polyline>
                </svg>
            </button>
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
            <div class="flex items-center gap-3">
                <span class="status-dot {isRecording ? 'status-dot-recording' : 'status-dot-ready'}"></span>
                <span class="text-sm font-medium text-slate-300">{status}</span>
                {#if isRecording}
                    <span class="badge-error text-xs px-2 py-1 rounded">LIVE</span>
                {/if}
            </div>

            <button
                class="{isRecording ? 'btn-recording' : 'btn-primary'}"
                onclick={toggleCapture}
            >
                {isRecording ? "Stop Recording" : "Start Recording"}
            </button>
        </div>

        <!-- CONTENT AREA -->
        <div class="flex-1 overflow-auto p-6">
            <div class="max-w-5xl mx-auto space-y-6">
                {#if !isRunningInTauri}
                    <div class="glass-card p-4 flex items-center gap-3 border-yellow-500/30">
                        <span class="text-2xl">⚠️</span>
                        <div class="flex-1">
                            <p class="font-bold text-yellow-400">Web Preview Mode</p>
                            <p class="text-sm text-slate-400">Native features disabled. Run <code class="bg-dark-700 px-1 rounded">npm run tauri dev</code> for full functionality.</p>
                        </div>
                    </div>
                {/if}

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
                            <button class="icon-btn">
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
                    <div class="content-card">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200">Psychosomatic Engine</span>
                            <button class="icon-btn">
                                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                                    <circle cx="12" cy="12" r="1"></circle>
                                    <circle cx="19" cy="12" r="1"></circle>
                                    <circle cx="5" cy="12" r="1"></circle>
                                </svg>
                            </button>
                        </div>
                        
                        <div class="p-6">
                            <h2 class="text-3xl font-light text-slate-100 text-center mb-6 text-glow-cyan">
                                Transcription
                            </h2>
                            
                            {#if transcripts.length === 0}
                                <div class="text-center py-8">
                                    <p class="text-lg text-slate-400">No transcripts yet. Start recording to begin.</p>
                                </div>
                            {:else}
                                <div class="space-y-4 max-h-64 overflow-y-auto">
                                    {#each transcripts.slice(-5).reverse() as t (t.id)}
                                        <div class="transcription-area">
                                            <p class="transcription-text">
                                                {t.text}
                                            </p>
                                            <div class="flex items-center gap-2 mt-3 text-xs text-slate-500">
                                                <span>{t.speaker}</span>
                                                <span>•</span>
                                                <span>{t.timestamp}</span>
                                                {#if t.tone}
                                                    <span class="badge-cyan ml-2">{t.tone}</span>
                                                {/if}
                                            </div>
                                        </div>
                                    {/each}
                                </div>
                            {/if}
                            
                            <!-- Cyber Eye Visual -->
                            <div class="flex justify-end mt-6">
                                <div class="w-24 h-24 rounded-lg overflow-hidden border border-cyan-500/20">
                                    <img src="/cyber_eye.png" alt="AI Vision" class="w-full h-full object-cover opacity-70" />
                                </div>
                            </div>
                        </div>
                    </div>

                {:else if activeTab === "graph"}
                    <div class="content-card h-[500px]">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200">Knowledge Graph Visualization</span>
                            <button class="icon-btn" onclick={simulateIntelligence}>
                                <span class="text-cyan-400">+ Add Node</span>
                            </button>
                        </div>
                        <div class="h-full p-4">
                            <KnowledgeGraph nodes={graphNodes} edges={graphEdges} />
                        </div>
                    </div>

                {:else if activeTab === "alerts"}
                    <div class="content-card">
                        <div class="content-card-header">
                            <span class="text-sm font-medium text-slate-200">🔔 Intelligence Alerts</span>
                            <button class="btn-ghost text-xs" onclick={() => alerts = []}>Clear All</button>
                        </div>
                        <div class="p-6 space-y-3 max-h-96 overflow-y-auto">
                            {#if alerts.length === 0}
                                <div class="text-center py-12 text-slate-500">
                                    <span class="text-4xl mb-4 block">🔕</span>
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
                                            <span class="text-lg">
                                                {alert.severity === 'critical' ? '🚨' : alert.severity === 'warning' ? '⚠️' : 'ℹ️'}
                                            </span>
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
                                    {isGeminiConnected ? '✓' : '✗'}
                                </div>
                                <div class="text-xs text-slate-500">API Connected</div>
                            </div>
                            <div>
                                <div class="text-2xl font-bold {isRecording ? 'text-red-400' : 'text-slate-500'}">
                                    {isRecording ? '🔴' : '⚪'}
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
                            <label class="block text-xs text-slate-400 mb-2">AI Model</label>
                            <select bind:value={selectedModel} class="select-field w-full">
                                {#each availableModels as model}
                                    <option value={model.id}>{model.name}</option>
                                {/each}
                            </select>
                        </div>
                        
                        <!-- API Key -->
                        <div class="mb-6">
                            <label class="block text-xs text-slate-400 mb-2">Gemini API Key</label>
                            <div class="flex gap-2">
                                <input
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
                                    {isGeminiConnected ? "✓ Connected" : "Connect"}
                                </button>
                            </div>
                        </div>
                        
                        <!-- Capture Source -->
                        <div class="mb-6">
                            <label class="block text-xs text-slate-400 mb-2">Capture Source</label>
                            <div class="flex gap-2">
                                <button
                                    class="{captureMode === 'mic' ? 'btn-primary' : 'btn-secondary'} flex-1"
                                    onclick={() => setCaptureMode('mic')}
                                >
                                    🎤 Mic
                                </button>
                                <button
                                    class="{captureMode === 'system' ? 'btn-primary' : 'btn-secondary'} flex-1"
                                    onclick={() => setCaptureMode('system')}
                                >
                                    🔊 System
                                </button>
                                <button
                                    class="{captureMode === 'both' ? 'btn-primary' : 'btn-secondary'} flex-1"
                                    onclick={() => setCaptureMode('both')}
                                >
                                    📻 Both
                                </button>
                            </div>
                        </div>
                        
                        <!-- Audio Level -->
                        <div class="mb-6">
                            <label class="block text-xs text-slate-400 mb-2">Audio Level</label>
                            <div class="h-3 bg-dark-700 rounded-full overflow-hidden">
                                <div 
                                    class="h-full transition-all duration-75 {currentVolume > 0.5 ? 'bg-red-500' : currentVolume > 0.1 ? 'bg-green-500' : 'bg-slate-600'}"
                                    style="width: {Math.min(currentVolume * 500, 100)}%"
                                ></div>
                            </div>
                            <div class="text-xs text-slate-500 mt-1 text-right">
                                {isRecording ? (currentVolume * 100).toFixed(1) + ' dB' : 'Idle'}
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
            <button class="btn-action" onclick={simulateIntelligence}>
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect>
                    <line x1="9" y1="3" x2="9" y2="21"></line>
                </svg>
                Collapse State
            </button>
            <button class="btn-action" onclick={simulateIntelligence}>
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
                Extract Memories
            </button>
            <button class="btn-action">
                <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
                    <polyline points="14 2 14 8 20 8"></polyline>
                    <line x1="16" y1="13" x2="8" y2="13"></line>
                    <line x1="16" y1="17" x2="8" y2="17"></line>
                    <polyline points="10 9 9 9 8 9"></polyline>
                </svg>
                Summary
            </button>
            
            <!-- Diamond Icon -->
            <div class="ml-4 diamond-icon">
                <span class="text-cyan-400">◆</span>
            </div>
        </div>
    </div>
</div>
