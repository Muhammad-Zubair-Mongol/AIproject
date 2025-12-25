<script lang="ts">
    import { createEventDispatcher } from "svelte";

    export let isProcessing = false;
    export let currentStep = 0;
    export let error: string | null = null;

    const dispatch = createEventDispatcher();

    const steps = [
        { id: 1, label: "Saving recording...", icon: "💾", duration: "instant" },
        { id: 2, label: "Transcribing audio...", icon: "🎤", duration: "~5s" },
        { id: 3, label: "Vocal Topography Analysis...", icon: "🌊", duration: "~3s" },
        { id: 4, label: "Building Knowledge Graph...", icon: "🕸️", duration: "~3s" },
        { id: 5, label: "Psychosomatic Engine...", icon: "🧠", duration: "~2s",
          substeps: [
            "Analyzing emotional patterns",
            "Detecting stress markers", 
            "Measuring engagement levels",
            "Extracting cognitive insights"
          ]
        },
        { id: 6, label: "Gemini Conduit Processing...", icon: "🔮", duration: "~2s" },
        { id: 7, label: "Complete!", icon: "✅", duration: "" }
    ];

    let currentSubstep = 0;
    let progressPercent = 0;

    // Animate progress based on current step
    $: if (isProcessing) {
        const baseProgress = ((currentStep - 1) / (steps.length - 1)) * 100;
        progressPercent = Math.min(baseProgress, 100);
    }

    function retry() {
        dispatch('retry');
    }

    function dismiss() {
        dispatch('dismiss');
    }
</script>

{#if isProcessing || error}
    <div class="glass-card p-6 mb-6 animate-fadeIn">
        <div class="flex items-center justify-between mb-4">
            <h3 class="text-lg font-semibold text-slate-100 flex items-center gap-2">
                {#if error}
                    <span class="text-red-500">⚠️</span>
                    Processing Error
                {:else if currentStep >= 7}
                    <span class="text-green-500">✅</span>
                    Processing Complete
                {:else}
                    <span class="animate-spin">⚙️</span>
                    Processing Recording
                {/if}
            </h3>
            
            {#if currentStep >= 7 || error}
                <button 
                    class="text-slate-400 hover:text-white transition-colors"
                    onclick={dismiss}
                    aria-label="Dismiss"
                >
                    <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                    </svg>
                </button>
            {/if}
        </div>

        {#if error}
            <!-- Error State -->
            <div class="bg-red-500/10 border border-red-500/30 rounded-lg p-4 mb-4">
                <p class="text-red-400">{error}</p>
            </div>
            <div class="flex gap-3">
                <button class="btn-primary" onclick={retry}>
                    🔄 Retry
                </button>
                <button class="btn-secondary" onclick={dismiss}>
                    Dismiss
                </button>
            </div>
        {:else}
            <!-- Progress Bar -->
            <div class="mb-6">
                <div class="h-2 bg-dark-700 rounded-full overflow-hidden">
                    <div 
                        class="h-full bg-gradient-to-r from-cyan-500 to-blue-500 transition-all duration-500 ease-out"
                        style="width: {progressPercent}%"
                    ></div>
                </div>
            </div>

            <!-- Steps List -->
            <div class="space-y-3">
                {#each steps as step, i}
                    {@const isActive = currentStep === step.id}
                    {@const isComplete = currentStep > step.id}
                    {@const isPending = currentStep < step.id}
                    
                    <div class="flex items-start gap-4 {isPending ? 'opacity-40' : ''}">
                        <!-- Step Icon/Status -->
                        <div class="w-8 h-8 rounded-full flex items-center justify-center text-sm
                            {isComplete ? 'bg-green-500/20 text-green-400 border border-green-500/30' : 
                             isActive ? 'bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 animate-pulse' : 
                             'bg-dark-600 text-slate-500 border border-dark-500'}">
                            {#if isComplete}
                                ✓
                            {:else}
                                {step.icon}
                            {/if}
                        </div>

                        <!-- Step Content -->
                        <div class="flex-1">
                            <div class="flex items-center justify-between">
                                <span class="font-medium {isActive ? 'text-cyan-400' : isComplete ? 'text-green-400' : 'text-slate-400'}">
                                    {step.label}
                                </span>
                                {#if step.duration && !isComplete}
                                    <span class="text-xs text-slate-500">{step.duration}</span>
                                {/if}
                            </div>

                            <!-- Substeps for Intelligence Engine -->
                            {#if step.substeps && isActive}
                                <div class="mt-2 ml-2 space-y-1">
                                    {#each step.substeps as substep, j}
                                        <div class="flex items-center gap-2 text-sm">
                                            {#if j < currentSubstep}
                                                <span class="text-green-400">✓</span>
                                                <span class="text-slate-400 line-through">{substep}</span>
                                            {:else if j === currentSubstep}
                                                <span class="animate-spin text-cyan-400">◉</span>
                                                <span class="text-cyan-300">{substep}</span>
                                            {:else}
                                                <span class="text-slate-600">○</span>
                                                <span class="text-slate-500">{substep}</span>
                                            {/if}
                                        </div>
                                    {/each}
                                </div>
                            {/if}
                        </div>
                    </div>
                {/each}
            </div>

            <!-- Completion Message -->
            {#if currentStep >= 7}
                <div class="mt-6 p-4 bg-green-500/10 border border-green-500/30 rounded-lg text-center">
                    <p class="text-green-400 font-medium">
                        🎉 Your recording has been processed successfully!
                    </p>
                    <p class="text-slate-400 text-sm mt-1">
                        Scroll down to view the new transcript and analysis.
                    </p>
                </div>
            {/if}
        {/if}
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
</style>
