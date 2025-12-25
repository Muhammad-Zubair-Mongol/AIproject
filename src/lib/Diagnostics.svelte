<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";

    export let isRecording = false;
    export let isGeminiConnected = false;

    // Real metrics from backend
    let audioDevices: string[] = [];
    let captureMode = "mic";
    let currentVolume = 0;
    
    // Performance metrics
    let fps = 60;
    let lastFrameTime = performance.now();
    let frameCount = 0;
    let memoryUsage = 0;
    
    // Connection status
    let apiEndpoint = "generativelanguage.googleapis.com";
    let lastRequestTime = "--";
    let requestCount = 0;

    let interval: ReturnType<typeof setInterval>;
    let animationId: number;

    // Calculate real FPS
    function measureFPS() {
        const now = performance.now();
        frameCount++;
        
        if (now - lastFrameTime >= 1000) {
            fps = Math.round(frameCount * 1000 / (now - lastFrameTime));
            frameCount = 0;
            lastFrameTime = now;
        }
        
        animationId = requestAnimationFrame(measureFPS);
    }

    async function loadDevices() {
        try {
            audioDevices = await invoke("list_audio_devices");
        } catch (e) {
            console.error("Failed to load devices:", e);
        }
    }

    async function pollVolume() {
        if (!isRecording) {
            currentVolume = 0;
            return;
        }
        try {
            currentVolume = await invoke("get_current_volume");
        } catch (e) {
            // Ignore
        }
    }

    onMount(() => {
        loadDevices();
        measureFPS();
        
        interval = setInterval(async () => {
            await pollVolume();
            
            // Estimate memory usage (browser-only approximation)
            if ((performance as any).memory) {
                memoryUsage = Math.round((performance as any).memory.usedJSHeapSize / 1024 / 1024);
            }
            
            if (isRecording) {
                requestCount++;
            }
        }, 500);
    });

    onDestroy(() => {
        if (interval) clearInterval(interval);
        if (animationId) cancelAnimationFrame(animationId);
    });

    $: if (isRecording) {
        lastRequestTime = new Date().toLocaleTimeString();
    }
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
    <!-- Component Status -->
    <div class="glass-card p-6">
        <h3 class="text-sm font-semibold text-slate-200 mb-4 flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                <path d="M22 12h-4l-3 9L9 3l-3 9H2"></path>
            </svg>
            System Health
        </h3>
        <div class="space-y-3">
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">Audio Engine</span>
                <span class="badge-cyan flex items-center gap-1.5 {isRecording ? 'bg-green-500/15 text-green-400 border-green-500/30' : ''}">
                    <span class="w-2 h-2 rounded-full {isRecording ? 'bg-green-500 animate-pulse' : 'bg-slate-500'}"></span>
                    {isRecording ? "Active" : "Standby"}
                </span>
            </div>
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">Gemini API</span>
                <span class="badge-cyan flex items-center gap-1.5 {isGeminiConnected ? 'bg-green-500/15 text-green-400 border-green-500/30' : 'bg-red-500/15 text-red-400 border-red-500/30'}">
                    <svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">{#if isGeminiConnected}<polyline points="20 6 9 17 4 12"/>{:else}<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>{/if}</svg>
                    {isGeminiConnected ? "Connected" : "Offline"}
                </span>
            </div>
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">API Endpoint</span>
                <span class="text-xs text-cyan-400 font-mono truncate max-w-40">
                    {apiEndpoint}
                </span>
            </div>
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">Last Request</span>
                <span class="text-xs text-slate-400 font-mono">
                    {lastRequestTime}
                </span>
            </div>
        </div>
    </div>

    <!-- Performance Metrics -->
    <div class="glass-card p-6">
        <h3 class="text-sm font-semibold text-slate-200 mb-4 flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                <line x1="18" y1="20" x2="18" y2="10"></line>
                <line x1="12" y1="20" x2="12" y2="4"></line>
                <line x1="6" y1="20" x2="6" y2="14"></line>
            </svg>
            Performance Metrics
        </h3>
        <div class="grid grid-cols-2 gap-3">
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Frame Rate</div>
                <div class="text-xl font-bold {fps >= 55 ? 'text-green-400' : fps >= 30 ? 'text-yellow-400' : 'text-red-400'}">
                    {fps}
                    <span class="text-xs font-normal text-slate-500">FPS</span>
                </div>
            </div>
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Audio Level</div>
                <div class="text-xl font-bold text-cyan-400">
                    {(currentVolume * 100).toFixed(0)}
                    <span class="text-xs font-normal text-slate-500">%</span>
                </div>
            </div>
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Sample Rate</div>
                <div class="text-xl font-bold text-cyan-400">
                    16
                    <span class="text-xs font-normal text-slate-500">kHz</span>
                </div>
            </div>
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Request Count</div>
                <div class="text-xl font-bold text-cyan-400">
                    {requestCount}
                </div>
            </div>
        </div>
    </div>

    <!-- Audio Pipeline Status -->
    <div class="glass-card p-6 md:col-span-2">
        <h3 class="text-sm font-semibold text-slate-200 mb-4 flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5"></polygon>
                <path d="M19.07 4.93a10 10 0 0 1 0 14.14M15.54 8.46a5 5 0 0 1 0 7.07"></path>
            </svg>
            Audio Pipeline
        </h3>
        <div class="flex items-center gap-4">
            <!-- Pipeline stages -->
            <div class="flex items-center gap-2 flex-1">
                <!-- Capture Stage -->
                <div class="flex-1 relative">
                    <div class="h-2 rounded-full {isRecording ? 'bg-gradient-to-r from-cyan-500 to-cyan-400' : 'bg-dark-600'} transition-all">
                        {#if isRecording && currentVolume > 0.01}
                            <div 
                                class="absolute inset-0 bg-white/20 rounded-full animate-pulse"
                                style="width: {Math.min(currentVolume * 100, 100)}%"
                            ></div>
                        {/if}
                    </div>
                </div>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400 {isRecording ? 'animate-pulse' : 'opacity-30'}">
                    <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <!-- Process Stage -->
                <div class="flex-1 h-2 rounded-full {isRecording ? 'bg-gradient-to-r from-cyan-400 to-blue-500' : 'bg-dark-600'} transition-all" style="transition-delay: 100ms"></div>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400 {isGeminiConnected ? 'animate-pulse' : 'opacity-30'}">
                    <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <!-- Transmit Stage -->
                <div class="flex-1 h-2 rounded-full {isGeminiConnected ? 'bg-gradient-to-r from-blue-500 to-purple-500' : 'bg-dark-600'} transition-all" style="transition-delay: 200ms"></div>
            </div>
        </div>
        <div class="flex justify-between text-xs text-slate-500 mt-2">
            <span class="flex items-center gap-1">
                <span class="w-1.5 h-1.5 rounded-full {isRecording ? 'bg-cyan-500' : 'bg-slate-600'}"></span>
                Capture
            </span>
            <span class="flex items-center gap-1">
                <span class="w-1.5 h-1.5 rounded-full {isRecording ? 'bg-blue-500' : 'bg-slate-600'}"></span>
                Process
            </span>
            <span class="flex items-center gap-1">
                <span class="w-1.5 h-1.5 rounded-full {isGeminiConnected ? 'bg-purple-500' : 'bg-slate-600'}"></span>
                Transmit
            </span>
        </div>
    </div>

    <!-- Audio Devices -->
    <div class="glass-card p-6 md:col-span-2">
        <h3 class="text-sm font-semibold text-slate-200 mb-4 flex items-center gap-2">
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"></path>
                <path d="M19 10v2a7 7 0 0 1-14 0v-2"></path>
                <line x1="12" y1="19" x2="12" y2="23"></line>
                <line x1="8" y1="23" x2="16" y2="23"></line>
            </svg>
            Audio Devices
        </h3>
        <div class="max-h-32 overflow-y-auto space-y-1">
            {#if audioDevices.length === 0}
                <p class="text-sm text-slate-500">Loading devices...</p>
            {:else}
                {#each audioDevices as device}
                    <div class="text-xs text-slate-400 font-mono py-1 px-2 rounded bg-dark-700/30">
                        {device}
                    </div>
                {/each}
            {/if}
        </div>
    </div>
</div>
