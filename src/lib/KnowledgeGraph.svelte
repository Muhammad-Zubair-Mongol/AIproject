<script lang="ts">
    export let nodes: Array<{ id: string; type: string; weight?: number }> = [];
    export let edges: Array<{ from: string; to: string; relation: string }> = [];

    // Simple force-directed layout simulation
    let positions: Map<string, { x: number; y: number }> = new Map();

    $: {
        // Initialize positions if nodes change
        nodes.forEach((node, i) => {
            if (!positions.has(node.id)) {
                const angle = (i / nodes.length) * 2 * Math.PI;
                const radius = 150;
                positions.set(node.id, {
                    x: 300 + radius * Math.cos(angle),
                    y: 200 + radius * Math.sin(angle),
                });
            }
        });
        positions = positions; // Trigger reactivity
    }

    function getNodeColor(type: string): string {
        const colors: Record<string, string> = {
            TASK: "#00c8ff",
            DECISION: "#4dd2ff",
            PERSON: "#00b4e6",
            DEADLINE: "#ef4444",
            Entity: "#00c8ff",
            default: "#00c8ff",
        };
        return colors[type] || colors.default;
    }
</script>

<div class="w-full h-full rounded-lg relative overflow-hidden" style="background: linear-gradient(135deg, rgba(13, 17, 23, 0.95) 0%, rgba(10, 12, 15, 0.9) 100%); border: 1px solid rgba(0, 200, 255, 0.15);">
    {#if nodes.length === 0}
        <div class="absolute inset-0 flex items-center justify-center">
            <div class="text-center">
                <div class="text-4xl mb-3 opacity-50">🕸️</div>
                <p class="text-sm text-cyan-500/50">Knowledge graph will appear here</p>
                <p class="text-xs text-slate-500 mt-1">Add nodes to visualize relationships</p>
            </div>
        </div>
    {:else}
        <svg class="w-full h-full">
            <!-- Glow filter -->
            <defs>
                <filter id="glow" x="-50%" y="-50%" width="200%" height="200%">
                    <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
                    <feMerge>
                        <feMergeNode in="coloredBlur"/>
                        <feMergeNode in="SourceGraphic"/>
                    </feMerge>
                </filter>
                
                <linearGradient id="edgeGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                    <stop offset="0%" style="stop-color:#00c8ff;stop-opacity:0.1" />
                    <stop offset="50%" style="stop-color:#00c8ff;stop-opacity:0.4" />
                    <stop offset="100%" style="stop-color:#00c8ff;stop-opacity:0.1" />
                </linearGradient>
                
                <marker
                    id="arrowhead"
                    markerWidth="10"
                    markerHeight="10"
                    refX="9"
                    refY="3"
                    orient="auto"
                >
                    <polygon
                        points="0 0, 10 3, 0 6"
                        fill="#00c8ff"
                        opacity="0.5"
                    />
                </marker>
            </defs>

            <!-- Edges -->
            <g class="edges">
                {#each edges as edge}
                    {@const from = positions.get(edge.from)}
                    {@const to = positions.get(edge.to)}
                    {#if from && to}
                        <line
                            x1={from.x}
                            y1={from.y}
                            x2={to.x}
                            y2={to.y}
                            stroke="url(#edgeGradient)"
                            stroke-width="2"
                            marker-end="url(#arrowhead)"
                            class="transition-all duration-300"
                        />
                        <text
                            x={(from.x + to.x) / 2}
                            y={(from.y + to.y) / 2}
                            fill="#00c8ff"
                            font-size="10"
                            opacity="0.6"
                            text-anchor="middle"
                            class="font-sans"
                        >
                            {edge.relation}
                        </text>
                    {/if}
                {/each}
            </g>

            <!-- Nodes -->
            <g class="nodes">
                {#each nodes as node}
                    {@const pos = positions.get(node.id)}
                    {#if pos}
                        <g transform="translate({pos.x}, {pos.y})" class="cursor-pointer transition-transform hover:scale-110">
                            <!-- Outer glow ring -->
                            <circle
                                r="28"
                                fill="none"
                                stroke={getNodeColor(node.type)}
                                stroke-width="1"
                                stroke-opacity="0.2"
                                class="animate-pulse"
                            />
                            <!-- Main node circle -->
                            <circle
                                r="22"
                                fill={getNodeColor(node.type)}
                                fill-opacity="0.1"
                                stroke={getNodeColor(node.type)}
                                stroke-width="2"
                                filter="url(#glow)"
                            />
                            <!-- Inner highlight -->
                            <circle
                                r="16"
                                fill={getNodeColor(node.type)}
                                fill-opacity="0.05"
                            />
                            <!-- Node label -->
                            <text
                                text-anchor="middle"
                                dy="4"
                                fill={getNodeColor(node.type)}
                                font-size="11"
                                font-weight="500"
                                class="font-sans"
                            >
                                {node.id.slice(0, 8)}
                            </text>
                            <!-- Type label -->
                            <text
                                text-anchor="middle"
                                dy="32"
                                fill="#64748b"
                                font-size="9"
                                class="font-sans"
                            >
                                {node.type}
                            </text>
                            <!-- Weight indicator -->
                            {#if node.weight && node.weight > 1}
                                <circle
                                    cx="18"
                                    cy="-18"
                                    r="8"
                                    fill="#00c8ff"
                                    fill-opacity="0.3"
                                    stroke="#00c8ff"
                                    stroke-width="1"
                                />
                                <text
                                    x="18"
                                    y="-14"
                                    text-anchor="middle"
                                    fill="#00c8ff"
                                    font-size="9"
                                    font-weight="bold"
                                >
                                    {node.weight}
                                </text>
                            {/if}
                        </g>
                    {/if}
                {/each}
            </g>
        </svg>
    {/if}
    
    <!-- Stats overlay -->
    <div class="absolute bottom-3 left-3 text-xs text-slate-500 flex gap-3">
        <span class="flex items-center gap-1">
            <span class="w-2 h-2 rounded-full bg-cyan-500/50"></span>
            {nodes.length} nodes
        </span>
        <span class="flex items-center gap-1">
            <span class="w-2 h-2 rounded-full bg-cyan-500/30"></span>
            {edges.length} edges
        </span>
    </div>
</div>
