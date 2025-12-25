<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { vadManager, type VADState } from "./vadManager";
    import Icon from "./Icon.svelte";

    export let isRecording = false;
    export let currentVolume = 0;
    export let isGeminiConnected = false;
    export let transcripts: Array<{ id: string; text: string; speaker: string; timestamp: string; tone?: string; isPartial?: boolean; }> = [];
    export let graphNodes: Array<{ id: string; type: string; label?: string; weight?: number; }> = [];
    export let graphEdges: Array<{ from: string; to: string; relation: string; }> = [];

    // Psychosomatic gauges (Exported to sync with session)
    export let stressLevel = 0;
    export let engagementLevel = 0.3;
    export let urgencyLevel = 0;
    export let clarityLevel = 0.4;

    // VAD State
    let vadState: VADState = vadManager.getState();
    let unsubscribe: (() => void) | null = null;

    // Waveform bars
    const BAR_COUNT = 50;
    let barHistory: { level: number; isSpeech: boolean }[] = [];
    const SPEECH_THRESHOLD = 0.015;

    // Animation
    let lastUpdateTime = 0;

    // Recent transcript for live display
    $: liveTranscript = transcripts.length > 0 ? transcripts[transcripts.length - 1] : null;

    // Update waveform with volume
    $: if (isRecording) {
        updateWaveform(currentVolume);
    }

    function updateWaveform(vol: number) {
        const now = Date.now();
        if (now - lastUpdateTime < 50) return;
        lastUpdateTime = now;

        // Determine if this is speech
        const isSpeech = vol > SPEECH_THRESHOLD;
        
        // Add to history with slight jitter for natural look
        const jitteredLevel = vol * (0.85 + Math.random() * 0.3);
        barHistory = [...barHistory.slice(-BAR_COUNT + 1), {
            level: jitteredLevel,
            isSpeech
        }];

        // Decay levels slowly
        urgencyLevel = Math.max(0, urgencyLevel - 0.005);
        stressLevel = Math.max(0, stressLevel - 0.005);
        engagementLevel = Math.max(0.2, engagementLevel - 0.002);
        clarityLevel = Math.max(0.3, clarityLevel - 0.002);
    }

    // Track emotions from transcript tones
    $: if (liveTranscript?.tone) {
        const t = liveTranscript.tone;
        if (t === 'URGENT') urgencyLevel = Math.min(1, urgencyLevel + 0.3);
        if (t === 'NEGATIVE' || t === 'FRUSTRATED') stressLevel = Math.min(1, stressLevel + 0.3);
        if (t === 'POSITIVE' || t === 'EXCITED') engagementLevel = Math.min(1, engagementLevel + 0.3);
        if (t === 'NEUTRAL') clarityLevel = Math.min(1, clarityLevel + 0.1);
    }

    function getBarColor(isSpeech: boolean, level: number): string {
        if (level < 0.003) return 'bg-slate-700';
        if (level < 0.01) return 'bg-slate-600';
        if (isSpeech) return 'bg-gradient-to-t from-green-600 to-green-400';
        return 'bg-gradient-to-t from-yellow-600 to-yellow-400';
    }

    function getGaugeColor(value: number): string {
        if (value > 0.7) return '#ef4444';
        if (value > 0.4) return '#eab308';
        return '#22c55e';
    }

    function formatTime(ms: number): string {
        const seconds = Math.floor(ms / 1000);
        const minutes = Math.floor(seconds / 60);
        const secs = seconds % 60;
        return `${minutes}:${secs.toString().padStart(2, '0')}`;
    }

    function getMinBufferPercent(): number {
        const minRequired = vadManager.getConfig().minSpeechDuration;
        return Math.min(100, (vadState.bufferDuration / minRequired) * 100);
    }

    onMount(() => {
        unsubscribe = vadManager.subscribe((state) => {
            vadState = state;
        });
    });

    onDestroy(() => {
        if (unsubscribe) unsubscribe();
    });
</script>

{#if isRecording}
    <div class="live-recording-panel animate-fadeIn">
        <!-- === COMPACT STATUS BAR (moved from VADWaveform - always at top) === -->
        <div class="status-bar glass-card p-3 mb-4 border-cyan-500/30">
            <div class="flex items-center justify-between gap-4">
                <!-- Left: Status + Waveform -->
                <div class="flex items-center gap-4 flex-1">
                    <!-- Status indicator -->
                    <div class="flex items-center gap-2 min-w-[140px]">
                        <div class="relative">
                            <span class="w-3 h-3 rounded-full block {vadState.isSpeaking ? 'bg-green-500' : 'bg-slate-600'}"></span>
                            {#if vadState.isSpeaking}
                                <span class="absolute inset-0 w-3 h-3 rounded-full bg-green-500 animate-ping opacity-50"></span>
                            {/if}
                        </div>
                        <span class="text-sm {vadState.isSpeaking ? 'text-green-400' : 'text-slate-400'} font-medium flex items-center gap-1.5">
                            <Icon name={vadState.isSpeaking ? 'microphone' : 'listening'} size={14} />
                            {vadState.isSpeaking ? 'Speaking' : 'Listening...'}
                        </span>
                    </div>

                    <!-- Mini Waveform -->
                    <div class="flex items-end gap-px h-8 flex-1 max-w-[300px]">
                        {#each barHistory.slice(-30) as bar, i}
                            {@const height = Math.max(3, Math.min(28, bar.level * 400))}
                            <div 
                                class="flex-1 min-w-[2px] max-w-[4px] rounded-t-sm transition-all duration-50 {getBarColor(bar.isSpeech, bar.level)}"
                                style="height: {height}px; opacity: {0.5 + (i / 30) * 0.5}"
                            ></div>
                        {/each}
                    </div>
                </div>

                <!-- Center: Buffer Progress -->
                <div class="flex items-center gap-3">
                    <span class="text-xs text-slate-500">Buffer</span>
                    <div class="w-24 h-2 bg-dark-900/80 rounded-full overflow-hidden">
                        <div 
                            class="h-full transition-all duration-200 rounded-full {getMinBufferPercent() >= 100 ? 'bg-green-500' : 'bg-cyan-500'}"
                            style="width: {getMinBufferPercent()}%"
                        ></div>
                    </div>
                    <span class="text-xs {getMinBufferPercent() >= 100 ? 'text-green-400' : 'text-cyan-400'} font-mono w-16">
                        {formatTime(vadState.bufferDuration)}
                    </span>
                </div>

                <!-- Right: Stats -->
                <div class="flex items-center gap-4 text-xs text-slate-500">
                    <span class="flex items-center gap-1">
                        <span class="w-2 h-2 rounded-full bg-green-500"></span>
                        {formatTime(vadState.totalSpeechTime)}
                    </span>
                    <span>Chunks: {vadState.chunksSent}</span>
                    <span class="text-slate-400">{(vadState.vadConfidence * 100).toFixed(0)}%</span>
                </div>
            </div>
        </div>

        <!-- === MAIN GRID: Large Transcript + Right Column === -->
        <div class="grid grid-cols-12 gap-4">
            <!-- LEFT: Large Live Transcript (8 columns) -->
            <div class="col-span-8">
                <div class="glass-card p-4 h-full border-cyan-500/20">
                    <div class="flex items-center justify-between mb-3">
                        <span class="text-xs text-slate-400 uppercase tracking-wider font-bold">Live Transcript</span>
                        {#if isGeminiConnected}
                            <span class="badge-success text-xs flex items-center gap-1"><Icon name="cpu" size={12} /> AI Active</span>
                        {/if}
                    </div>
                    
                    <!-- Scrolling Transcript Area -->
                    <div class="transcript-scroll space-y-3 max-h-[350px] overflow-y-auto pr-2">
                        {#if transcripts.length > 0}
                            {#each transcripts.slice(-10).reverse() as t (t.id)}
                                {@const isYou = t.speaker?.includes('1') || t.speaker === 'You'}
                                <div class="flex {isYou ? 'justify-end' : 'justify-start'} animate-fadeIn">
                                    <div class="max-w-[85%]">
                                        <!-- Speaker info -->
                                        <div class="flex items-center gap-2 mb-1 {isYou ? 'justify-end' : 'justify-start'}">
                                            <span class="text-xs {isYou ? 'text-cyan-400' : 'text-purple-400'} font-medium">
                                                {isYou ? 'You' : t.speaker || 'Speaker 2'}
                                            </span>
                                            <span class="text-xs text-slate-600">{t.timestamp}</span>
                                            {#if t.tone}
                                                <span class="px-1.5 py-0.5 text-[9px] rounded bg-cyan-500/20 text-cyan-400 uppercase">{t.tone}</span>
                                            {/if}
                                        </div>
                                        <!-- Message bubble -->
                                        <div class="p-3 rounded-2xl {isYou 
                                            ? 'bg-cyan-500/15 border border-cyan-500/30 rounded-tr-sm' 
                                            : 'bg-purple-500/10 border border-purple-500/20 rounded-tl-sm'}">
                                            <p class="text-sm text-slate-200 leading-relaxed">
                                                {t.text}
                                                {#if t.isPartial}
                                                    <span class="inline-flex gap-1 ml-1">
                                                        <span class="w-1 h-1 bg-cyan-500/50 rounded-full animate-bounce"></span>
                                                        <span class="w-1 h-1 bg-cyan-500/50 rounded-full animate-bounce" style="animation-delay: 0.1s"></span>
                                                        <span class="w-1 h-1 bg-cyan-500/50 rounded-full animate-bounce" style="animation-delay: 0.2s"></span>
                                                    </span>
                                                {/if}
                                            </p>
                                        </div>
                                    </div>
                                </div>
                            {/each}
                        {:else}
                            <!-- Minimal waiting state - NOT a huge placeholder -->
                            <div class="flex items-center justify-center py-8 text-slate-500">
                                <div class="flex items-center gap-3">
                                    <div class="w-2 h-2 rounded-full bg-cyan-500 animate-pulse"></div>
                                    <span class="text-sm">Waiting for speech...</span>
                                </div>
                            </div>
                        {/if}
                    </div>
                </div>
            </div>

            <!-- RIGHT COLUMN: Graph + Gauges + Status (4 columns) -->
            <div class="col-span-4 space-y-3">
                <!-- Knowledge Graph Mini -->
                <div class="glass-card p-3 border-cyan-500/20">
                    <div class="flex items-center justify-between mb-2">
                        <span class="text-xs text-slate-400 uppercase tracking-wider">Knowledge Graph</span>
                        <span class="text-[10px] text-cyan-400">{graphNodes.length} • {graphEdges.length}</span>
                    </div>
                    <div class="h-24 relative overflow-hidden bg-dark-900/40 rounded-lg">
                        {#if graphNodes.length > 0}
                            <svg class="w-full h-full">
                                <defs>
                                    <filter id="live-glow">
                                        <feGaussianBlur stdDeviation="2" result="coloredBlur"/>
                                        <feMerge>
                                            <feMergeNode in="coloredBlur"/>
                                            <feMergeNode in="SourceGraphic"/>
                                        </feMerge>
                                    </filter>
                                </defs>
                                {#each graphNodes.slice(-8) as node, i}
                                    {@const x = 20 + (i % 4) * 45}
                                    {@const y = 20 + Math.floor(i / 4) * 40}
                                    <g filter="url(#live-glow)">
                                        <circle cx={x} cy={y} r="10" fill="#00c8ff" opacity="0.8" class="animate-pulse"/>
                                        <text x={x} y={y + 3} text-anchor="middle" fill="white" font-size="7">
                                            {(node.label || node.id).slice(0, 3)}
                                        </text>
                                    </g>
                                {/each}
                            </svg>
                        {:else}
                            <div class="flex items-center justify-center h-full text-slate-600 text-xs">
                                Topics appear here
                            </div>
                        {/if}
                    </div>
                </div>

                <!-- Psychosomatic Gauges (Compact) -->
                <div class="glass-card p-3 border-cyan-500/20">
                    <span class="text-xs text-slate-400 uppercase tracking-wider block mb-2">Psychosomatic Engine</span>
                    <div class="space-y-2">
                        <!-- Stress -->
                        <div class="flex items-center gap-2">
                            <span class="text-[9px] text-slate-500 w-16 uppercase">Stress</span>
                            <div class="flex-1 h-1.5 bg-dark-600 rounded-full overflow-hidden">
                                <div class="h-full transition-all duration-500" style="width: {stressLevel * 100}%; background: {getGaugeColor(stressLevel)};"></div>
                            </div>
                            <span class="text-[9px] w-8 text-right" style="color: {getGaugeColor(stressLevel)}">{(stressLevel * 100).toFixed(0)}%</span>
                        </div>
                        <!-- Engagement -->
                        <div class="flex items-center gap-2">
                            <span class="text-[9px] text-slate-500 w-16 uppercase">Engage</span>
                            <div class="flex-1 h-1.5 bg-dark-600 rounded-full overflow-hidden">
                                <div class="h-full bg-cyan-500 transition-all duration-500" style="width: {engagementLevel * 100}%;"></div>
                            </div>
                            <span class="text-[9px] text-cyan-400 w-8 text-right">{(engagementLevel * 100).toFixed(0)}%</span>
                        </div>
                        <!-- Urgency -->
                        <div class="flex items-center gap-2">
                            <span class="text-[9px] text-slate-500 w-16 uppercase">Urgency</span>
                            <div class="flex-1 h-1.5 bg-dark-600 rounded-full overflow-hidden">
                                <div class="h-full transition-all duration-500" style="width: {urgencyLevel * 100}%; background: {getGaugeColor(urgencyLevel)};"></div>
                            </div>
                            <span class="text-[9px] w-8 text-right" style="color: {getGaugeColor(urgencyLevel)}">{(urgencyLevel * 100).toFixed(0)}%</span>
                        </div>
                        <!-- Clarity -->
                        <div class="flex items-center gap-2">
                            <span class="text-[9px] text-slate-500 w-16 uppercase">Clarity</span>
                            <div class="flex-1 h-1.5 bg-dark-600 rounded-full overflow-hidden">
                                <div class="h-full bg-green-500 transition-all duration-500" style="width: {clarityLevel * 100}%;"></div>
                            </div>
                            <span class="text-[9px] text-green-400 w-8 text-right">{(clarityLevel * 100).toFixed(0)}%</span>
                        </div>
                    </div>
                </div>

                <!-- Gemini Conduit Status (Very Compact) -->
                <div class="glass-card p-3 border-cyan-500/20">
                    <div class="flex items-center gap-3">
                        <div class="relative">
                            <div class="w-8 h-8 rounded-full bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center {isGeminiConnected ? 'animate-pulse' : ''}">
                                <Icon name="crystal" size={14} className="text-white" />
                            </div>
                            {#if isGeminiConnected}
                                <div class="absolute -top-0.5 -right-0.5 w-2.5 h-2.5 bg-green-500 rounded-full border border-dark-700"></div>
                            {/if}
                        </div>
                        <div class="flex-1">
                            <p class="text-xs text-slate-200 font-medium">
                                {isGeminiConnected ? 'Processing...' : 'Offline'}
                            </p>
                            <p class="text-[10px] text-slate-500">
                                {isGeminiConnected ? 'Real-time extraction active' : 'Connect in settings'}
                            </p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </div>
{/if}

<style>
    @keyframes fadeIn {
        from { opacity: 0; transform: translateY(-10px); }
        to { opacity: 1; transform: translateY(0); }
    }

    .animate-fadeIn {
        animation: fadeIn 0.3s ease-out forwards;
    }

    .transcript-scroll::-webkit-scrollbar {
        width: 4px;
    }

    .transcript-scroll::-webkit-scrollbar-track {
        background: rgba(0, 0, 0, 0.2);
        border-radius: 2px;
    }

    .transcript-scroll::-webkit-scrollbar-thumb {
        background: rgba(0, 200, 255, 0.3);
        border-radius: 2px;
    }

    .live-recording-panel {
        /* No extra padding needed - content fills space */
    }
</style>
