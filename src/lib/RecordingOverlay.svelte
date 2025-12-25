<script lang="ts">
    import { onMount, onDestroy, createEventDispatcher } from "svelte";
    import { vadManager } from "./vadManager";

    export let isRecording = false;
    export let currentVolume = 0;
    export let isGeminiConnected = false;

    const dispatch = createEventDispatcher();

    // Timer state
    let elapsedSeconds = 0;
    let timerInterval: ReturnType<typeof setInterval> | null = null;
    let startTime: number | null = null;

    // Audio visualization - more bars for smoother waveform
    const BAR_COUNT = 40;
    let volumeHistory: number[] = Array(BAR_COUNT).fill(0);
    
    // Silence detection - much smarter thresholds
    let consecutiveSilenceFrames = 0;
    let showSilenceWarning = false;
    let showVoiceDetected = false;
    let lastSpeechTime = 0;
    
    // Calibration
    let isCalibrating = true;
    let calibrationSeconds = 0;
    let baselineNoise = 0.001; // Very low default baseline
    let calibrationSamples: number[] = [];

    // Smart threshold (adjusts based on calibration) - MUCH more sensitive
    const SPEECH_THRESHOLD_MULTIPLIER = 1.5; // Speech must be 1.5x baseline (was 3)
    const SILENCE_WARNING_SECONDS = 30; // Show warning after 30s (was 15)
    const FRAMES_PER_SECOND = 10;
    const VOLUME_AMPLIFICATION = 50; // Amplify visual display

    // Format time as HH:MM:SS
    function formatTime(seconds: number): string {
        const hrs = Math.floor(seconds / 3600);
        const mins = Math.floor((seconds % 3600) / 60);
        const secs = seconds % 60;
        return `${hrs.toString().padStart(2, '0')}:${mins.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
    }

    // Determine if current volume is speech - VERY sensitive
    function isSpeech(vol: number): boolean {
        const threshold = Math.max(0.001, baselineNoise * SPEECH_THRESHOLD_MULTIPLIER);
        return vol > threshold;
    }

    // Get bar color based on volume level - more sensitive colors
    function getBarColor(vol: number, index: number): string {
        if (vol < 0.0005) return 'bg-slate-600'; // Dead silence
        if (vol < 0.002) return 'bg-slate-500'; // Very quiet
        if (vol > 0.01) return 'bg-green-400'; // Definite speech - green
        if (vol > 0.003) return 'bg-yellow-400'; // Some sound - yellow
        return 'bg-cyan-500'; // Low sound - cyan
    }

    // Update volume history for waveform
    $: if (isRecording) {
        // Add jitter for more natural feel
        const jitteredVol = currentVolume * (0.9 + Math.random() * 0.2);
        volumeHistory = [...volumeHistory.slice(1), jitteredVol];
        
        // Calibration phase (first 2 seconds)
        if (isCalibrating && calibrationSeconds < 2) {
            calibrationSamples.push(currentVolume);
        } else if (isCalibrating) {
            // End calibration, calculate baseline
            if (calibrationSamples.length > 0) {
                baselineNoise = calibrationSamples.reduce((a, b) => a + b, 0) / calibrationSamples.length;
                baselineNoise = Math.max(0.002, baselineNoise * 1.5); // Add margin
            }
            isCalibrating = false;
        }
        
        // Smart silence detection
        if (isSpeech(currentVolume)) {
            consecutiveSilenceFrames = 0;
            showSilenceWarning = false;
            showVoiceDetected = true;
            lastSpeechTime = Date.now();
        } else {
            consecutiveSilenceFrames++;
            // Only show warning after SILENCE_WARNING_SECONDS of continuous silence
            if (consecutiveSilenceFrames > SILENCE_WARNING_SECONDS * FRAMES_PER_SECOND) {
                showSilenceWarning = true;
            }
            // Hide voice detected badge after 2 seconds of silence
            if (Date.now() - lastSpeechTime > 2000) {
                showVoiceDetected = false;
            }
        }
    }

    // Start/stop timer based on recording state
    $: if (isRecording && !timerInterval) {
        startTime = Date.now();
        elapsedSeconds = 0;
        consecutiveSilenceFrames = 0;
        showSilenceWarning = false;
        showVoiceDetected = false;
        isCalibrating = true;
        calibrationSeconds = 0;
        calibrationSamples = [];
        volumeHistory = Array(BAR_COUNT).fill(0);
        
        timerInterval = setInterval(() => {
            if (startTime) {
                elapsedSeconds = Math.floor((Date.now() - startTime) / 1000);
                if (isCalibrating) calibrationSeconds++;
            }
        }, 1000);
    } else if (!isRecording && timerInterval) {
        clearInterval(timerInterval);
        timerInterval = null;
        startTime = null;
    }

    function openSettings() {
        dispatch('openSettings');
    }

    function dismissWarning() {
        showSilenceWarning = false;
        consecutiveSilenceFrames = 0;
    }

    onDestroy(() => {
        if (timerInterval) clearInterval(timerInterval);
    });
</script>

{#if isRecording}
    <div class="fixed top-0 left-0 right-0 z-50 pointer-events-none animate-slideDown">
        <!-- Main Recording Banner -->
        <div class="bg-gradient-to-r from-red-900/95 via-red-800/95 to-red-900/95 border-b border-red-500/50 backdrop-blur-sm shadow-lg shadow-red-500/20 pointer-events-auto">
            <div class="max-w-5xl mx-auto px-6 py-4">
                <div class="flex items-center justify-between gap-4">
                    <!-- Left: Recording Indicator + Voice Status -->
                    <div class="flex items-center gap-4">
                        <!-- Pulsing Record Icon -->
                        <div class="relative">
                            <div class="w-4 h-4 bg-red-500 rounded-full animate-pulse"></div>
                            <div class="absolute inset-0 w-4 h-4 bg-red-500 rounded-full animate-ping opacity-75"></div>
                        </div>
                        
                        <!-- Recording Label -->
                        <div>
                            <span class="text-white font-semibold text-lg">Recording</span>
                            {#if isCalibrating}
                                <span class="text-yellow-300 text-sm ml-2 animate-pulse">Calibrating...</span>
                            {:else if showVoiceDetected}
                                <span class="text-green-300 text-sm ml-2">Voice detected</span>
                            {:else}
                                <span class="text-red-200 text-sm ml-2">Listening...</span>
                            {/if}
                        </div>

                        <!-- Voice Detection Badge -->
                        {#if showVoiceDetected && !isCalibrating}
                            <div class="flex items-center gap-1.5 px-2 py-1 bg-green-500/20 rounded-full border border-green-500/30 animate-fadeIn">
                                <div class="w-2 h-2 bg-green-500 rounded-full animate-pulse"></div>
                                <span class="text-green-300 text-xs">Buffering</span>
                            </div>
                        {/if}
                    </div>

                    <!-- Center: Timer ONLY -->
                    <div class="flex-1 flex items-center justify-center">
                        <div class="text-white font-mono text-3xl font-bold tracking-wider tabular-nums">
                            {formatTime(elapsedSeconds)}
                        </div>
                    </div>

                    <!-- Right: Connection Status -->
                    <div class="flex items-center gap-3">
                        {#if isGeminiConnected}
                            <div class="flex items-center gap-2 px-3 py-1.5 bg-green-500/20 rounded-full border border-green-500/30">
                                <div class="w-2 h-2 bg-green-500 rounded-full"></div>
                                <span class="text-green-300 text-xs font-medium">AI Connected</span>
                            </div>
                        {:else}
                            <div class="flex items-center gap-2 px-3 py-1.5 bg-yellow-500/20 rounded-full border border-yellow-500/30">
                                <div class="w-2 h-2 bg-yellow-500 rounded-full animate-pulse"></div>
                                <span class="text-yellow-300 text-xs font-medium">Local Mode</span>
                            </div>
                        {/if}
                    </div>
                </div>

                <!-- Silence Warning - Only after 15 seconds -->
                {#if showSilenceWarning && !isCalibrating}
                    <div class="mt-3 flex items-center justify-center gap-3 text-yellow-300 bg-yellow-500/10 rounded-lg px-4 py-2 border border-yellow-500/30 animate-fadeIn">
                        <svg class="w-5 h-5 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                        </svg>
                        <span class="font-medium">No speech detected for {Math.floor(consecutiveSilenceFrames / FRAMES_PER_SECOND)}s – check microphone?</span>
                        <button 
                            class="text-yellow-200 hover:text-white underline text-sm"
                            onclick={openSettings}
                        >
                            Settings
                        </button>
                        <button 
                            class="text-yellow-400 hover:text-white text-sm px-2 py-0.5 rounded bg-yellow-500/20"
                            onclick={dismissWarning}
                        >
                            Dismiss
                        </button>
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
    @keyframes slideDown {
        from {
            transform: translateY(-100%);
            opacity: 0;
        }
        to {
            transform: translateY(0);
            opacity: 1;
        }
    }

    @keyframes fadeIn {
        from { opacity: 0; }
        to { opacity: 1; }
    }

    .animate-slideDown {
        animation: slideDown 0.3s ease-out forwards;
    }

    .animate-fadeIn {
        animation: fadeIn 0.3s ease-out forwards;
    }
</style>
