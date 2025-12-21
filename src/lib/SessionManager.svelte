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

            // Use Tauri dialog to save file
            const extensions = {
                json: ["json"],
                csv: ["csv"],
                markdown: ["md"],
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
                // Write content to file using Tauri fs plugin
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

<div class="space-y-2">
    <!-- Action Buttons -->
    <div class="flex gap-2 flex-wrap">
        <button
            class="god-button flex-1"
            onclick={() => (showSaveDialog = true)}
        >
            💾 Save
        </button>
        <button
            class="god-button flex-1"
            onclick={() => (showLoadDialog = true)}
        >
            📂 Load
        </button>
        <button
            class="god-button flex-1"
            onclick={() => (showExportDialog = true)}
        >
            📤 Export
        </button>
        <button
            class="god-button flex-1"
            onclick={generateSummary}
            disabled={isGeneratingSummary}
        >
            {isGeneratingSummary ? '⏳' : '📊'} Summary
        </button>
    </div>

    <!-- Save Dialog -->
    {#if showSaveDialog}
        <div
            class="fixed inset-0 bg-black/80 flex items-center justify-center z-50"
        >
            <div class="god-panel p-6 max-w-md w-full mx-4">
                <h3 class="text-lg font-bold text-green-300 mb-4">
                    Save Session
                </h3>

                <label
                    for="session-title"
                    class="text-xs text-green-400 block mb-2"
                    >Session Title</label
                >
                <input
                    id="session-title"
                    type="text"
                    bind:value={sessionTitle}
                    placeholder="Enter session title..."
                    class="god-input mb-4"
                />

                <div class="flex gap-2">
                    <button
                        class="god-button flex-1"
                        onclick={saveCurrentSession}
                        disabled={isSaving}
                    >
                        {isSaving ? "Saving..." : "Save"}
                    </button>
                    <button
                        class="god-button flex-1 opacity-50"
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
        <div
            class="fixed inset-0 bg-black/80 flex items-center justify-center z-50"
        >
            <div
                class="god-panel p-6 max-w-2xl w-full mx-4 max-h-[80vh] overflow-y-auto"
            >
                <h3 class="text-lg font-bold text-green-300 mb-4">
                    Load Session
                </h3>

                {#if sessions.length === 0}
                    <p class="text-sm text-green-600 text-center py-8">
                        No saved sessions found
                    </p>
                {:else}
                    <div class="space-y-2">
                        {#each sessions as session}
                            <div
                                class="border border-green-900/50 rounded p-3 hover:bg-green-900/10 transition-all"
                            >
                                <div
                                    class="flex items-start justify-between mb-2"
                                >
                                    <div>
                                        <h4
                                            class="text-sm font-bold text-green-300"
                                        >
                                            {session.metadata.title}
                                        </h4>
                                        <p class="text-xs text-green-600">
                                            {new Date(
                                                session.created_at,
                                            ).toLocaleString()}
                                        </p>
                                    </div>
                                    <div class="flex gap-2">
                                        <button
                                            class="text-xs px-2 py-1 bg-green-500/30 text-green-100 rounded border border-green-500 hover:bg-green-500/50"
                                            onclick={() =>
                                                loadSession(session.id)}
                                        >
                                            Load
                                        </button>
                                        <button
                                            class="text-xs px-2 py-1 bg-red-500/30 text-red-100 rounded border border-red-500 hover:bg-red-500/50"
                                            onclick={() =>
                                                deleteSession(session.id)}
                                        >
                                            Delete
                                        </button>
                                    </div>
                                </div>
                                <div class="text-xs text-green-700 flex gap-4">
                                    <span
                                        >📝 {session.metadata.total_transcripts}
                                        transcripts</span
                                    >
                                    <span
                                        >🕸️ {session.graph_nodes?.length || 0} nodes</span
                                    >
                                    <span
                                        >⏱️ {session.metadata
                                            .duration_seconds}s</span
                                    >
                                </div>
                            </div>
                        {/each}
                    </div>
                {/if}

                <button
                    class="god-button w-full mt-4"
                    onclick={() => (showLoadDialog = false)}
                >
                    Close
                </button>
            </div>
        </div>
    {/if}

    <!-- Export Dialog -->
    {#if showExportDialog}
        <div
            class="fixed inset-0 bg-black/80 flex items-center justify-center z-50"
        >
            <div class="god-panel p-6 max-w-md w-full mx-4">
                <h3 class="text-lg font-bold text-green-300 mb-4">
                    Export Session
                </h3>

                <div class="text-xs text-green-400 block mb-2">
                    Export Format
                </div>
                <div class="grid grid-cols-3 gap-2 mb-4">
                    <button
                        class="px-3 py-2 text-xs rounded border transition-all {exportFormat ===
                        'json'
                            ? 'bg-green-500/30 border-green-500 text-green-100'
                            : 'bg-green-900/20 border-green-900 text-green-600'}"
                        onclick={() => (exportFormat = "json")}
                    >
                        JSON
                    </button>
                    <button
                        class="px-3 py-2 text-xs rounded border transition-all {exportFormat ===
                        'csv'
                            ? 'bg-green-500/30 border-green-500 text-green-100'
                            : 'bg-green-900/20 border-green-900 text-green-600'}"
                        onclick={() => (exportFormat = "csv")}
                    >
                        CSV
                    </button>
                    <button
                        class="px-3 py-2 text-xs rounded border transition-all {exportFormat ===
                        'markdown'
                            ? 'bg-green-500/30 border-green-500 text-green-100'
                            : 'bg-green-900/20 border-green-900 text-green-600'}"
                        onclick={() => (exportFormat = "markdown")}
                    >
                        Markdown
                    </button>
                    <button
                        class="px-3 py-2 text-xs rounded border transition-all {exportFormat ===
                        'graphml'
                            ? 'bg-green-500/30 border-green-500 text-green-100'
                            : 'bg-green-900/20 border-green-900 text-green-600'}"
                        onclick={() => (exportFormat = "graphml")}
                    >
                        GraphML
                    </button>
                    <button
                        class="px-3 py-2 text-xs rounded border transition-all {exportFormat ===
                        'entities'
                            ? 'bg-green-500/30 border-green-500 text-green-100'
                            : 'bg-green-900/20 border-green-900 text-green-600'}"
                        onclick={() => (exportFormat = "entities")}
                    >
                        Entities
                    </button>
                </div>

                <div class="flex gap-2">
                    <button class="god-button flex-1" onclick={exportSession}>
                        Export
                    </button>
                    <button
                        class="god-button flex-1 opacity-50"
                        onclick={() => (showExportDialog = false)}
                    >
                        Cancel
                    </button>
                </div>
            </div>
        </div>
    {/if}
</div>
