<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { vadManager, type VADState } from "./vadManager";

    export let isRecording = false;
    export let currentVolume = 0;

    let vadState: VADState = vadManager.getState();
    let unsubscribe: (() => void) | null = null;

    // Waveform bars - more for smoother visualization
    const BAR_COUNT = 50;
    let barHistory: { level: number; isSpeech: boolean }[] = [];

    // Volume threshold for speech (calibrates over time)
    const SPEECH_THRESHOLD = 0.015;

    // Update bars based on volume
    $: if (isRecording) {
        // Determine if this is speech
        const isSpeech = currentVolume > SPEECH_THRESHOLD;
        
        // Add to history with slight jitter for natural look
        const jitteredLevel = currentVolume * (0.85 + Math.random() * 0.3);
        barHistory = [...barHistory.slice(-BAR_COUNT + 1), {
            level: jitteredLevel,
            isSpeech
        }];
        
        // Process volume through VAD
        vadManager.processVolume(currentVolume);
    }

    onMount(() => {
        unsubscribe = vadManager.subscribe((state) => {
            vadState = state;
        });
    });

    onDestroy(() => {
        if (unsubscribe) unsubscribe();
    });

    function getBarColor(isSpeech: boolean, level: number): string {
        if (level < 0.003) return 'bg-slate-700'; // Dead silence
        if (level < 0.01) return 'bg-slate-600'; // Very quiet
        if (isSpeech) return 'bg-gradient-to-t from-green-600 to-green-400'; // Active speech - green
        return 'bg-gradient-to-t from-yellow-600 to-yellow-400'; // Some noise - yellow
    }

    function formatTime(ms: number): string {
        const seconds = Math.floor(ms / 1000);
        const minutes = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${minutes}:${secs.toString().padStart(2, '0')}`;
    }

    function getStatusText(): string {
        if (!isRecording) return 'Idle';
        
        switch (vadState.status) {
            case 'buffering':
                return `Buffering... ${formatTime(vadState.bufferDuration)}`;
            case 'sending':
                return '📤 Sending to AI...';
            case 'processing':
                return '⚙️ Processing...';
            default:
                return vadState.isSpeaking ? '🎤 Speech detected' : '👂 Listening...';
        }
    }

    function getStatusColor(): string {
        if (!isRecording) return 'text-slate-500';
        if (vadState.status === 'sending') return 'text-blue-400';
        if (vadState.isSpeaking) return 'text-green-400';
        return 'text-slate-400';
    }

    function getMinBufferPercent(): number {
        const minRequired = vadManager.getConfig().minSpeechDuration;
        return Math.min(100, (vadState.bufferDuration / minRequired) * 100);
    }
</script>

<div class="vad-waveform rounded-xl bg-dark-800/60 border border-cyan-500/20 p-4 backdrop-blur-sm">
    <!-- Header -->
    <div class="flex items-center justify-between mb-3">
        <div class="flex items-center gap-3">
            <!-- Status indicator -->
            <div class="relative">
                <span class="w-3 h-3 rounded-full block {vadState.isSpeaking ? 'bg-green-500' : 'bg-slate-600'}"></span>
                {#if vadState.isSpeaking}
                    <span class="absolute inset-0 w-3 h-3 rounded-full bg-green-500 animate-ping opacity-50"></span>
                {/if}
            </div>
            <span class="text-sm {getStatusColor()} font-medium">{getStatusText()}</span>
        </div>
        <div class="flex items-center gap-4 text-xs text-slate-500">
            <span class="flex items-center gap-1">
                <span class="w-2 h-2 rounded-full bg-green-500"></span>
                Speech: {formatTime(vadState.totalSpeechTime)}
            </span>
            <span>Chunks: {vadState.chunksSent}</span>
        </div>
    </div>

    <!-- Waveform Visualization -->
    <div class="flex items-end justify-center gap-px h-20 bg-dark-900/60 rounded-lg p-3 mb-3">
        {#each barHistory as bar, i}
            {@const height = Math.max(4, Math.min(64, bar.level * 600))}
            <div 
                class="flex-1 min-w-[3px] max-w-[6px] rounded-t-sm transition-all duration-50 {getBarColor(bar.isSpeech, bar.level)}"
                style="height: {height}px; opacity: {0.5 + (i / BAR_COUNT) * 0.5}"
            ></div>
        {/each}
        {#if barHistory.length < BAR_COUNT}
            {#each Array(BAR_COUNT - barHistory.length) as _, i}
                <div class="flex-1 min-w-[3px] max-w-[6px] rounded-t-sm bg-slate-800 h-1"></div>
            {/each}
        {/if}
    </div>

    <!-- Buffer Progress Bar -->
    <div class="space-y-2">
        <div class="flex items-center justify-between">
            <span class="text-xs text-slate-500">Speech Buffer</span>
            <span class="text-xs {getMinBufferPercent() >= 100 ? 'text-green-400' : 'text-cyan-400'}">
                {formatTime(vadState.bufferDuration)} / {formatTime(vadManager.getConfig().minSpeechDuration)}
            </span>
        </div>
        <div class="h-2 bg-dark-900/80 rounded-full overflow-hidden">
            <div 
                class="h-full transition-all duration-200 rounded-full {getMinBufferPercent() >= 100 ? 'bg-green-500' : 'bg-cyan-500'}"
                style="width: {getMinBufferPercent()}%"
            ></div>
        </div>
    </div>

    <!-- Silence/Activity Indicators -->
    <div class="mt-3 flex items-center justify-between">
        <div class="flex items-center gap-2">
            <span class="text-xs text-slate-500">Activity:</span>
            <div class="flex gap-1">
                {#each [0, 1, 2, 3, 4, 5, 6, 7] as dot}
                    {@const isActive = vadState.vadConfidence > dot * 0.12}
                    <div 
                        class="w-2 h-4 rounded-sm transition-all duration-100 {
                            isActive 
                                ? (dot > 5 ? 'bg-green-500' : dot > 2 ? 'bg-yellow-500' : 'bg-slate-500')
                                : 'bg-slate-700'
                        }"
                    ></div>
                {/each}
            </div>
        </div>
        
        <!-- Confidence percentage -->
        <span class="text-xs text-slate-400">
            {(vadState.vadConfidence * 100).toFixed(0)}% confidence
        </span>
    </div>
</div>

<style>
    .vad-waveform {
        box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
    }
</style>
