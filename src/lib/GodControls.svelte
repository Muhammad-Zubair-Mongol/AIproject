<script lang="ts">
    import { createEventDispatcher } from "svelte";

    export let onSettingsChange: (settings: any) => void;

    let confidenceThreshold = 0.7;
    let vadSensitivity = 0.5;
    let autoConnect = false;

    // Categories
    let categories = [
        { id: "task", label: "Tasks", checked: true },
        { id: "decision", label: "Decisions", checked: true },
        { id: "idea", label: "Ideas", checked: true },
        { id: "risk", label: "Risks", checked: false },
    ];

    const dispatch = createEventDispatcher();

    function updateSettings() {
        const settings = {
            confidenceThreshold,
            vadSensitivity,
            autoConnect,
            categories: categories.filter((c) => c.checked).map((c) => c.id),
        };
        dispatch("settingsChange", settings);
        if (onSettingsChange) onSettingsChange(settings);
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
                on:change={updateSettings}
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
                on:change={updateSettings}
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
            on:change={updateSettings}
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
                        on:change={updateSettings}
                        class="h-4 w-4 rounded border-slate-300 text-slate-900 focus:ring-slate-900"
                    />
                    <label for={cat.id} class="text-sm text-slate-700"
                        >{cat.label}</label
                    >
                </div>
            {/each}
        </div>
    </div>
</div>
