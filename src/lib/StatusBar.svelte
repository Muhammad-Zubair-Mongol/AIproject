<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { keyManager } from "./keyManager";

    export let isGeminiConnected = false;
    export let isRecording = false;
    export let apiKeyCount = 0;
    export let activeKeyName = "";
    export let activeKeyIndex = 1;
    export let isRateLimited = false;
    export let lastRequestTime: string | null = null;
    export let debugMode = false;
    export let requestCount = 0;

    const dispatch = createEventDispatcher();

    function getStatusText(): string {
        if (apiKeyCount === 0) {
            return "No API key – click to setup";
        }
        if (isRateLimited) {
            return `Rate limited – switching key...`;
        }
        if (isGeminiConnected) {
            if (apiKeyCount > 1) {
                return `API: Key ${activeKeyIndex}/${apiKeyCount} – Connected`;
            }
            return `Connected to Gemini`;
        }
        // Keys exist but not connected yet
        return `${apiKeyCount} key${apiKeyCount > 1 ? 's' : ''} ready – connecting...`;
    }

    function getStatusColor(): string {
        if (apiKeyCount === 0) return "bg-red-500";
        if (isRateLimited) return "bg-yellow-500 animate-pulse";
        if (isGeminiConnected) return "bg-green-500";
        return "bg-cyan-500 animate-pulse"; // Keys exist, connecting
    }

    function handleClick() {
        if (apiKeyCount === 0 || !isGeminiConnected) {
            dispatch("openSettings");
        }
    }
</script>

<div class="fixed bottom-0 left-0 right-0 z-40 h-8 bg-dark-900/95 border-t border-cyan-500/10 backdrop-blur-sm flex items-center justify-between px-4">
    <!-- Left: Connection Status -->
    <button 
        class="flex items-center gap-2 hover:bg-cyan-500/10 px-2 py-1 rounded transition-colors"
        onclick={handleClick}
    >
        <span class="w-2 h-2 rounded-full {getStatusColor()}"></span>
        <span class="text-xs {isGeminiConnected ? 'text-green-400' : apiKeyCount === 0 ? 'text-red-400' : 'text-slate-400'}">
            {getStatusText()}
        </span>
    </button>

    <!-- Center: Recording indicator -->
    {#if isRecording}
        <div class="flex items-center gap-2">
            <span class="w-2 h-2 rounded-full bg-red-500 animate-pulse"></span>
            <span class="text-xs text-red-400">Recording Active</span>
        </div>
    {/if}

    <!-- Right: Stats & Debug -->
    <div class="flex items-center gap-4 text-xs text-slate-500">
        {#if debugMode && lastRequestTime}
            <span>Last request: {lastRequestTime}</span>
        {/if}
        {#if requestCount > 0}
            <span>API calls: {requestCount}</span>
        {/if}
        {#if apiKeyCount > 1}
            <span title="Multiple keys for failover" class="flex items-center gap-1"><svg class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/></svg> {apiKeyCount} keys</span>
        {/if}
        <span class="text-slate-600">Meeting Mind v1.0</span>
    </div>
</div>
