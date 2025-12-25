<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { invoke } from "@tauri-apps/api/core";

    export let onSettingsChange: (settings: any) => void;

    let confidenceThreshold = 0.7;
    let vadSensitivity = 0.5;
    let predictionAggression = 0.5;
    let autoConnect = false;
    let enableOptimistic = true;
    
    // Manual injection
    let manualText = "";
    let manualCategory = "TASK";

    // All 16 categories from GOD PROMPT V9
    let categories = [
        { id: "TASK", label: "Tasks", checked: true },
        { id: "DECISION", label: "Decisions", checked: true },
        { id: "DEADLINE", label: "Deadlines", checked: true },
        { id: "ACTION_ITEM", label: "Action Items", checked: true },
        { id: "RISK", label: "Risks", checked: true },
        { id: "URGENCY", label: "Urgency", checked: true },
        { id: "SENTIMENT", label: "Sentiment", checked: false },
        { id: "INTERRUPTION", label: "Interruptions", checked: false },
        { id: "AGREEMENT", label: "Agreement", checked: false },
        { id: "DISAGREEMENT", label: "Disagreement", checked: false },
        { id: "EMOTION_SHIFT", label: "Emotion Shifts", checked: false },
        { id: "TOPIC_DRIFT", label: "Topic Drifts", checked: false },
    ];

    const dispatch = createEventDispatcher();

    async function updateSettings() {
        const selectedCategories = categories.filter((c) => c.checked).map((c) => c.id);
        
        const settings = {
            confidenceThreshold,
            vadSensitivity,
            predictionAggression,
            autoConnect,
            enableOptimistic,
            categories: selectedCategories,
        };
        
        // Call backend
        try {
            await invoke("update_processing_settings", {
                confidenceThreshold,
                predictionAggression,
                enableOptimistic,
                categories: selectedCategories,
            });
        } catch (error) {
            console.error("Failed to update settings:", error);
        }
        
        dispatch("settingsChange", settings);
        if (onSettingsChange) onSettingsChange(settings);
    }
    
    async function injectIntelligence() {
        if (!manualText.trim()) return;
        
        try {
            const result = await invoke("inject_manual_intelligence", {
                text: manualText,
                category: manualCategory,
                confidence: 1.0,
            });
            console.log("Injected intelligence:", result);
            manualText = "";
            dispatch("intelligenceInjected", result);
        } catch (error) {
            console.error("Failed to inject intelligence:", error);
        }
    }
</script>

<div class="space-y-6">
    <!-- Confidence -->
    <div>
        <label for="conf" class="block text-xs text-slate-400 mb-2">
            Min Confidence Threshold
        </label>
        <div class="flex items-center gap-4">
            <input
                id="conf"
                type="range"
                min="0"
                max="1"
                step="0.05"
                bind:value={confidenceThreshold}
                onchange={updateSettings}
                class="w-full"
            />
            <span class="text-sm font-mono text-cyan-400 w-12 text-right">
                {(confidenceThreshold * 100).toFixed(0)}%
            </span>
        </div>
    </div>

    <!-- VAD -->
    <div>
        <label for="vad" class="block text-xs text-slate-400 mb-2">
            Voice Activity Sensitivity
        </label>
        <div class="flex items-center gap-4">
            <input
                id="vad"
                type="range"
                min="0"
                max="1"
                step="0.1"
                bind:value={vadSensitivity}
                onchange={updateSettings}
                class="w-full"
            />
            <span class="text-sm font-mono text-cyan-400 w-12 text-right">
                {vadSensitivity}
            </span>
        </div>
    </div>

    <!-- Toggles -->
    <div class="flex items-center gap-3">
        <input
            id="autoconnect"
            type="checkbox"
            bind:checked={autoConnect}
            onchange={updateSettings}
            class="w-4 h-4 rounded"
        />
        <label for="autoconnect" class="text-sm text-slate-300">
            Auto-connect on startup
        </label>
    </div>

    <!-- Filters -->
    <div class="pt-4 border-t border-cyan-500/10">
        <h4 class="text-xs font-semibold text-cyan-400 uppercase tracking-wider mb-3">
            Intelligence Filters
        </h4>
        <div class="grid grid-cols-2 gap-2">
            {#each categories as cat}
                <div class="flex items-center gap-2">
                    <input
                        id={cat.id}
                        type="checkbox"
                        bind:checked={cat.checked}
                        onchange={updateSettings}
                        class="w-4 h-4 rounded"
                    />
                    <label for={cat.id} class="text-sm text-slate-300">
                        {cat.label}
                    </label>
                </div>
            {/each}
        </div>
    </div>
    
    <!-- Prediction Controls -->
    <div class="pt-4 border-t border-cyan-500/10">
        <h4 class="text-xs font-semibold text-cyan-400 uppercase tracking-wider mb-3">
            Prediction & Optimistic Mode
        </h4>
        
        <div class="space-y-4">
            <div>
                <label for="pred" class="block text-xs text-slate-400 mb-2">
                    Prediction Aggression
                </label>
                <div class="flex items-center gap-4">
                    <input
                        id="pred"
                        type="range"
                        min="0"
                        max="1"
                        step="0.1"
                        bind:value={predictionAggression}
                        onchange={updateSettings}
                        class="w-full"
                    />
                    <span class="text-sm font-mono text-cyan-400 w-12 text-right">
                        {(predictionAggression * 100).toFixed(0)}%
                    </span>
                </div>
            </div>
            
            <div class="flex items-center gap-3">
                <input
                    id="optimistic"
                    type="checkbox"
                    bind:checked={enableOptimistic}
                    onchange={updateSettings}
                    class="w-4 h-4 rounded"
                />
                <label for="optimistic" class="text-sm text-slate-300">
                    Enable Optimistic Predictions (Local)
                </label>
            </div>
        </div>
    </div>
    
    <!-- Manual Intelligence Injection -->
    <div class="pt-4 border-t border-cyan-500/10">
        <h4 class="text-xs font-semibold text-cyan-400 uppercase tracking-wider mb-3">
            Manual Intelligence Inject
        </h4>
        
        <div class="space-y-3">
            <input
                type="text"
                placeholder="Enter text to inject..."
                bind:value={manualText}
                class="input-field"
            />
            <div class="flex gap-2">
                <select
                    bind:value={manualCategory}
                    class="select-field flex-1"
                >
                    <option value="TASK">Task</option>
                    <option value="DECISION">Decision</option>
                    <option value="RISK">Risk</option>
                    <option value="ACTION_ITEM">Action Item</option>
                </select>
                <button
                    class="btn-primary"
                    onclick={injectIntelligence}
                >
                    Inject
                </button>
            </div>
        </div>
    </div>
    
    <!-- Keyboard Shortcuts Reference -->
    <div class="pt-4 border-t border-cyan-500/10">
        <h4 class="text-xs font-semibold text-cyan-400 uppercase tracking-wider mb-3 flex items-center gap-2">
            <svg class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="4" width="20" height="16" rx="2" ry="2"/><path d="M6 8h.001"/><path d="M10 8h.001"/><path d="M14 8h.001"/><path d="M18 8h.001"/><path d="M8 12h.001"/><path d="M12 12h.001"/><path d="M16 12h.001"/><path d="M7 16h10"/></svg> Keyboard Shortcuts
        </h4>
        <div class="text-xs text-slate-400 space-y-1.5">
            <div class="flex items-center gap-2">
                <kbd class="px-1.5 py-0.5 bg-dark-700 rounded text-cyan-400 border border-cyan-500/20">Ctrl+Shift+R</kbd>
                <span>Toggle Recording</span>
            </div>
            <div class="flex items-center gap-2">
                <kbd class="px-1.5 py-0.5 bg-dark-700 rounded text-cyan-400 border border-cyan-500/20">Ctrl+Shift+G</kbd>
                <span>Graph View</span>
            </div>
            <div class="flex items-center gap-2">
                <kbd class="px-1.5 py-0.5 bg-dark-700 rounded text-cyan-400 border border-cyan-500/20">Ctrl+Shift+A</kbd>
                <span>Alerts</span>
            </div>
            <div class="flex items-center gap-2">
                <kbd class="px-1.5 py-0.5 bg-dark-700 rounded text-cyan-400 border border-cyan-500/20">Ctrl+Shift+T</kbd>
                <span>Transcripts</span>
            </div>
        </div>
    </div>
</div>
