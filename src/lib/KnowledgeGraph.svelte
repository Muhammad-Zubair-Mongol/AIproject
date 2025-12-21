<script lang="ts">
    export let nodes: Array<{ id: string; type: string }> = [];
    export let edges: Array<{ from: string; to: string; relation: string }> =
        [];

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
            TASK: "#00ff41",
            DECISION: "#00e68a",
            PERSON: "#1affaa",
            DEADLINE: "#ff4141",
            default: "#80ffce",
        };
        return colors[type] || colors.default;
    }
</script>

<div
    class="w-full h-full bg-black/50 rounded-lg border border-green-900/50 relative overflow-hidden"
>
    {#if nodes.length === 0}
        <div
            class="absolute inset-0 flex items-center justify-center text-green-700"
        >
            <div class="text-center">
                <div class="text-3xl mb-2">🕸️</div>
                <p class="text-sm">Knowledge graph will appear here</p>
            </div>
        </div>
    {:else}
        <svg class="w-full h-full">
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
                            stroke="#00ff41"
                            stroke-width="1"
                            stroke-opacity="0.3"
                            marker-end="url(#arrowhead)"
                        />
                        <text
                            x={(from.x + to.x) / 2}
                            y={(from.y + to.y) / 2}
                            fill="#00ff41"
                            font-size="10"
                            opacity="0.7"
                            text-anchor="middle"
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
                        <g transform="translate({pos.x}, {pos.y})">
                            <circle
                                r="20"
                                fill={getNodeColor(node.type)}
                                fill-opacity="0.2"
                                stroke={getNodeColor(node.type)}
                                stroke-width="2"
                                class="animate-pulse-slow"
                            />
                            <text
                                text-anchor="middle"
                                dy="4"
                                fill={getNodeColor(node.type)}
                                font-size="10"
                                font-weight="bold"
                            >
                                {node.id.slice(0, 8)}
                            </text>
                            <text
                                text-anchor="middle"
                                dy="30"
                                fill="#00ff41"
                                font-size="8"
                                opacity="0.7"
                            >
                                {node.type}
                            </text>
                        </g>
                    {/if}
                {/each}
            </g>

            <!-- Arrow marker definition -->
            <defs>
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
                        fill="#00ff41"
                        opacity="0.5"
                    />
                </marker>
            </defs>
        </svg>
    {/if}
</div>
