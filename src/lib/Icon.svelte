<script lang="ts">
    export let name: string;
    export let size: number = 16;
    export let className: string = "";
</script>

<!-- Hand-crafted SVG icons for production quality - no emojis -->
{#if name === "recording"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" class={className}>
        <circle cx="12" cy="12" r="8" class="animate-pulse"/>
    </svg>

{:else if name === "live"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="12" r="4" fill="currentColor"/>
        <path d="M4.93 4.93a10 10 0 0 1 14.14 0"/>
        <path d="M19.07 19.07a10 10 0 0 1-14.14 0"/>
    </svg>

{:else if name === "microphone"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/>
        <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
        <line x1="12" y1="19" x2="12" y2="22"/>
    </svg>

{:else if name === "listening"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M3 18v-6a9 9 0 0 1 18 0v6"/>
        <path d="M21 19a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3zM3 19a2 2 0 0 0 2 2h1a2 2 0 0 0 2-2v-3a2 2 0 0 0-2-2H3z"/>
    </svg>

{:else if name === "check"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" class={className}>
        <polyline points="20 6 9 17 4 12"/>
    </svg>

{:else if name === "x-circle"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="12" r="10"/>
        <line x1="15" y1="9" x2="9" y2="15"/>
        <line x1="9" y1="9" x2="15" y2="15"/>
    </svg>

{:else if name === "warning"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
        <line x1="12" y1="9" x2="12" y2="13"/>
        <line x1="12" y1="17" x2="12.01" y2="17"/>
    </svg>

{:else if name === "settings"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="12" r="3"/>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/>
    </svg>

{:else if name === "key"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M21 2l-2 2m-7.61 7.61a5.5 5.5 0 1 1-7.778 7.778 5.5 5.5 0 0 1 7.777-7.777zm0 0L15.5 7.5m0 0l3 3L22 7l-3-3m-3.5 3.5L19 4"/>
    </svg>

{:else if name === "send"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <line x1="22" y1="2" x2="11" y2="13"/>
        <polygon points="22 2 15 22 11 13 2 9 22 2"/>
    </svg>

{:else if name === "brain"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M12 4.5a2.5 2.5 0 0 0-4.96-.46 2.5 2.5 0 0 0-1.98 3 2.5 2.5 0 0 0-1.32 4.24 3 3 0 0 0 .34 5.58 2.5 2.5 0 0 0 2.96 3.08A2.5 2.5 0 0 0 12 19.5"/>
        <path d="M12 4.5a2.5 2.5 0 0 1 4.96-.46 2.5 2.5 0 0 1 1.98 3 2.5 2.5 0 0 1 1.32 4.24 3 3 0 0 1-.34 5.58 2.5 2.5 0 0 1-2.96 3.08A2.5 2.5 0 0 1 12 19.5"/>
        <path d="M12 4.5v15"/>
    </svg>

{:else if name === "network"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="5" r="3"/>
        <circle cx="5" cy="19" r="3"/>
        <circle cx="19" cy="19" r="3"/>
        <line x1="12" y1="8" x2="5" y2="16"/>
        <line x1="12" y1="8" x2="19" y2="16"/>
    </svg>

{:else if name === "crystal"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <polygon points="12 2 2 7 12 12 22 7 12 2"/>
        <polyline points="2 17 12 22 22 17"/>
        <polyline points="2 12 12 17 22 12"/>
    </svg>

{:else if name === "save"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"/>
        <polyline points="17 21 17 13 7 13 7 21"/>
        <polyline points="7 3 7 8 15 8"/>
    </svg>

{:else if name === "folder"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/>
    </svg>

{:else if name === "chart"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <line x1="18" y1="20" x2="18" y2="10"/>
        <line x1="12" y1="20" x2="12" y2="4"/>
        <line x1="6" y1="20" x2="6" y2="14"/>
    </svg>

{:else if name === "clipboard"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/>
        <rect x="8" y="2" width="8" height="4" rx="1" ry="1"/>
    </svg>

{:else if name === "wave"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M2 12h2a2 2 0 0 1 2 2v1a2 2 0 0 0 2 2h0a2 2 0 0 0 2-2v-4a2 2 0 0 1 2-2h0a2 2 0 0 1 2 2v4a2 2 0 0 0 2 2h0a2 2 0 0 0 2-2v-1a2 2 0 0 1 2-2h2"/>
    </svg>

{:else if name === "sparkle"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5z"/>
        <path d="M19 14l.9 2.7 2.7.9-2.7.9-.9 2.7-.9-2.7-2.7-.9 2.7-.9z"/>
    </svg>

{:else if name === "message"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>
    </svg>

{:else if name === "lightbulb"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M9 18h6"/>
        <path d="M10 22h4"/>
        <path d="M15.09 14c.18-.98.65-1.74 1.41-2.5A4.65 4.65 0 0 0 18 8 6 6 0 0 0 6 8c0 1 .23 2.23 1.5 3.5A4.61 4.61 0 0 1 8.91 14"/>
    </svg>

{:else if name === "bell"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"/>
        <path d="M13.73 21a2 2 0 0 1-3.46 0"/>
    </svg>

{:else if name === "alert"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="12" r="10"/>
        <line x1="12" y1="8" x2="12" y2="12"/>
        <line x1="12" y1="16" x2="12.01" y2="16"/>
    </svg>

{:else if name === "cpu"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <rect x="4" y="4" width="16" height="16" rx="2" ry="2"/>
        <rect x="9" y="9" width="6" height="6"/>
        <line x1="9" y1="1" x2="9" y2="4"/>
        <line x1="15" y1="1" x2="15" y2="4"/>
        <line x1="9" y1="20" x2="9" y2="23"/>
        <line x1="15" y1="20" x2="15" y2="23"/>
        <line x1="20" y1="9" x2="23" y2="9"/>
        <line x1="20" y1="14" x2="23" y2="14"/>
        <line x1="1" y1="9" x2="4" y2="9"/>
        <line x1="1" y1="14" x2="4" y2="14"/>
    </svg>

{:else if name === "zap"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" class={className}>
        <polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/>
    </svg>

{:else if name === "flame"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M12 22c4-2.5 6-6 6-10a6.5 6.5 0 0 0-6-6.5 6.5 6.5 0 0 0-6 6.5c0 4 2 7.5 6 10z"/>
        <path d="M12 22c-2-2-3-4.5-3-7a3 3 0 0 1 3-3 3 3 0 0 1 3 3c0 2.5-1 5-3 7z"/>
    </svg>

{:else if name === "diamond"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M2.7 10.3a2.41 2.41 0 0 0 0 3.41l7.59 7.59a2.41 2.41 0 0 0 3.41 0l7.59-7.59a2.41 2.41 0 0 0 0-3.41l-7.59-7.59a2.41 2.41 0 0 0-3.41 0Z"/>
    </svg>

{:else if name === "audio"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z"/>
        <path d="M19 10v2a7 7 0 0 1-14 0v-2"/>
        <line x1="12" y1="19" x2="12" y2="22"/>
    </svg>

{:else if name === "keyboard"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <rect x="2" y="4" width="20" height="16" rx="2" ry="2"/>
        <path d="M6 8h.001"/>
        <path d="M10 8h.001"/>
        <path d="M14 8h.001"/>
        <path d="M18 8h.001"/>
        <path d="M8 12h.001"/>
        <path d="M12 12h.001"/>
        <path d="M16 12h.001"/>
        <path d="M7 16h10"/>
    </svg>

{:else if name === "stop"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" class={className}>
        <rect x="4" y="4" width="16" height="16" rx="2"/>
    </svg>

{:else if name === "circle"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" class={className}>
        <circle cx="12" cy="12" r="6"/>
    </svg>

{:else if name === "circle-outline"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="12" r="6"/>
    </svg>

{:else if name === "robot"}
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <rect x="3" y="11" width="18" height="10" rx="2"/>
        <circle cx="12" cy="5" r="2"/>
        <path d="M12 7v4"/>
        <line x1="8" y1="16" x2="8" y2="16"/>
        <line x1="16" y1="16" x2="16" y2="16"/>
    </svg>

{:else}
    <!-- Fallback - empty circle -->
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class={className}>
        <circle cx="12" cy="12" r="10"/>
    </svg>
{/if}
