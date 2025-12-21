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

<div class="card space-y-6">
    <!-- Confidence -->
    <div>
        <label for="conf" class="block text-sm font-medium text-slate-700 mb-1">
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
                class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-slate-900"
            />
            <span class="text-sm font-mono text-slate-600 w-12 text-right">
                {(confidenceThreshold * 100).toFixed(0)}%
            </span>
        </div>
    </div>

    <!-- VAD -->
    <div>
        <label for="vad" class="block text-sm font-medium text-slate-700 mb-1">
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
                class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-slate-900"
            />
            <span class="text-sm font-mono text-slate-600 w-12 text-right">
                {vadSensitivity}
            </span>
        </div>
    </div>

    <!-- Toggles -->
    <div class="flex items-center gap-2">
        <input
            id="autoconnect"
            type="checkbox"
            bind:checked={autoConnect}
            onchange={updateSettings}
            class="h-4 w-4 rounded border-slate-300 text-slate-900 focus:ring-slate-900"
        />
        <label for="autoconnect" class="text-sm text-slate-700"
            >Auto-connect on startup</label
        >
    </div>

    <!-- Filters -->
    <div class="pt-4 border-t border-slate-100">
        <h4
            class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3"
        >
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
                        class="h-4 w-4 rounded border-slate-300 text-slate-900 focus:ring-slate-900"
                    />
                    <label for={cat.id} class="text-sm text-slate-700"
                        >{cat.label}</label
                    >
                </div>
            {/each}
        </div>
    </div>
    
    <!-- Prediction Controls -->
    <div class="pt-4 border-t border-slate-100">
        <h4 class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">
            Prediction & Optimistic Mode
        </h4>
        
        <div class="space-y-3">
            <div>
                <label for="pred" class="block text-sm font-medium text-slate-700 mb-1">
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
                        class="w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-slate-900"
                    />
                    <span class="text-sm font-mono text-slate-600 w-12 text-right">
                        {(predictionAggression * 100).toFixed(0)}%
                    </span>
                </div>
            </div>
            
            <div class="flex items-center gap-2">
                <input
                    id="optimistic"
                    type="checkbox"
                    bind:checked={enableOptimistic}
                    onchange={updateSettings}
                    class="h-4 w-4 rounded border-slate-300 text-slate-900 focus:ring-slate-900"
                />
                <label for="optimistic" class="text-sm text-slate-700">
                    Enable Optimistic Predictions (Local)
                </label>
            </div>
        </div>
    </div>
    
    <!-- Manual Intelligence Injection -->
    <div class="pt-4 border-t border-slate-100">
        <h4 class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">
            Manual Intelligence Inject
        </h4>
        
        <div class="space-y-2">
            <input
                type="text"
                placeholder="Enter text to inject..."
                bind:value={manualText}
                class="w-full px-3 py-2 text-sm border border-slate-200 rounded-md focus:border-slate-400 outline-none"
            />
            <div class="flex gap-2">
                <select
                    bind:value={manualCategory}
                    class="flex-1 px-3 py-2 text-sm border border-slate-200 rounded-md bg-white"
                >
                    <option value="TASK">Task</option>
                    <option value="DECISION">Decision</option>
                    <option value="RISK">Risk</option>
                    <option value="ACTION_ITEM">Action Item</option>
                </select>
                <button
                    class="px-4 py-2 text-sm bg-slate-900 text-white rounded-md hover:bg-slate-800 transition-colors"
                    onclick={injectIntelligence}
                >
                    Inject
                </button>
            </div>
        </div>
    </div>
    
    <!-- Keyboard Shortcuts Reference -->
    <div class="pt-4 border-t border-slate-100">
        <h4 class="text-xs font-semibold text-slate-500 uppercase tracking-wider mb-3">
            ⌨️ Keyboard Shortcuts
        </h4>
        <div class="text-xs text-slate-600 space-y-1">
            <div><kbd class="px-1 bg-slate-100 rounded">Ctrl+Shift+R</kbd> Toggle Recording</div>
            <div><kbd class="px-1 bg-slate-100 rounded">Ctrl+Shift+G</kbd> Graph View</div>
            <div><kbd class="px-1 bg-slate-100 rounded">Ctrl+Shift+A</kbd> Alerts</div>
            <div><kbd class="px-1 bg-slate-100 rounded">Ctrl+Shift+T</kbd> Transcripts</div>
        </div>
    </div>
</div>
