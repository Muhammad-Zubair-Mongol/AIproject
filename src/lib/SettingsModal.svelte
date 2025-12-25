<script lang="ts">
    import { onMount, createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";

    export let isOpen = false;

    const dispatch = createEventDispatcher();

    // Audio devices
    let audioDevices: string[] = [];
    let selectedDevice = "";
    let isLoadingDevices = true;

    // Test microphone
    let isTesting = false;
    let testVolume = 0;
    let testInterval: ReturnType<typeof setInterval> | null = null;

    // Settings
    let captureMode = "both";
    let confidenceThreshold = 0.7;
    let vadSensitivity = 0.5;
    let enableDebugMode = false;
    let autoConnect = false;

    // Intelligence Filters
    let filters = {
        tasks: true,
        decisions: true,
        deadlines: true,
        actionItems: true,
        risks: true,
        urgency: true,
        sentiment: false,
        interruptions: false,
        agreement: false,
        disagreement: false,
        emotionShifts: false,
        topicDrifts: false
    };

    // === VAD CONFIGURATION ===
    import { vadManager } from './vadManager';
    
    let vadConfig = vadManager.getConfig();
    let vadMinSpeech = vadConfig.minSpeechDuration / 1000; // Convert to seconds for UI
    let vadSilenceTime = vadConfig.silenceDuration / 1000;
    let vadMinChunk = vadConfig.minChunkDuration / 1000;
    let enableFillerDetection = vadConfig.enableFillerDetection;

    // === API KEYS MANAGEMENT ===
    import { keyManager, type ApiKey, type KeyManagerState } from './keyManager';
    
    let keyState: KeyManagerState = keyManager.getState();
    let apiKeys: ApiKey[] = [];
    let newKeyInput = "";
    let newKeyName = "";
    let shuffleMode = false;
    let isTestingConnection = false;
    let connectionTestResult: "success" | "error" | null = null;
    let connectionTestMessage = "";

    // Subscribe to key manager updates
    $: {
        if (isOpen) {
            const unsubscribe = keyManager.subscribe((state) => {
                keyState = state;
                apiKeys = state.keys;
                shuffleMode = state.shuffleMode;
            });
        }
    }

    // AI Model
    let selectedModel = "gemini-2.5-flash-preview-09-2025";
    const availableModels = [
        { id: "gemini-2.5-flash-preview-09-2025", name: "⚡ Gemini 2.5 Flash" },
        { id: "gemini-2.5-flash-lite-preview-09-2025", name: "🔥 Gemini 2.5 Flash Lite" },
        { id: "gemini-3-flash-preview", name: "💎 Gemini 3 Flash Preview" },
    ];

    // Permissions status
    let micPermission: "granted" | "denied" | "unknown" = "unknown";

    // === API KEY FUNCTIONS ===
    function addApiKey() {
        if (!newKeyInput.trim()) return;
        keyManager.addKey(newKeyInput.trim(), newKeyName.trim() || undefined);
        newKeyInput = "";
        newKeyName = "";
    }

    function removeApiKey(id: string) {
        keyManager.removeKey(id);
    }

    function toggleShuffleMode() {
        keyManager.setShuffleMode(!shuffleMode);
    }

    function loadApiKeys() {
        // Key manager auto-loads on construction
        const state = keyManager.getState();
        apiKeys = state.keys;
        shuffleMode = state.shuffleMode;
    }

    function maskKey(key: string): string {
        return keyManager.maskKey(key);
    }

    function getActiveKey(): string | null {
        const current = keyManager.getCurrentKey();
        return current?.key || null;
    }

    async function testConnection() {
        // Get next key using rotation
        const keyObj = keyManager.getNextKey();
        if (!keyObj) {
            connectionTestResult = "error";
            connectionTestMessage = "No API keys configured";
            return;
        }

        isTestingConnection = true;
        connectionTestResult = null;
        connectionTestMessage = `Testing ${keyObj.name}...`;

        try {
            const result = await invoke("test_gemini_connection", { 
                key: keyObj.key,
                model: selectedModel 
            });
            connectionTestResult = "success";
            connectionTestMessage = `✓ Connected via ${keyObj.name}!`;
            
            // Report success to key manager
            keyManager.reportSuccess();
            
            // Dispatch connected event
            dispatch("connected", { key: keyObj.key, model: selectedModel });
        } catch (error: any) {
            const errorStr = String(error);
            const errorCode = errorStr.includes("429") ? 429 : 
                              errorStr.includes("401") ? 401 :
                              errorStr.includes("500") ? 500 : 0;
            
            // Handle error with key manager
            const result = keyManager.handleError(errorCode, errorStr);
            
            connectionTestResult = "error";
            connectionTestMessage = result.message;
            
            // If switched, show which key
            if (result.switched && result.newKey) {
                connectionTestMessage = `⚠ ${result.message}`;
            }
        } finally {
            isTestingConnection = false;
        }
    }

    // === AUDIO FUNCTIONS ===
    async function loadDevices() {
        isLoadingDevices = true;
        try {
            audioDevices = await invoke("list_audio_devices");
            if (audioDevices.length > 0 && !selectedDevice) {
                selectedDevice = audioDevices[0];
            }
            micPermission = "granted";
        } catch (e) {
            console.error("Failed to load devices:", e);
            micPermission = "denied";
        } finally {
            isLoadingDevices = false;
        }
    }

    async function testMicrophone() {
        if (isTesting) {
            if (testInterval) clearInterval(testInterval);
            testInterval = null;
            isTesting = false;
            testVolume = 0;
            try { await invoke("stop_audio_capture"); } catch (e) {}
            return;
        }

        isTesting = true;
        try {
            await invoke("start_audio_capture");
            testInterval = setInterval(async () => {
                try {
                    testVolume = await invoke("get_current_volume");
                } catch (e) {}
            }, 100);
        } catch (e) {
            console.error("Failed to start test:", e);
            isTesting = false;
        }
    }

    async function stopTest() {
        if (testInterval) clearInterval(testInterval);
        testInterval = null;
        isTesting = false;
        testVolume = 0;
        try { await invoke("stop_audio_capture"); } catch (e) {}
    }

    async function setCaptureMode(mode: string) {
        captureMode = mode;
        try {
            await invoke("set_capture_mode", { mode });
        } catch (e) {
            console.error("Failed to set capture mode:", e);
        }
    }

    function saveSettings() {
        localStorage.setItem("gemini_model", selectedModel);
        localStorage.setItem("confidence_threshold", confidenceThreshold.toString());
        localStorage.setItem("vad_sensitivity", vadSensitivity.toString());
        localStorage.setItem("debug_mode", enableDebugMode.toString());
        localStorage.setItem("auto_connect", autoConnect.toString());
        localStorage.setItem("intelligence_filters", JSON.stringify(filters));
        
        // Save VAD configuration
        vadManager.setConfig({
            minSpeechDuration: vadMinSpeech * 1000,
            silenceDuration: vadSilenceTime * 1000,
            minChunkDuration: vadMinChunk * 1000,
            enableFillerDetection
        });
        
        dispatch("save", { 
            selectedModel, 
            confidenceThreshold, 
            vadSensitivity,
            enableDebugMode, 
            autoConnect,
            filters,
            apiKey: getActiveKey()
        });
        close();
    }

    function close() {
        if (isTesting) stopTest();
        dispatch("close");
    }

    function checkPermissions() {
        loadDevices();
    }

    onMount(() => {
        if (isOpen) {
            loadDevices();
            loadApiKeys();
            selectedModel = localStorage.getItem("gemini_model") || selectedModel;
            confidenceThreshold = parseFloat(localStorage.getItem("confidence_threshold") || "0.7");
            vadSensitivity = parseFloat(localStorage.getItem("vad_sensitivity") || "0.5");
            enableDebugMode = localStorage.getItem("debug_mode") === "true";
            autoConnect = localStorage.getItem("auto_connect") === "true";
            
            // Load intelligence filters
            try {
                const storedFilters = localStorage.getItem("intelligence_filters");
                if (storedFilters) {
                    filters = { ...filters, ...JSON.parse(storedFilters) };
                }
            } catch (e) {
                console.error("Failed to load filters:", e);
            }
        }
    });

    $: if (isOpen) {
        loadDevices();
        loadApiKeys();
        // Reload settings when modal opens
        selectedModel = localStorage.getItem("gemini_model") || selectedModel;
        confidenceThreshold = parseFloat(localStorage.getItem("confidence_threshold") || "0.7");
        vadSensitivity = parseFloat(localStorage.getItem("vad_sensitivity") || "0.5");
        try {
            const storedFilters = localStorage.getItem("intelligence_filters");
            if (storedFilters) {
                filters = { ...filters, ...JSON.parse(storedFilters) };
            }
        } catch (e) {}
    }
</script>

{#if isOpen}
    <!-- Backdrop -->
    <div 
        class="fixed inset-0 bg-black/80 backdrop-blur-sm z-50 animate-fadeIn"
        onclick={close}
        role="presentation"
    ></div>

    <!-- Modal -->
    <div class="fixed inset-0 z-50 flex items-center justify-center p-4 pointer-events-none">
        <div 
            class="glass-card w-full max-w-2xl max-h-[90vh] overflow-y-auto pointer-events-auto animate-scaleIn"
            onclick={(e) => e.stopPropagation()}
            role="dialog"
            tabindex="-1"
            aria-labelledby="settings-title"
            onkeydown={(e) => { if (e.key === 'Escape') close(); }}
        >
            <!-- Header -->
            <div class="flex items-center justify-between p-6 border-b border-cyan-500/10">
                <h2 id="settings-title" class="text-xl font-bold text-slate-100 flex items-center gap-2">
                    <svg class="w-6 h-6 text-cyan-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                    </svg>
                    Settings
                </h2>
                <button 
                    class="text-slate-400 hover:text-white transition-colors p-2"
                    onclick={close}
                    aria-label="Close settings"
                >
                    <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            </div>

            <div class="p-6 space-y-8">
                <!-- === API KEYS SECTION === -->
                <section>
                    <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider mb-4 flex items-center gap-2">
                        <span>🔑</span> API Keys
                    </h3>

                    <!-- Existing Keys List -->
                    {#if apiKeys.length > 0}
                        <div class="space-y-2 mb-4">
                            {#each apiKeys as apiKey, i}
                                <div class="flex items-center gap-3 p-3 rounded-lg bg-dark-700/50 border 
                                    {apiKey.isActive ? 'border-cyan-500/50 bg-cyan-500/5' : 
                                     apiKey.isDisabled ? 'border-red-500/30 opacity-50' : 
                                     apiKey.rateLimited ? 'border-yellow-500/30' : 'border-cyan-500/10'}">
                                    
                                    <!-- Status indicator -->
                                    <div class="w-3 h-3 rounded-full flex-shrink-0
                                        {apiKey.isActive ? 'bg-green-500 animate-pulse' : 
                                         apiKey.isDisabled ? 'bg-red-500' :
                                         apiKey.rateLimited ? 'bg-yellow-500' : 'bg-slate-600'}">
                                    </div>
                                    
                                    <div class="flex-1 min-w-0">
                                        <div class="flex items-center gap-2 flex-wrap">
                                            <span class="text-sm font-medium text-slate-200">{apiKey.name}</span>
                                            
                                            <!-- Primary Key Badge -->
                                            {#if apiKey.isPrimary}
                                                <span class="text-xs px-1.5 py-0.5 rounded bg-purple-500/20 text-purple-400">Primary</span>
                                            {/if}
                                            
                                            <!-- Status Badges -->
                                            {#if apiKey.isActive}
                                                <span class="text-xs px-1.5 py-0.5 rounded bg-green-500/20 text-green-400">Active</span>
                                            {/if}
                                            {#if apiKey.rateLimited}
                                                <span class="text-xs px-1.5 py-0.5 rounded bg-yellow-500/20 text-yellow-400">Cooldown</span>
                                            {/if}
                                            {#if apiKey.isDisabled}
                                                <span class="text-xs px-1.5 py-0.5 rounded bg-red-500/20 text-red-400">Disabled</span>
                                            {/if}
                                        </div>
                                        <div class="flex items-center gap-2 mt-1">
                                            <span class="text-xs text-slate-500 font-mono">{maskKey(apiKey.key)}</span>
                                            {#if apiKey.usageCount > 0}
                                                <span class="text-xs text-slate-600">• {apiKey.usageCount} calls</span>
                                            {/if}
                                        </div>
                                    </div>
                                    
                                    <button
                                        class="text-slate-400 hover:text-red-400 transition-colors p-1"
                                        onclick={() => removeApiKey(apiKey.id)}
                                        aria-label="Delete key"
                                    >
                                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                                        </svg>
                                    </button>
                                </div>
                            {/each}
                        </div>
                    {:else}
                        <div class="p-4 rounded-lg bg-yellow-500/10 border border-yellow-500/30 mb-4">
                            <p class="text-yellow-400 text-sm">⚠️ No API keys configured. Add a Gemini API key to enable AI features.</p>
                        </div>
                    {/if}

                    <!-- Add New Key -->
                    <div class="p-4 rounded-lg bg-dark-700/30 border border-cyan-500/10">
                        <p class="text-xs text-slate-400 mb-3">Add multiple keys for automatic rotation on rate limits</p>
                        <div class="space-y-2">
                            <div class="flex gap-2">
                                <input
                                    type="text"
                                    placeholder="Key name (optional)"
                                    bind:value={newKeyName}
                                    class="input-field w-40 text-sm"
                                />
                            </div>
                            <div class="flex gap-2">
                                <input
                                    type="password"
                                    placeholder="Paste Gemini API key here..."
                                    bind:value={newKeyInput}
                                    class="input-field flex-1 text-sm font-mono"
                                />
                                <button
                                    class="btn-secondary text-sm px-4 whitespace-nowrap"
                                    onclick={addApiKey}
                                    disabled={!newKeyInput.trim()}
                                >
                                    + Add Key
                                </button>
                            </div>
                        </div>
                    </div>

                    <!-- Test Connection & Options -->
                    <div class="mt-4 space-y-3">
                        <div class="flex items-center gap-3">
                            <button
                                class="btn-primary text-sm"
                                onclick={testConnection}
                                disabled={isTestingConnection || apiKeys.length === 0}
                            >
                                {isTestingConnection ? '⏳ Testing...' : '🔌 Test Connection'}
                            </button>
                            
                            {#if connectionTestResult}
                                <span class="text-sm {connectionTestResult === 'success' ? 'text-green-400' : 'text-yellow-400'}">
                                    {connectionTestMessage}
                                </span>
                            {/if}
                        </div>

                        <!-- Rotation Mode Toggle -->
                        {#if apiKeys.length > 1}
                            <div class="flex items-center justify-between p-3 rounded-lg bg-dark-700/30 border border-cyan-500/10">
                                <div>
                                    <span class="text-sm text-slate-300">Key Rotation Mode</span>
                                    <p class="text-xs text-slate-500">
                                        {shuffleMode ? 'Random key selection for better distribution' : 'Sequential round-robin rotation'}
                                    </p>
                                </div>
                                <button 
                                    class="px-3 py-1 text-xs rounded {shuffleMode ? 'bg-purple-500/20 text-purple-400' : 'bg-cyan-500/20 text-cyan-400'}"
                                    onclick={toggleShuffleMode}
                                >
                                    {shuffleMode ? '🔀 Shuffle' : '🔁 Sequential'}
                                </button>
                            </div>
                        {/if}

                        <!-- Key Stats -->
                        {#if apiKeys.length > 0}
                            <div class="text-xs text-slate-500 flex items-center gap-4">
                                <span>📊 {keyState.totalCalls} total API calls</span>
                                <span>🔑 {keyManager.getActiveKeyCount()}/{apiKeys.length} keys available</span>
                            </div>
                        {/if}
                    </div>
                </section>

                <!-- === AUDIO INPUT SECTION === -->
                <section>
                    <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider mb-4 flex items-center gap-2">
                        <span>🎤</span> Audio Input
                    </h3>

                    <!-- Permission Status -->
                    <div class="flex items-center justify-between mb-4 p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                        <span class="text-sm text-slate-300">Microphone Permission</span>
                        {#if micPermission === "granted"}
                            <span class="badge-success text-xs">✓ Granted</span>
                        {:else if micPermission === "denied"}
                            <span class="badge-error text-xs">✗ Denied</span>
                        {:else}
                            <span class="badge-cyan text-xs">? Unknown</span>
                        {/if}
                        <button class="btn-ghost text-xs" onclick={checkPermissions}>
                            Refresh
                        </button>
                    </div>

                    <!-- Device Selection -->
                    <div class="mb-4">
                        <label for="device-select" class="block text-xs text-slate-400 mb-2">
                            Select Microphone
                        </label>
                        {#if isLoadingDevices}
                            <div class="input-field text-slate-500">Loading devices...</div>
                        {:else}
                            <select 
                                id="device-select"
                                bind:value={selectedDevice} 
                                class="select-field w-full"
                            >
                                {#each audioDevices as device}
                                    <option value={device}>{device}</option>
                                {/each}
                            </select>
                        {/if}
                    </div>

                    <!-- Capture Mode -->
                    <div class="mb-4">
                        <div class="block text-xs text-slate-400 mb-2">Capture Source</div>
                        <div class="grid grid-cols-3 gap-2">
                            <button 
                                class="{captureMode === 'mic' ? 'btn-primary' : 'btn-secondary'}"
                                onclick={() => setCaptureMode('mic')}
                            >
                                🎤 Microphone
                            </button>
                            <button 
                                class="{captureMode === 'system' ? 'btn-primary' : 'btn-secondary'}"
                                onclick={() => setCaptureMode('system')}
                            >
                                🔊 System
                            </button>
                            <button 
                                class="{captureMode === 'both' ? 'btn-primary' : 'btn-secondary'}"
                                onclick={() => setCaptureMode('both')}
                            >
                                📻 Both
                            </button>
                        </div>
                    </div>

                    <!-- Test Microphone -->
                    <div class="p-4 rounded-lg bg-dark-700/30 border border-cyan-500/10">
                        <div class="flex items-center justify-between mb-3">
                            <span class="text-sm text-slate-300">Test Microphone</span>
                            <button 
                                class="{isTesting ? 'btn-recording' : 'btn-secondary'} text-sm"
                                onclick={testMicrophone}
                            >
                                {isTesting ? '⏹ Stop Test' : '🎙 Start Test'}
                            </button>
                        </div>
                        
                        {#if isTesting}
                            <div class="space-y-2">
                                <div class="h-4 bg-dark-600 rounded-full overflow-hidden">
                                    <div 
                                        class="h-full transition-all duration-75 {testVolume > 0.15 ? 'bg-red-500' : testVolume > 0.05 ? 'bg-green-500' : 'bg-cyan-500'}"
                                        style="width: {Math.min(testVolume * 10, 1) * 100}%"
                                    ></div>
                                </div>
                                <p class="text-xs text-slate-400 text-center">
                                    {testVolume > 0.02 ? '✓ Audio detected!' : 'Speak into your microphone...'}
                                </p>
                            </div>
                        {:else}
                            <p class="text-xs text-slate-500">Click to test your microphone before recording.</p>
                        {/if}
                    </div>
                </section>

                <!-- === AI ENGINE SECTION === -->
                <section>
                    <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider mb-4 flex items-center gap-2">
                        <span>🤖</span> AI Engine
                    </h3>

                    <div class="mb-4">
                        <label for="model-select" class="block text-xs text-slate-400 mb-2">
                            Transcription Model
                        </label>
                        <select 
                            id="model-select"
                            bind:value={selectedModel} 
                            class="select-field w-full"
                        >
                            {#each availableModels as model}
                                <option value={model.id}>{model.name}</option>
                            {/each}
                        </select>
                    </div>

                    <div class="mb-4">
                        <label for="confidence" class="block text-xs text-slate-400 mb-2">
                            Confidence Threshold: <span class="text-cyan-400">{(confidenceThreshold * 100).toFixed(0)}%</span>
                        </label>
                        <input 
                            id="confidence"
                            type="range" 
                            min="0" 
                            max="1" 
                            step="0.05"
                            bind:value={confidenceThreshold}
                            class="w-full"
                        />
                    </div>

                    <div class="mb-4">
                        <label for="vad" class="block text-xs text-slate-400 mb-2">
                            Voice Activity Sensitivity: <span class="text-cyan-400">{vadSensitivity.toFixed(1)}</span>
                        </label>
                        <input 
                            id="vad"
                            type="range" 
                            min="0" 
                            max="1" 
                            step="0.1"
                            bind:value={vadSensitivity}
                            class="w-full"
                        />
                    </div>

                    <div class="flex items-center gap-3">
                        <input 
                            type="checkbox" 
                            id="auto-connect"
                            bind:checked={autoConnect}
                        />
                        <label for="auto-connect" class="text-sm text-slate-300">
                            Auto-connect to AI on startup
                        </label>
                    </div>
                </section>

                <!-- === INTELLIGENCE FILTERS === -->
                <section>
                    <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider mb-4 flex items-center gap-2">
                        <span>🎯</span> Intelligence Filters
                    </h3>
                    <p class="text-xs text-slate-500 mb-4">Select which insights to extract during recording</p>
                    
                    <div class="grid grid-cols-2 gap-3">
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.tasks} />
                            <span class="text-sm text-slate-300">Tasks</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.decisions} />
                            <span class="text-sm text-slate-300">Decisions</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.deadlines} />
                            <span class="text-sm text-slate-300">Deadlines</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.actionItems} />
                            <span class="text-sm text-slate-300">Action Items</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.risks} />
                            <span class="text-sm text-slate-300">Risks</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.urgency} />
                            <span class="text-sm text-slate-300">Urgency</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.sentiment} />
                            <span class="text-sm text-slate-300">Sentiment</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.interruptions} />
                            <span class="text-sm text-slate-300">Interruptions</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.agreement} />
                            <span class="text-sm text-slate-300">Agreement</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.disagreement} />
                            <span class="text-sm text-slate-300">Disagreement</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.emotionShifts} />
                            <span class="text-sm text-slate-300">Emotion Shifts</span>
                        </label>
                        <label class="flex items-center gap-2 cursor-pointer">
                            <input type="checkbox" bind:checked={filters.topicDrifts} />
                            <span class="text-sm text-slate-300">Topic Drifts</span>
                        </label>
                    </div>
                </section>

                <!-- === SMART AUDIO BUFFERING (VAD) === -->
                <section>
                    <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider mb-4 flex items-center gap-2">
                        <span>🎤</span> Smart Audio Buffering
                    </h3>
                    <p class="text-xs text-slate-500 mb-4">Configure intelligent speech detection to reduce API costs</p>

                    <div class="mb-4">
                        <label for="vad-min-speech" class="block text-xs text-slate-400 mb-2">
                            Min Speech Buffer: <span class="text-cyan-400">{vadMinSpeech}s</span>
                        </label>
                        <input 
                            id="vad-min-speech"
                            type="range" 
                            min="3" 
                            max="30" 
                            step="1"
                            bind:value={vadMinSpeech}
                            class="w-full"
                        />
                        <p class="text-xs text-slate-600 mt-1">Accumulate this much speech before sending to AI</p>
                    </div>

                    <div class="mb-4">
                        <label for="vad-silence" class="block text-xs text-slate-400 mb-2">
                            Silence Detection: <span class="text-cyan-400">{vadSilenceTime}s</span>
                        </label>
                        <input 
                            id="vad-silence"
                            type="range" 
                            min="1" 
                            max="5" 
                            step="0.5"
                            bind:value={vadSilenceTime}
                            class="w-full"
                        />
                        <p class="text-xs text-slate-600 mt-1">Pause length to trigger chunk send</p>
                    </div>

                    <div class="mb-4">
                        <label for="vad-min-chunk" class="block text-xs text-slate-400 mb-2">
                            Min Chunk Size: <span class="text-cyan-400">{vadMinChunk}s</span>
                        </label>
                        <input 
                            id="vad-min-chunk"
                            type="range" 
                            min="1" 
                            max="10" 
                            step="1"
                            bind:value={vadMinChunk}
                            class="w-full"
                        />
                        <p class="text-xs text-slate-600 mt-1">Ignore chunks shorter than this</p>
                    </div>

                    <div class="flex items-center gap-3">
                        <input 
                            type="checkbox" 
                            id="filler-detection"
                            bind:checked={enableFillerDetection}
                        />
                        <label for="filler-detection" class="text-sm text-slate-300">
                            Detect & flag filler words (um, uh, like...)
                        </label>
                    </div>
                </section>

                <!-- === DEVELOPER OPTIONS === -->
                <section>
                    <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider mb-4 flex items-center gap-2">
                        <span>🛠</span> Developer Options
                    </h3>

                    <div class="flex items-center gap-3">
                        <input 
                            type="checkbox" 
                            id="debug-mode"
                            bind:checked={enableDebugMode}
                        />
                        <label for="debug-mode" class="text-sm text-slate-300">
                            Enable Debug Mode
                        </label>
                    </div>
                    <p class="text-xs text-slate-500 ml-7 mt-1">
                        Shows raw logs and audio file paths in console
                    </p>
                </section>
            </div>

            <!-- Footer -->
            <div class="p-6 border-t border-cyan-500/10 flex justify-end gap-3">
                <button class="btn-secondary" onclick={close}>
                    Cancel
                </button>
                <button class="btn-primary" onclick={saveSettings}>
                    Save Settings
                </button>
            </div>
        </div>
    </div>
{/if}

<style>
    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }

    @keyframes scaleIn {
        from { 
            opacity: 0; 
            transform: scale(0.95);
        }
        to { 
            opacity: 1; 
            transform: scale(1);
        }
    }

    .animate-fadeIn {
        animation: fadeIn 0.2s ease-out forwards;
    }

    .animate-scaleIn {
        animation: scaleIn 0.3s ease-out forwards;
    }
</style>
