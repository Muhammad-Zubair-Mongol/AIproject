<script lang="ts">
    import { invoke } from "@tauri-apps/api/core";
    import { save } from "@tauri-apps/plugin-dialog";

    export let currentSession: any = null;
    export let onSessionLoad: (session: any) => void = () => {};

    let sessions: any[] = [];
    let showSaveDialog = false;
    let showLoadDialog = false;
    let showExportDialog = false;
    let showSummaryDialog = false;
    let sessionTitle = "Untitled Meeting";
    let exportFormat: "json" | "csv" | "markdown" | "graphml" | "entities" = "json";
    let isSaving = false;
    let isGeneratingSummary = false;
    let sessionSummary: any = null;

    async function loadSessions() {
        try {
            const result = await invoke("list_sessions");
            sessions = JSON.parse(result as string);
        } catch (error) {
            console.error("Failed to load sessions:", error);
        }
    }

    async function saveCurrentSession() {
        if (!currentSession) return;

        isSaving = true;
        try {
            currentSession.metadata.title = sessionTitle;
            const sessionJson = JSON.stringify(currentSession);
            const filepath = await invoke("save_session", { sessionJson });
            console.log("Session saved:", filepath);
            showSaveDialog = false;
            await loadSessions();
        } catch (error) {
            console.error("Failed to save session:", error);
        } finally {
            isSaving = false;
        }
    }

    async function loadSession(sessionId: string) {
        try {
            const result = await invoke("load_session", { sessionId });
            const session = JSON.parse(result as string);
            onSessionLoad(session);
            showLoadDialog = false;
        } catch (error) {
            console.error("Failed to load session:", error);
        }
    }

    async function deleteSession(sessionId: string) {
        if (!confirm("Are you sure you want to delete this session?")) return;

        try {
            await invoke("delete_session", { sessionId });
            await loadSessions();
        } catch (error) {
            console.error("Failed to delete session:", error);
        }
    }

    async function exportSession() {
        if (!currentSession) return;

        try {
            const sessionJson = JSON.stringify(currentSession);
            const content = (await invoke("export_session", {
                sessionJson,
                format: exportFormat,
            })) as string;

            const extensions: Record<string, string[]> = {
                json: ["json"],
                csv: ["csv"],
                markdown: ["md"],
                graphml: ["graphml"],
                entities: ["json"],
            };

            const filePath = await save({
                defaultPath: `session_${currentSession.id}.${exportFormat === "markdown" ? "md" : exportFormat}`,
                filters: [
                    {
                        name: exportFormat.toUpperCase(),
                        extensions: extensions[exportFormat],
                    },
                ],
            });

            if (filePath) {
                const { writeTextFile } = await import("@tauri-apps/plugin-fs");
                await writeTextFile(filePath, content);
                console.log("Exported to:", filePath);
                showExportDialog = false;
                alert(`Successfully exported to ${filePath}`);
            }
        } catch (error) {
            console.error("Failed to export session:", error);
            alert(`Export failed: ${error}`);
        }
    }
    
    async function generateSummary() {
        if (!currentSession) return;
        
        isGeneratingSummary = true;
        try {
            const sessionJson = JSON.stringify(currentSession);
            const result = await invoke("generate_session_summary", { sessionJson });
            const updatedSession = JSON.parse(result as string);
            currentSession = updatedSession;
            sessionSummary = updatedSession.summary;
            showSummaryDialog = true;
        } catch (error) {
            console.error("Failed to generate summary:", error);
            alert(`Summary generation failed: ${error}`);
        } finally {
            isGeneratingSummary = false;
        }
    }

    $: if (showLoadDialog) loadSessions();
</script>

<div class="grid grid-cols-4 gap-2">
    <button
        class="flex flex-col items-center gap-1 p-2 rounded-lg transition-all duration-300 hover:bg-cyan-500/10 text-cyan-400 hover:text-cyan-300"
        onclick={() => (showSaveDialog = true)}
    >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"></path>
            <polyline points="17 21 17 13 7 13 7 21"></polyline>
            <polyline points="7 3 7 8 15 8"></polyline>
        </svg>
        <span class="text-xs">Save</span>
    </button>
    <button
        class="flex flex-col items-center gap-1 p-2 rounded-lg transition-all duration-300 hover:bg-cyan-500/10 text-cyan-400 hover:text-cyan-300"
        onclick={() => (showLoadDialog = true)}
    >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"></path>
        </svg>
        <span class="text-xs">Load</span>
    </button>
    <button
        class="flex flex-col items-center gap-1 p-2 rounded-lg transition-all duration-300 hover:bg-cyan-500/10 text-cyan-400 hover:text-cyan-300"
        onclick={() => (showExportDialog = true)}
    >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="17 8 12 3 7 8"></polyline>
            <line x1="12" y1="3" x2="12" y2="15"></line>
        </svg>
        <span class="text-xs">Export</span>
    </button>
    <button
        class="flex flex-col items-center gap-1 p-2 rounded-lg transition-all duration-300 hover:bg-cyan-500/10 text-cyan-400 hover:text-cyan-300"
        onclick={generateSummary}
        disabled={isGeneratingSummary}
    >
        <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="18" y1="20" x2="18" y2="10"></line>
            <line x1="12" y1="20" x2="12" y2="4"></line>
            <line x1="6" y1="20" x2="6" y2="14"></line>
        </svg>
        <span class="text-xs">{isGeneratingSummary ? '...' : 'Summary'}</span>
    </button>
</div>

<!-- Save Dialog -->
{#if showSaveDialog}
    <div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
        <div class="glass-card p-6 max-w-md w-full mx-4">
            <h3 class="text-lg font-bold text-slate-100 mb-4">
                Save Session
            </h3>

            <label for="session-title" class="text-xs text-slate-400 block mb-2">
                Session Title
            </label>
            <input
                id="session-title"
                type="text"
                bind:value={sessionTitle}
                placeholder="Enter session title..."
                class="input-field mb-4"
            />

            <div class="flex gap-2">
                <button
                    class="btn-primary flex-1"
                    onclick={saveCurrentSession}
                    disabled={isSaving}
                >
                    {isSaving ? "Saving..." : "Save"}
                </button>
                <button
                    class="btn-secondary flex-1"
                    onclick={() => (showSaveDialog = false)}
                >
                    Cancel
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- Load Dialog -->
{#if showLoadDialog}
    <div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
        <div class="glass-card p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto">
            <h3 class="text-lg font-bold text-slate-100 mb-4">
                Load Session
            </h3>

            {#if sessions.length === 0}
                <p class="text-sm text-slate-500 text-center py-8">
                    No saved sessions found
                </p>
            {:else}
                <div class="space-y-2">
                    {#each sessions as session}
                        <div class="glass-card p-4 hover:border-cyan-500/30 transition-all cursor-pointer">
                            <div class="flex items-start justify-between mb-2">
                                <div>
                                    <h4 class="text-sm font-bold text-slate-200">
                                        {session.metadata.title}
                                    </h4>
                                    <p class="text-xs text-slate-500">
                                        {new Date(session.created_at).toLocaleString()}
                                    </p>
                                </div>
                                <div class="flex gap-2">
                                    <button
                                        class="btn-primary text-xs px-3 py-1"
                                        onclick={() => loadSession(session.id)}
                                    >
                                        Load
                                    </button>
                                    <button
                                        class="btn-ghost text-xs px-3 py-1 text-red-400 hover:text-red-300"
                                        onclick={() => deleteSession(session.id)}
                                    >
                                        Delete
                                    </button>
                                </div>
                            </div>
                            <div class="text-xs text-slate-500 flex gap-4">
                                <span>📝 {session.metadata.total_transcripts} transcripts</span>
                                <span>🕸️ {session.graph_nodes?.length || 0} nodes</span>
                                <span>⏱️ {session.metadata.duration_seconds}s</span>
                            </div>
                        </div>
                    {/each}
                </div>
            {/if}

            <button
                class="btn-secondary w-full mt-4"
                onclick={() => (showLoadDialog = false)}
            >
                Close
            </button>
        </div>
    </div>
{/if}

<!-- Export Dialog -->
{#if showExportDialog}
    <div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
        <div class="glass-card p-6 max-w-md w-full mx-4">
            <h3 class="text-lg font-bold text-slate-100 mb-4">
                Export Session
            </h3>

            <div class="text-xs text-slate-400 block mb-2">
                Export Format
            </div>
            <div class="grid grid-cols-3 gap-2 mb-4">
                {#each ['json', 'csv', 'markdown', 'graphml', 'entities'] as format}
                    <button
                        class="px-3 py-2 text-xs rounded-lg border transition-all {exportFormat === format
                            ? 'bg-cyan-500/20 border-cyan-500/50 text-cyan-300'
                            : 'bg-dark-700 border-cyan-500/20 text-slate-400 hover:border-cyan-500/40'}"
                        onclick={() => (exportFormat = format as any)}
                    >
                        {format.toUpperCase()}
                    </button>
                {/each}
            </div>

            <div class="flex gap-2">
                <button class="btn-primary flex-1" onclick={exportSession}>
                    Export
                </button>
                <button
                    class="btn-secondary flex-1"
                    onclick={() => (showExportDialog = false)}
                >
                    Cancel
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- Summary Dialog -->
{#if showSummaryDialog}
    <div class="fixed inset-0 bg-black/80 backdrop-blur-sm flex items-center justify-center z-50">
        <div class="glass-card p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto">
            <h3 class="text-lg font-bold text-slate-100 mb-4">
                Session Summary
            </h3>

            {#if sessionSummary}
                <div class="space-y-4">
                    <div class="glass-card p-4">
                        <h4 class="text-sm font-medium text-cyan-400 mb-2">Overview</h4>
                        <p class="text-sm text-slate-300">{sessionSummary.overview || 'No overview available'}</p>
                    </div>
                    
                    {#if sessionSummary.key_points?.length > 0}
                        <div class="glass-card p-4">
                            <h4 class="text-sm font-medium text-cyan-400 mb-2">Key Points</h4>
                            <ul class="text-sm text-slate-300 space-y-1">
                                {#each sessionSummary.key_points as point}
                                    <li class="flex gap-2">
                                        <span class="text-cyan-400">•</span>
                                        {point}
                                    </li>
                                {/each}
                            </ul>
                        </div>
                    {/if}
                    
                    {#if sessionSummary.action_items?.length > 0}
                        <div class="glass-card p-4">
                            <h4 class="text-sm font-medium text-cyan-400 mb-2">Action Items</h4>
                            <ul class="text-sm text-slate-300 space-y-1">
                                {#each sessionSummary.action_items as item}
                                    <li class="flex gap-2">
                                        <span class="text-green-400">→</span>
                                        {item}
                                    </li>
                                {/each}
                            </ul>
                        </div>
                    {/if}
                </div>
            {:else}
                <p class="text-sm text-slate-500 text-center py-8">
                    No summary available
                </p>
            {/if}

            <button
                class="btn-secondary w-full mt-4"
                onclick={() => (showSummaryDialog = false)}
            >
                Close
            </button>
        </div>
    </div>
{/if}
