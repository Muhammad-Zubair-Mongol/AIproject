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
    <div class="card">
        <h3 class="text-sm font-semibold text-slate-800 mb-3">System Health</h3>
        <div class="space-y-3">
            <div class="flex justify-between items-center text-sm">
                <span class="text-slate-500">Audio Engine</span>
                <span
                    class="badge {isRecording
                        ? 'bg-green-100 text-green-700'
                        : 'bg-slate-100 text-slate-500'}"
                >
                    {isRecording ? "Active" : "Standby"}
                </span>
            </div>
            <div class="flex justify-between items-center text-sm">
                <span class="text-slate-500">Gemini Connection</span>
                <span
                    class="badge {isGeminiConnected
                        ? 'bg-green-100 text-green-700'
                        : 'bg-red-100 text-red-700'}"
                >
                    {isGeminiConnected ? "Connected" : "Offline"}
                </span>
            </div>
            <div class="flex justify-between items-center text-sm">
                <span class="text-slate-500">WebSocket</span>
                <span class="text-slate-700 font-mono"
                    >wss://generativelanguage...</span
                >
            </div>
        </div>
    </div>

    <!-- Metrics -->
    <div class="card">
        <h3 class="text-sm font-semibold text-slate-800 mb-3">
            Performance Metrics
        </h3>
        <div class="grid grid-cols-2 gap-4">
            <div class="p-3 bg-slate-50 rounded border border-slate-100">
                <div class="text-xs text-slate-500 mb-1">Frame Rate</div>
                <div class="text-lg font-bold text-slate-800">
                    {fps}
                    <span class="text-xs font-normal text-slate-400">FPS</span>
                </div>
            </div>
            <div class="p-3 bg-slate-50 rounded border border-slate-100">
                <div class="text-xs text-slate-500 mb-1">Buffer Size</div>
                <div class="text-lg font-bold text-slate-800">
                    {audioBufferSize}
                    <span class="text-xs font-normal text-slate-400"
                        >samples</span
                    >
                </div>
            </div>
            <div class="p-3 bg-slate-50 rounded border border-slate-100">
                <div class="text-xs text-slate-500 mb-1">Sample Rate</div>
                <div class="text-lg font-bold text-slate-800">
                    {sampleRate / 1000}
                    <span class="text-xs font-normal text-slate-400">kHz</span>
                </div>
            </div>
            <div class="p-3 bg-slate-50 rounded border border-slate-100">
                <div class="text-xs text-slate-500 mb-1">
                    Backplane Load (Est)
                </div>
                <div class="text-lg font-bold text-slate-800">{cpuUsage}%</div>
            </div>
        </div>
    </div>
</div>
