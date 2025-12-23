<script lang="ts">
    import { onMount, onDestroy } from "svelte";

    export let isRecording = false;
    export let isGeminiConnected = false;

    let fps = 60;
    let audioBufferSize = 4096;
    let sampleRate = 48000;
    let cpuUsage = 0;

    // Simulations for values we don't have real bindings for yet
    let interval: any;

    onMount(() => {
        interval = setInterval(() => {
            fps = Math.floor(58 + Math.random() * 4);
            cpuUsage = Math.floor(Math.random() * 15);
        }, 1000);
    });

    onDestroy(() => {
        if (interval) clearInterval(interval);
    });
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
        <div class="space-y-4">
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">Audio Engine</span>
                <span class="badge-cyan {isRecording ? 'bg-green-500/15 text-green-400 border-green-500/30' : ''}">
                    {isRecording ? "Active" : "Standby"}
                </span>
            </div>
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">Gemini Connection</span>
                <span class="badge-cyan {isGeminiConnected ? 'bg-green-500/15 text-green-400 border-green-500/30' : 'bg-red-500/15 text-red-400 border-red-500/30'}">
                    {isGeminiConnected ? "Connected" : "Offline"}
                </span>
            </div>
            <div class="flex justify-between items-center">
                <span class="text-sm text-slate-400">WebSocket</span>
                <span class="text-xs text-cyan-400 font-mono truncate max-w-40">
                    wss://generativelanguage...
                </span>
            </div>
        </div>
    </div>

    <!-- Metrics -->
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
                <div class="text-xl font-bold text-cyan-400">
                    {fps}
                    <span class="text-xs font-normal text-slate-500">FPS</span>
                </div>
            </div>
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Buffer Size</div>
                <div class="text-xl font-bold text-cyan-400">
                    {audioBufferSize}
                    <span class="text-xs font-normal text-slate-500">samples</span>
                </div>
            </div>
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Sample Rate</div>
                <div class="text-xl font-bold text-cyan-400">
                    {sampleRate / 1000}
                    <span class="text-xs font-normal text-slate-500">kHz</span>
                </div>
            </div>
            <div class="p-3 rounded-lg bg-dark-700/50 border border-cyan-500/10">
                <div class="text-xs text-slate-500 mb-1">Backplane Load</div>
                <div class="text-xl font-bold {cpuUsage > 10 ? 'text-yellow-400' : 'text-green-400'}">
                    {cpuUsage}%
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
                <div class="flex-1 h-2 rounded-full {isRecording ? 'bg-gradient-to-r from-cyan-500 to-cyan-400' : 'bg-dark-600'} transition-all"></div>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                    <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <div class="flex-1 h-2 rounded-full {isRecording ? 'bg-gradient-to-r from-cyan-400 to-blue-500' : 'bg-dark-600'} transition-all" style="transition-delay: 100ms"></div>
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="text-cyan-400">
                    <polyline points="9 18 15 12 9 6"></polyline>
                </svg>
                <div class="flex-1 h-2 rounded-full {isGeminiConnected ? 'bg-gradient-to-r from-blue-500 to-purple-500' : 'bg-dark-600'} transition-all" style="transition-delay: 200ms"></div>
            </div>
        </div>
        <div class="flex justify-between text-xs text-slate-500 mt-2">
            <span>Capture</span>
            <span>Process</span>
            <span>Transmit</span>
        </div>
    </div>
</div>
