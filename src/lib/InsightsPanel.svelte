<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { intelligenceExtractor, type ExtractedInsights } from "./intelligenceExtractor";
    import Icon from "./Icon.svelte";

    let insights: ExtractedInsights = intelligenceExtractor.getInsights();
    let unsubscribe: (() => void) | null = null;
    let activeSection: string | null = null;

    onMount(() => {
        unsubscribe = intelligenceExtractor.subscribe((newInsights) => {
            insights = newInsights;
        });
    });

    onDestroy(() => {
        if (unsubscribe) unsubscribe();
    });

    function toggleSection(section: string) {
        activeSection = activeSection === section ? null : section;
    }

    function getTotalCount(): number {
        return (
            insights.tasks.length +
            insights.decisions.length +
            insights.actionItems.length +
            insights.deadlines.length +
            insights.risks.length +
            insights.urgency.length
        );
    }

    function getPriorityColor(priority?: string): string {
        switch (priority?.toLowerCase()) {
            case 'high': return 'text-red-400';
            case 'medium': return 'text-yellow-400';
            case 'low': return 'text-green-400';
            default: return 'text-slate-400';
        }
    }

    function getSeverityBg(severity?: string): string {
        switch (severity?.toLowerCase()) {
            case 'high': return 'bg-red-500/20 border-red-500/30';
            case 'medium': return 'bg-yellow-500/20 border-yellow-500/30';
            case 'low': return 'bg-green-500/20 border-green-500/30';
            case 'critical': return 'bg-red-600/30 border-red-500/50';
            default: return 'bg-slate-500/20 border-slate-500/30';
        }
    }
</script>

<div class="insights-panel space-y-3">
    <!-- Header with count -->
    <div class="flex items-center justify-between">
        <h3 class="text-sm font-semibold text-cyan-400 uppercase tracking-wider flex items-center gap-2">
            <Icon name="brain" size={14} /> Intelligence Insights
        </h3>
        {#if getTotalCount() > 0}
            <span class="text-xs px-2 py-0.5 rounded-full bg-cyan-500/20 text-cyan-400">
                {getTotalCount()} items
            </span>
        {/if}
    </div>

    {#if getTotalCount() === 0}
        <div class="text-center py-6 text-slate-500 text-sm">
            <p>No insights extracted yet</p>
            <p class="text-xs mt-1">Record a conversation to extract Tasks, Decisions, etc.</p>
        </div>
    {:else}
        <div class="space-y-4">
            <!-- Tasks -->
            {#if insights.tasks.length > 0}
                <div class="insight-section">
                    <div class="flex items-center justify-between mb-2">
                        <span class="flex items-center gap-2 text-xs font-bold text-blue-400 uppercase tracking-widest">
                            <Icon name="clipboard" size={12} /> Tasks
                        </span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded bg-blue-500/10 text-blue-400 border border-blue-500/20">{insights.tasks.length}</span>
                    </div>
                    <div class="space-y-2 animate-fadeIn">
                        {#each insights.tasks as task}
                            <div class="group flex items-start gap-3 p-3 rounded-xl bg-[#0d1117] border border-cyan-500/5 hover:border-blue-500/30 transition-all duration-300 shadow-sm">
                                <label class="mt-1 flex-shrink-0 cursor-pointer">
                                    <input type="checkbox" class="w-4 h-4 rounded border-cyan-500/30 bg-dark-800 text-blue-500 focus:ring-blue-500/50" />
                                </label>
                                <div class="flex-1">
                                    <p class="text-xs text-slate-200 leading-snug group-hover:text-blue-100 transition-colors">{task.text}</p>
                                    {#if task.assignee}
                                        <div class="flex items-center gap-1.5 mt-2">
                                            <div class="w-4 h-4 rounded-full bg-blue-500/20 flex items-center justify-center text-[8px] font-bold text-blue-400">
                                                {task.assignee[0]}
                                            </div>
                                            <span class="text-[10px] text-slate-500">Assignee: {task.assignee}</span>
                                        </div>
                                    {/if}
                                </div>
                                {#if task.priority}
                                    <span class="text-[9px] font-bold uppercase tracking-tighter {getPriorityColor(task.priority)}">{task.priority}</span>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}

            <!-- Decisions -->
            {#if insights.decisions.length > 0}
                <div class="insight-section">
                    <div class="flex items-center justify-between mb-2">
                        <span class="flex items-center gap-2 text-xs font-bold text-green-400 uppercase tracking-widest">
                            <Icon name="check" size={12} /> Decisions
                        </span>
                        <span class="text-[10px] px-1.5 py-0.5 rounded bg-green-500/10 text-green-400 border border-green-500/20">{insights.decisions.length}</span>
                    </div>
                    <div class="space-y-2 animate-fadeIn">
                        {#each insights.decisions as decision}
                            <div class="relative p-4 rounded-xl bg-gradient-to-br from-green-500/5 to-transparent border border-green-500/10 shadow-inner overflow-hidden">
                                <div class="absolute top-0 left-0 w-1 h-full bg-green-500/40"></div>
                                <p class="text-xs text-slate-300 italic leading-relaxed font-serif">
                                    "{decision.text}"
                                </p>
                                {#if decision.context}
                                    <p class="text-[10px] text-slate-500 mt-2 flex items-center gap-1">
                                        <span class="opacity-50">Context:</span> {decision.context}
                                    </p>
                                {/if}
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}

            <!-- Risks -->
            {#if insights.risks.length > 0}
                <div class="insight-section">
                    <div class="flex items-center justify-between mb-2">
                        <span class="flex items-center gap-2 text-xs font-bold text-red-400 uppercase tracking-widest">
                            <Icon name="warning" size={12} /> Critical Risks
                        </span>
                    </div>
                    <div class="space-y-2 animate-fadeIn">
                        {#each insights.risks as risk}
                            <div class="flex items-center gap-3 p-3 rounded-lg {getSeverityBg(risk.severity)} border animate-pulse-subtle">
                                <div class="w-8 h-8 rounded-full bg-red-500/20 flex items-center justify-center text-red-500">
                                    <svg xmlns="http://www.w3.org/2000/svg" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
                                </div>
                                <div class="flex-1">
                                    <p class="text-xs font-medium text-red-200">{risk.text}</p>
                                    <span class="text-[9px] uppercase tracking-widest text-red-400/70">{risk.severity || 'Critical'} Severity</span>
                                </div>
                            </div>
                        {/each}
                    </div>
                </div>
            {/if}

            <!-- Urgency -->
            {#if insights.urgency.length > 0}
                <div class="insight-section">
                     {#each insights.urgency as item}
                        <div class="flex items-center gap-2 p-2 rounded-full px-4 bg-purple-500/10 border border-purple-500/20 mb-2">
                            <span class="relative flex h-2 w-2">
                                <span class="animate-ping absolute inline-flex h-full w-full rounded-full bg-purple-400 opacity-75"></span>
                                <span class="relative inline-flex rounded-full h-2 w-2 bg-purple-500"></span>
                            </span>
                            <span class="text-[10px] font-bold text-purple-300 uppercase tracking-widest">Urgent:</span>
                            <span class="text-[10px] text-slate-300 truncate">{item.text}</span>
                        </div>
                    {/each}
                </div>
            {/if}
        </div>
    {/if}

    <!-- Key Topics -->
    {#if insights.keyTopics.length > 0}
        <div class="mt-4 pt-3 border-t border-cyan-500/10">
            <p class="text-xs text-slate-500 mb-2">Key Topics</p>
            <div class="flex flex-wrap gap-1">
                {#each insights.keyTopics as topic}
                    <span class="text-xs px-2 py-1 rounded bg-cyan-500/10 text-cyan-400">{topic}</span>
                {/each}
            </div>
        </div>
    {/if}
</div>

<style>
    .insight-section {
        transition: all 0.2s ease;
    }
</style>
