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

    // Core Data
    let transcripts: Array<{
        timestamp: string;
        speaker: string; // "User" or "Gemini"
        text: string;
        tone?: string;
        category?: string[];
        confidence?: number;
    }> = [];

    let activeTab: "transcript" | "graph" | "settings" | "diagnostics" =
        "transcript";

    // Graph Data
    let graphNodes: Array<{ id: string; type: string }> = [];
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

    async function loadDevices() {
        try {
            devices = await invoke("list_audio_devices");
        } catch (error) {
            console.error(error);
            status = "Error listing devices";
        }
    }

    async function toggleCapture() {
        try {
            if (isRecording) {
                await invoke("stop_audio_capture");
                isRecording = false;
                status = "Idle";
            } else {
                await invoke("start_audio_capture");
                isRecording = true;
                status = "Recording...";
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
        try {
            status = "Connecting...";
            await invoke("test_gemini_connection", { apiKey });
        } catch (error) {
            console.error(error);
            status = "Connection Failed";
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
        loadDevices();

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
            transcripts = [
                ...transcripts,
                {
                    timestamp: new Date().toLocaleTimeString([], {
                        hour: "2-digit",
                        minute: "2-digit",
                    }),
                    speaker: "Speaker",
                    text: text,
                    confidence: 0.0, // Placeholder until intelligence update
                },
            ];
        });

        unlistenIntelligence = await listen("god:intelligence", (event) => {
            const intel = event.payload as any;
            console.log("Intelligence:", intel);
            // Logic to update existing transcript or add new structured item could go here
        });
    });

    onDestroy(() => {
        if (unlistenStatus) unlistenStatus();
        if (unlistenTranscript) unlistenTranscript();
        if (unlistenIntelligence) unlistenIntelligence();
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
                Knowledge Graph
            </button>
            <button
                class="w-full text-left px-3 py-2 rounded-md text-sm font-medium transition-colors {activeTab ===
                'diagnostics'
                    ? 'bg-slate-100 text-slate-900'
                    : 'text-slate-600 hover:bg-slate-50 hover:text-slate-900'}"
                onclick={() => (activeTab = "diagnostics")}
            >
                Diagnostics
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

            <!-- Quick connection controls -->
            <div
                class="mt-6 p-3 bg-slate-50 rounded-md border border-slate-100"
            >
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
                    placeholder="Start typing..."
                />

                <button
                    class="w-full btn-secondary text-xs"
                    onclick={connectGemini}
                    disabled={isGeminiConnected}
                >
                    {isGeminiConnected ? "Connected" : "Connect"}
                </button>
            </div>

            <!-- Audio Device Selection Placeholder in Sidebar or Settings -->
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
                {#if activeTab === "transcript"}
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
                        <div class="space-y-4 pb-12">
                            {#each transcripts as t}
                                <div
                                    class="bg-white p-4 rounded-lg border border-slate-200 shadow-sm hover:shadow-md transition-shadow"
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
                                        </div>
                                        <div class="flex gap-2">
                                            {#if t.tone}
                                                <span
                                                    class="badge bg-slate-100 text-slate-600"
                                                    >{t.tone}</span
                                                >
                                            {/if}
                                        </div>
                                    </div>
                                    <p
                                        class="text-slate-700 leading-relaxed text-sm lg:text-base"
                                    >
                                        {t.text}
                                    </p>
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
