<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    
    export let nodes: Array<{ id: string; type: string; weight?: number; label?: string }> = [];
    export let edges: Array<{ from: string; to: string; relation: string }> = [];

    // Force-directed layout state
    interface NodePosition {
        x: number;
        y: number;
        vx: number;
        vy: number;
        fx?: number | null; // Fixed x (when dragging)
        fy?: number | null; // Fixed y (when dragging)
    }
    
    let positions: Map<string, NodePosition> = new Map();
    let svgElement: SVGSVGElement;
    let containerWidth = 600;
    let containerHeight = 400;
    let animationFrame: number;
    let isSimulating = true;
    let draggedNode: string | null = null;
    let simulationTick = 0;
    
    // Pan & Zoom state
    let zoomLevel = 1.0;
    let panX = 0;
    let panY = 0;
    let isPanning = false;
    
    // Force simulation constants
    const REPULSION = 5000;
    const ATTRACTION = 0.05;
    const DAMPING = 0.85;
    const CENTER_PULL = 0.01;
    const MIN_DISTANCE = 80;
    
    // Initialize node positions
    function initializePositions() {
        const centerX = containerWidth / 2;
        const centerY = containerHeight / 2;
        
        nodes.forEach((node, i) => {
            if (!positions.has(node.id)) {
                const angle = (i / Math.max(nodes.length, 1)) * 2 * Math.PI;
                const radius = Math.min(containerWidth, containerHeight) * 0.3;
                positions.set(node.id, {
                    x: centerX + radius * Math.cos(angle) + (Math.random() - 0.5) * 50,
                    y: centerY + radius * Math.sin(angle) + (Math.random() - 0.5) * 50,
                    vx: 0,
                    vy: 0,
                });
            }
        });
        
        // Remove positions for deleted nodes
        const nodeIds = new Set(nodes.map(n => n.id));
        for (const id of positions.keys()) {
            if (!nodeIds.has(id)) {
                positions.delete(id);
            }
        }
        
        positions = new Map(positions);
    }
    
    // Force simulation step
    function simulateStep() {
        if (!isSimulating || nodes.length === 0) return;
        
        const centerX = containerWidth / 2;
        const centerY = containerHeight / 2;
        
        // Apply forces
        nodes.forEach((node) => {
            const pos = positions.get(node.id);
            if (!pos || pos.fx !== undefined && pos.fx !== null) return;
            
            let fx = 0, fy = 0;
            
            // Repulsion from other nodes
            nodes.forEach((other) => {
                if (other.id === node.id) return;
                const otherPos = positions.get(other.id);
                if (!otherPos) return;
                
                const dx = pos.x - otherPos.x;
                const dy = pos.y - otherPos.y;
                const dist = Math.max(Math.sqrt(dx * dx + dy * dy), MIN_DISTANCE);
                const force = REPULSION / (dist * dist);
                
                fx += (dx / dist) * force;
                fy += (dy / dist) * force;
            });
            
            // Attraction to connected nodes
            edges.forEach((edge) => {
                let otherId: string | null = null;
                if (edge.from === node.id) otherId = edge.to;
                else if (edge.to === node.id) otherId = edge.from;
                
                if (otherId) {
                    const otherPos = positions.get(otherId);
                    if (otherPos) {
                        const dx = otherPos.x - pos.x;
                        const dy = otherPos.y - pos.y;
                        fx += dx * ATTRACTION;
                        fy += dy * ATTRACTION;
                    }
                }
            });
            
            // Pull toward center
            fx += (centerX - pos.x) * CENTER_PULL;
            fy += (centerY - pos.y) * CENTER_PULL;
            
            // Update velocity
            pos.vx = (pos.vx + fx) * DAMPING;
            pos.vy = (pos.vy + fy) * DAMPING;
            
            // Update position
            pos.x += pos.vx;
            pos.y += pos.vy;
            
            // Boundary constraints
            const padding = 50;
            pos.x = Math.max(padding, Math.min(containerWidth - padding, pos.x));
            pos.y = Math.max(padding, Math.min(containerHeight - padding, pos.y));
        });
        
        positions = new Map(positions);
        simulationTick++;
        
        // Slow down simulation over time
        if (simulationTick > 300) {
            isSimulating = false;
        }
    }
    
    function animate() {
        simulateStep();
        animationFrame = requestAnimationFrame(animate);
    }
    
    // Drag handlers
    function handleMouseDown(event: MouseEvent, nodeId: string) {
        draggedNode = nodeId;
        const pos = positions.get(nodeId);
        if (pos) {
            pos.fx = pos.x;
            pos.fy = pos.y;
        }
        event.preventDefault();
    }
    
    function handleMouseMove(event: MouseEvent) {
        if (!draggedNode || !svgElement) return;
        
        const rect = svgElement.getBoundingClientRect();
        const x = event.clientX - rect.left;
        const y = event.clientY - rect.top;
        
        const pos = positions.get(draggedNode);
        if (pos) {
            pos.x = x;
            pos.y = y;
            pos.fx = x;
            pos.fy = y;
            pos.vx = 0;
            pos.vy = 0;
            positions = new Map(positions);
        }
    }
    
    function handleMouseUp() {
        if (draggedNode) {
            const pos = positions.get(draggedNode);
            if (pos) {
                pos.fx = null;
                pos.fy = null;
            }
            draggedNode = null;
            // Restart simulation briefly
            isSimulating = true;
            simulationTick = 200;
        }
        isPanning = false;
    }

    function handleZoom(event: WheelEvent) {
        event.preventDefault();
        const zoomSpeed = 0.001;
        const delta = -event.deltaY;
        const newZoom = Math.max(0.2, Math.min(3, zoomLevel + delta * zoomSpeed));
        
        // Target focus
        const rect = svgElement.getBoundingClientRect();
        const mouseX = event.clientX - rect.left;
        const mouseY = event.clientY - rect.top;
        
        // Adjust pan to zoom toward mouse
        const zoomRatio = newZoom / zoomLevel;
        panX = mouseX - (mouseX - panX) * zoomRatio;
        panY = mouseY - (mouseY - panY) * zoomRatio;
        zoomLevel = newZoom;
    }

    function handleBackgroundMouseDown(event: MouseEvent) {
        if (event.target === svgElement) {
            isPanning = true;
            event.preventDefault();
        }
    }

    function handleGlobalMouseMove(event: MouseEvent) {
        if (draggedNode) {
            handleMouseMove(event);
        } else if (isPanning) {
            panX += event.movementX;
            panY += event.movementY;
        }
    }

    function getNodeColor(type: string): string {
        const colors: Record<string, string> = {
            TASK: "#00c8ff",
            DECISION: "#4dd2ff",
            PERSON: "#00b4e6",
            DEADLINE: "#ef4444",
            RISK: "#f59e0b",
            ACTION_ITEM: "#10b981",
            Entity: "#00c8ff",
            entity: "#00c8ff",
            default: "#00c8ff",
        };
        return colors[type] || colors.default;
    }
    
    // Reactive updates
    $: if (nodes.length > 0) {
        initializePositions();
        isSimulating = true;
        simulationTick = 0;
    }
    
    onMount(() => {
        // Get container dimensions
        if (svgElement?.parentElement) {
            const rect = svgElement.parentElement.getBoundingClientRect();
            containerWidth = rect.width || 600;
            containerHeight = rect.height || 400;
        }
        
        initializePositions();
        animate();
        
        // Add global mouse handlers
        window.addEventListener('mousemove', handleGlobalMouseMove);
        window.addEventListener('mouseup', handleMouseUp);
        // Only add wheel listener if svg element exists (won't exist if nodes.length === 0)
        if (svgElement) {
            svgElement.addEventListener('wheel', handleZoom, { passive: false });
        }
    });
    
    onDestroy(() => {
        if (animationFrame) cancelAnimationFrame(animationFrame);
        window.removeEventListener('mousemove', handleGlobalMouseMove);
        window.removeEventListener('mouseup', handleMouseUp);
        if (svgElement) svgElement.removeEventListener('wheel', handleZoom);
    });
</script>

<div class="w-full h-full rounded-lg relative overflow-hidden" style="background: linear-gradient(135deg, rgba(13, 17, 23, 0.95) 0%, rgba(10, 12, 15, 0.9) 100%); border: 1px solid rgba(0, 200, 255, 0.15);">
    {#if nodes.length === 0}
        <div class="absolute inset-0 flex items-center justify-center">
            <div class="text-center">
                <svg class="w-10 h-10 mx-auto mb-3 opacity-50 text-cyan-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <circle cx="12" cy="5" r="3"/>
                    <circle cx="5" cy="19" r="3"/>
                    <circle cx="19" cy="19" r="3"/>
                    <line x1="12" y1="8" x2="5" y2="16"/>
                    <line x1="12" y1="8" x2="19" y2="16"/>
                </svg>
                <p class="text-sm text-cyan-500/50">Knowledge graph will appear here</p>
                <p class="text-xs text-slate-500 mt-1">Start recording or simulate to add nodes</p>
            </div>
        </div>
    {:else}
        <svg 
            bind:this={svgElement} 
            class="w-full h-full touch-none" 
            viewBox="0 0 {containerWidth} {containerHeight}"
            onmousedown={handleBackgroundMouseDown}
            role="application"
            aria-label="Knowledge graph visualization"
        >
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

            <g transform="translate({panX}, {panY}) scale({zoomLevel})">
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
                            <g 
                                transform="translate({pos.x}, {pos.y})" 
                                class="cursor-grab transition-transform {draggedNode === node.id ? 'cursor-grabbing' : 'hover:scale-110'}"
                                onmousedown={(e) => handleMouseDown(e, node.id)}
                                role="button"
                                tabindex="0"
                            >
                                <!-- Outer glow ring -->
                                <circle
                                    r="28"
                                    fill="none"
                                    stroke={getNodeColor(node.type)}
                                    stroke-width="1"
                                    stroke-opacity={draggedNode === node.id ? 0.5 : 0.2}
                                    class={draggedNode === node.id ? '' : 'animate-pulse'}
                                />
                                <!-- Main node circle -->
                                <circle
                                    r="22"
                                    fill={getNodeColor(node.type)}
                                    fill-opacity={draggedNode === node.id ? 0.25 : 0.1}
                                    stroke={getNodeColor(node.type)}
                                    stroke-width={draggedNode === node.id ? 3 : 2}
                                    filter="url(#glow)"
                                />
                                <!-- Inner highlight -->
                                <circle
                                    r="16"
                                    fill={getNodeColor(node.type)}
                                    fill-opacity="0.05"
                                />
                                <!-- Node label - show full label or truncated id -->
                                <text
                                    text-anchor="middle"
                                    dy="4"
                                    fill={getNodeColor(node.type)}
                                    font-size="10"
                                    font-weight="500"
                                    class="font-sans pointer-events-none select-none"
                                >
                                    {(node.label || node.id).slice(0, 10)}
                                </text>
                                <!-- Type label -->
                                <text
                                    text-anchor="middle"
                                    dy="32"
                                    fill="#64748b"
                                    font-size="8"
                                    class="font-sans pointer-events-none select-none"
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
                                        class="pointer-events-none select-none"
                                    >
                                        {node.weight}
                                    </text>
                                {/if}
                            </g>
                        {/if}
                    {/each}
                </g>
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
