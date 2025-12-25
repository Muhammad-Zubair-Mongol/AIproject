/**
 * API Key Manager - Smart round-robin rotation with fallback
 */

export interface ApiKey {
    id: string;
    key: string;
    name: string;
    isActive: boolean;
    isPrimary: boolean;
    lastUsed: string | null;
    rateLimited: boolean;
    rateLimitedUntil: number | null;
    failCount: number;
    isDisabled: boolean;
    usageCount: number;
    lastError?: string;
    cooldownUntil?: number;
}

export interface KeyManagerState {
    keys: ApiKey[];
    currentIndex: number;
    shuffleMode: boolean;
    totalCalls: number;
    lastError: string | null;
    lastRotationReason?: string;
}

const STORAGE_KEY = "gemini_api_keys_v2";
const STATE_KEY = "key_manager_state";
const RATE_LIMIT_COOLDOWN_MS = 60000; // 1 minute cooldown after rate limit
const QUOTA_EXHAUSTED_COOLDOWN_MS = 3600000; // 1 hour for quota exhaustion
const CONNECTION_TIMEOUT_MS = 3000; // 3 second timeout for fast feedback
const MAX_FAIL_COUNT = 3;

class ApiKeyManager {
    private state: KeyManagerState = {
        keys: [],
        currentIndex: 0,
        shuffleMode: false,
        totalCalls: 0,
        lastError: null
    };

    private listeners: Set<(state: KeyManagerState) => void> = new Set();

    constructor() {
        this.loadState();
    }

    // === STATE MANAGEMENT ===

    private loadState() {
        try {
            // Load keys
            const storedKeys = localStorage.getItem(STORAGE_KEY);
            if (storedKeys) {
                this.state.keys = JSON.parse(storedKeys);
            }

            // Migrate from old format if needed
            const oldKeys = localStorage.getItem("gemini_api_keys");
            if (oldKeys && this.state.keys.length === 0) {
                const oldFormat = JSON.parse(oldKeys);
                this.state.keys = oldFormat.map((k: any, i: number) => ({
                    ...k,
                    isPrimary: i === 0,
                    failCount: 0,
                    isDisabled: false,
                    usageCount: 0,
                    rateLimitedUntil: null
                }));
                this.saveState();
                localStorage.removeItem("gemini_api_keys");
            }

            // Load state
            const storedState = localStorage.getItem(STATE_KEY);
            if (storedState) {
                const parsed = JSON.parse(storedState);
                this.state.currentIndex = parsed.currentIndex || 0;
                this.state.shuffleMode = parsed.shuffleMode || false;
                this.state.totalCalls = parsed.totalCalls || 0;
            }

            // Validate current index
            if (this.state.currentIndex >= this.state.keys.length) {
                this.state.currentIndex = 0;
            }
        } catch (e) {
            console.error("[KeyManager] Failed to load state:", e);
        }
    }

    private saveState() {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(this.state.keys));
            localStorage.setItem(STATE_KEY, JSON.stringify({
                currentIndex: this.state.currentIndex,
                shuffleMode: this.state.shuffleMode,
                totalCalls: this.state.totalCalls
            }));
        } catch (e) {
            console.error("[KeyManager] Failed to save state:", e);
        }
    }

    // === KEY MANAGEMENT ===

    addKey(key: string, name?: string): ApiKey {
        const newKey: ApiKey = {
            id: `key_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            key: key.trim(),
            name: name?.trim() || `Key ${this.state.keys.length + 1}`,
            isActive: this.state.keys.length === 0,
            isPrimary: this.state.keys.length === 0,
            lastUsed: null,
            rateLimited: false,
            rateLimitedUntil: null,
            failCount: 0,
            isDisabled: false,
            usageCount: 0
        };

        this.state.keys.push(newKey);
        this.saveState();
        this.notifyListeners();

        console.log(`[KeyManager] Added key: ${newKey.name} (${this.state.keys.length} total)`);
        return newKey;
    }

    removeKey(id: string) {
        const index = this.state.keys.findIndex(k => k.id === id);
        if (index === -1) return;

        const wasPrimary = this.state.keys[index].isPrimary;
        this.state.keys = this.state.keys.filter(k => k.id !== id);

        // If we removed the current key, adjust index
        if (this.state.currentIndex >= this.state.keys.length) {
            this.state.currentIndex = Math.max(0, this.state.keys.length - 1);
        }

        // If we removed primary, make first key primary
        if (wasPrimary && this.state.keys.length > 0) {
            this.state.keys[0].isPrimary = true;
        }

        this.saveState();
        this.notifyListeners();
    }

    getKeys(): ApiKey[] {
        return [...this.state.keys];
    }

    getKeyCount(): number {
        return this.state.keys.length;
    }

    getActiveKeyCount(): number {
        return this.state.keys.filter(k => !k.isDisabled && !this.isRateLimited(k)).length;
    }

    // === ROTATION LOGIC ===

    /**
     * Get the next available key for an API call
     * Implements round-robin with smart fallback
     */
    getNextKey(): ApiKey | null {
        // First, refresh states to clear any expired cooldowns
        this.refreshKeyStates();

        if (this.state.keys.length === 0) return null;

        const availableKeys = this.state.keys
            .map((k, i) => ({ key: k, index: i }))
            .filter(({ key }) => !key.isDisabled && !this.isRateLimited(key));

        if (availableKeys.length === 0) {
            // All keys exhausted - try primary as last resort
            const primary = this.state.keys.find(k => k.isPrimary && !k.isDisabled);
            if (primary) {
                console.warn("[KeyManager] All keys rate-limited, using primary as fallback");
                return primary;
            }
            console.error("[KeyManager] No available keys!");
            return null;
        }

        let nextKey: ApiKey;
        let nextIndex: number;

        if (this.state.shuffleMode) {
            // Random selection from available keys
            const randomIdx = Math.floor(Math.random() * availableKeys.length);
            nextKey = availableKeys[randomIdx].key;
            nextIndex = availableKeys[randomIdx].index;
        } else {
            // Round-robin: find next available starting from current
            let found = false;
            for (let i = 0; i < this.state.keys.length; i++) {
                const checkIndex = (this.state.currentIndex + i) % this.state.keys.length;
                const key = this.state.keys[checkIndex];
                if (!key.isDisabled && !this.isRateLimited(key)) {
                    nextKey = key;
                    nextIndex = checkIndex;
                    found = true;
                    break;
                }
            }
            if (!found) {
                nextKey = availableKeys[0].key;
                nextIndex = availableKeys[0].index;
            }
        }

        // Update state
        this.state.currentIndex = (nextIndex! + 1) % this.state.keys.length;
        nextKey!.usageCount++;
        nextKey!.lastUsed = new Date().toISOString();
        nextKey!.isActive = true;

        // Deactivate others
        this.state.keys.forEach((k, i) => {
            if (i !== nextIndex) k.isActive = false;
        });

        this.state.totalCalls++;
        this.saveState();
        this.notifyListeners();

        console.log(`[KeyManager] Using key: ${nextKey!.name} (${nextIndex! + 1}/${this.state.keys.length})`);
        return nextKey!;
    }

    /**
     * Get current active key without rotating
     */
    getCurrentKey(): ApiKey | null {
        const active = this.state.keys.find(k => k.isActive && !k.isDisabled);
        if (active) return active;

        // Fallback to first available
        return this.state.keys.find(k => !k.isDisabled && !this.isRateLimited(k)) || null;
    }

    getCurrentKeyInfo(): { name: string; index: number; total: number } | null {
        const current = this.getCurrentKey();
        if (!current) return null;

        const index = this.state.keys.findIndex(k => k.id === current.id);
        return {
            name: current.name,
            index: index + 1,
            total: this.state.keys.length
        };
    }

    // === ERROR HANDLING ===

    /**
     * Handle API error and switch keys if needed
     * Returns true if switched to new key, false if no more keys available
     */
    handleError(errorCode: number, errorMessage?: string): { switched: boolean; newKey: ApiKey | null; message: string } {
        const current = this.getCurrentKey();
        if (!current) {
            return { switched: false, newKey: null, message: "No keys available" };
        }

        const isRateLimit = errorCode === 429 ||
            (errorMessage?.toLowerCase().includes("rate limit"));
        const isQuotaExhausted = errorMessage?.toLowerCase().includes("quota");
        const isServerError = errorCode >= 500 && errorCode < 600;
        const isAuthError = errorCode === 401 || errorCode === 403;

        current.lastError = errorMessage || `Error ${errorCode}`;

        if (isRateLimit) {
            current.rateLimited = true;
            current.rateLimitedUntil = Date.now() + RATE_LIMIT_COOLDOWN_MS;
            current.failCount++;
            this.state.lastRotationReason = `Rate limit on ${current.name}`;
            console.warn(`[KeyManager] Rate limit on ${current.name}, cooldown for ${RATE_LIMIT_COOLDOWN_MS / 1000}s`);
        } else if (isQuotaExhausted) {
            current.rateLimited = true;
            current.rateLimitedUntil = Date.now() + QUOTA_EXHAUSTED_COOLDOWN_MS;
            current.failCount++;
            this.state.lastRotationReason = `Quota exhausted on ${current.name}`;
            console.warn(`[KeyManager] Quota exhausted on ${current.name}, cooldown for 1h`);
        } else if (isAuthError) {
            current.isDisabled = true;
            current.failCount = MAX_FAIL_COUNT;
            this.state.lastRotationReason = `Auth error on ${current.name}`;
            console.error(`[KeyManager] Auth error on ${current.name}, key disabled`);
        } else if (isServerError) {
            current.failCount++;
            this.state.lastRotationReason = `Server error on ${current.name}`;
        }

        if (current.failCount >= MAX_FAIL_COUNT && !current.isPrimary) {
            current.isDisabled = true;
            console.warn(`[KeyManager] Key ${current.name} disabled after ${MAX_FAIL_COUNT} failures`);
        }

        this.state.lastError = `${errorCode}: ${errorMessage || 'Unknown error'}`;
        this.saveState();

        const nextKey = this.getNextKey();
        if (nextKey && nextKey.id !== current.id) {
            const message = isRateLimit ? `Rate limit hit on ${current.name} – switching to ${nextKey.name}` :
                isQuotaExhausted ? `Quota exhausted on ${current.name} – switching to ${nextKey.name}` :
                    `Error on ${current.name} – switching to ${nextKey.name}`;

            this.notifyListeners();
            return { switched: true, newKey: nextKey, message };
        }

        this.notifyListeners();
        return { switched: false, newKey: null, message: `All keys exhausted or unavailable` };
    }

    /**
     * Report successful API call - reset fail count
     */
    reportSuccess() {
        const current = this.getCurrentKey();
        if (current) {
            current.failCount = 0;
            current.rateLimited = false;
            current.rateLimitedUntil = null;
            this.saveState();
        }
    }

    private isRateLimited(key: ApiKey): boolean {
        if (!key.rateLimited) return false;
        if (!key.rateLimitedUntil) return false;

        if (Date.now() >= key.rateLimitedUntil) {
            // Cooldown expired - clear rate limit and re-enable
            key.rateLimited = false;
            key.rateLimitedUntil = null;
            key.failCount = 0; // Reset fail count after cooldown
            if (key.isDisabled && !key.isPrimary) {
                key.isDisabled = false; // Re-enable after cooldown
                console.log(`[KeyManager] ${key.name} cooldown expired - re-enabled`);
            }
            return false;
        }

        return true;
    }

    /**
     * Refresh all key states - clear expired cooldowns and re-enable disabled keys
     * Call this before any key operation to ensure fresh state
     */
    refreshKeyStates(): void {
        const now = Date.now();
        let changed = false;

        this.state.keys.forEach(key => {
            // Clear expired rate limits AND re-enable the key
            if (key.rateLimitedUntil && now >= key.rateLimitedUntil) {
                key.rateLimited = false;
                key.rateLimitedUntil = null;
                key.failCount = 0;
                key.isDisabled = false; // RE-ENABLE when rate limit expires!
                changed = true;
                console.log(`[KeyManager] ${key.name} rate limit expired - re-enabled`);
            }

            // Clear expired quota cooldowns AND re-enable the key
            if (key.cooldownUntil && now >= key.cooldownUntil) {
                key.cooldownUntil = undefined;
                key.failCount = 0;
                key.isDisabled = false; // RE-ENABLE when quota cooldown expires!
                changed = true;
                console.log(`[KeyManager] ${key.name} quota cooldown expired - re-enabled`);
            }

            // Also re-enable keys that are disabled but have no active cooldowns
            // (this catches keys that were disabled manually or by old logic)
            if (key.isDisabled && !key.rateLimited && !key.rateLimitedUntil && !key.cooldownUntil) {
                key.isDisabled = false;
                key.failCount = 0;
                changed = true;
                console.log(`[KeyManager] ${key.name} re-enabled (no active cooldowns)`);
            }
        });

        if (changed) {
            this.saveState();
            this.notifyListeners();
        }
    }

    /**
     * Force reset all key cooldowns (user action)
     */
    resetAllCooldowns(): void {
        this.state.keys.forEach(key => {
            key.rateLimited = false;
            key.rateLimitedUntil = null;
            key.cooldownUntil = undefined;
            key.isDisabled = false;
            key.failCount = 0;
        });
        this.saveState();
        this.notifyListeners();
        console.log("[KeyManager] All key cooldowns reset");
    }

    // === SETTINGS ===

    setShuffleMode(enabled: boolean) {
        this.state.shuffleMode = enabled;
        this.saveState();
        this.notifyListeners();
    }

    isShuffleMode(): boolean {
        return this.state.shuffleMode;
    }

    // === LISTENERS ===

    subscribe(listener: (state: KeyManagerState) => void): () => void {
        this.listeners.add(listener);
        listener(this.state);
        return () => this.listeners.delete(listener);
    }

    private notifyListeners() {
        this.listeners.forEach(l => l({ ...this.state }));
    }

    getState(): KeyManagerState {
        return { ...this.state };
    }

    // === UTILITIES ===

    maskKey(key: string): string {
        if (key.length <= 8) return "••••••••";
        return key.slice(0, 4) + "••••••••" + key.slice(-4);
    }

    /**
     * SMART: Find the next working key before recording starts.
     * Skips rate-limited and quota-exhausted keys automatically.
     * Uses fast 3-second timeout per key.
     */
    async getNextWorkingKeyFast(): Promise<{ success: boolean; key?: ApiKey; message: string }> {
        // First, refresh states to clear expired cooldowns and re-enable keys
        this.refreshKeyStates();

        const keys = this.state.keys.filter(k => !k.isDisabled && !this.isRateLimited(k));

        if (keys.length === 0) {
            // Check if all keys are just rate-limited (temporary)
            const rateLimitedOnly = this.state.keys.filter(k => !k.isDisabled && this.isRateLimited(k));
            if (rateLimitedOnly.length > 0) {
                return { success: false, message: `All ${rateLimitedOnly.length} keys rate-limited. Wait ${Math.ceil(RATE_LIMIT_COOLDOWN_MS / 1000)}s.` };
            }
            return { success: false, message: "No API keys configured" };
        }

        console.log(`[KeyManager] Fast-checking ${keys.length} keys...`);

        for (let i = 0; i < keys.length; i++) {
            const k = keys[i];
            console.log(`[KeyManager] Trying key ${i + 1}/${keys.length}: ${k.name}`);

            const result = await this.testConnection(k.key);

            if (result.success) {
                // Found working key - activate it
                k.isActive = true;
                this.state.keys.forEach(other => { if (other.id !== k.id) other.isActive = false; });
                this.state.currentIndex = this.state.keys.findIndex(x => x.id === k.id);
                this.saveState();
                this.notifyListeners();

                console.log(`[KeyManager] Found working key: ${k.name}`);
                return { success: true, key: k, message: `Connected via ${k.name}` };
            }

            // Mark failures appropriately
            if (result.statusCode === 429) {
                k.rateLimited = true;
                k.rateLimitedUntil = Date.now() + RATE_LIMIT_COOLDOWN_MS;
                k.failCount++;
                console.log(`[KeyManager] ${k.name} rate-limited, trying next...`);
            } else if (result.statusCode === 403 || result.message.includes("quota")) {
                k.cooldownUntil = Date.now() + QUOTA_EXHAUSTED_COOLDOWN_MS;
                console.log(`[KeyManager] ${k.name} quota exhausted, skipping for 1 hour`);
            }
        }

        return { success: false, message: "All keys failed - check quota/rate limits" };
    }

    /**
     * SMART: Rotate to next key after each API call (for load distribution)
     */
    rotateToNextKey(): ApiKey | null {
        const available = this.state.keys.filter(k => !k.isDisabled && !this.isRateLimited(k));

        if (available.length <= 1) {
            return available[0] || null; // No rotation possible
        }

        // Move to next available key
        const currentIdx = this.state.currentIndex;
        let nextIdx = (currentIdx + 1) % this.state.keys.length;

        // Find next available (skip disabled/rate-limited)
        for (let i = 0; i < this.state.keys.length; i++) {
            const candidate = this.state.keys[nextIdx];
            if (!candidate.isDisabled && !this.isRateLimited(candidate)) {
                this.state.currentIndex = nextIdx;
                candidate.isActive = true;
                this.state.keys.forEach((k, idx) => { if (idx !== nextIdx) k.isActive = false; });
                this.saveState();
                this.notifyListeners();

                console.log(`[KeyManager] Rotated to: ${candidate.name} (${nextIdx + 1}/${this.state.keys.length})`);
                return candidate;
            }
            nextIdx = (nextIdx + 1) % this.state.keys.length;
        }

        return null;
    }

    /**
     * Test a specific API key against Gemini API (with 3s timeout for fast feedback)
     */
    async testConnection(key: string): Promise<{ success: boolean; message: string; statusCode?: number }> {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), CONNECTION_TIMEOUT_MS);

        try {
            console.log(`[KeyManager] Testing connection (timeout: ${CONNECTION_TIMEOUT_MS}ms)...`);

            const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-preview-09-2025:generateContent?key=${key}`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    contents: [{ parts: [{ text: "Hi" }] }]
                }),
                signal: controller.signal
            });

            clearTimeout(timeoutId);

            if (response.ok) {
                console.log("[KeyManager] Connection successful!");
                return { success: true, message: "Connected", statusCode: 200 };
            } else if (response.status === 429) {
                console.warn("[KeyManager] Rate limited (429) - will try next key");
                return { success: false, message: "Rate limited", statusCode: 429 };
            } else if (response.status === 403) {
                console.error("[KeyManager] Quota exhausted (403)");
                return { success: false, message: "Quota exhausted", statusCode: 403 };
            } else {
                const error = await response.json().catch(() => ({}));
                const msg = error.error?.message || `HTTP ${response.status}`;
                console.warn(`[KeyManager] Failed: ${msg}`);
                return { success: false, message: msg, statusCode: response.status };
            }
        } catch (e: any) {
            clearTimeout(timeoutId);
            if (e.name === 'AbortError') {
                console.error(`[KeyManager] Timed out after ${CONNECTION_TIMEOUT_MS / 1000}s`);
                return { success: false, message: `Timeout (${CONNECTION_TIMEOUT_MS / 1000}s)` };
            }
            console.error("[KeyManager] Network error:", e.message);
            return { success: false, message: "Network error" };
        }
    }

    /**
     * Automatically validate saved keys on startup
     */
    async validateOnStartup(): Promise<{ success: boolean; key?: ApiKey; message: string }> {
        const keys = this.state.keys.filter(k => !k.isDisabled);
        if (keys.length === 0) return { success: false, message: "No keys found" };

        // Try primary first, then others
        const primary = keys.find(k => k.isPrimary) || keys[0];
        console.log(`[KeyManager] Validating primary key: ${primary.name}`);

        const result = await this.testConnection(primary.key);
        if (result.success) {
            primary.isActive = true;
            this.state.keys.forEach(k => { if (k.id !== primary.id) k.isActive = false; });
            this.saveState();
            this.notifyListeners();
            return { success: true, key: primary, message: "Primary key connected" };
        }

        // Broad fallback: just find any working key
        for (const k of keys) {
            if (k.id === primary.id) continue;
            console.log(`[KeyManager] Validating fallback key: ${k.name}`);
            const res = await this.testConnection(k.key);
            if (res.success) {
                k.isActive = true;
                this.state.keys.forEach(other => { if (other.id !== k.id) other.isActive = false; });
                this.saveState();
                this.notifyListeners();
                return { success: true, key: k, message: `Connected to ${k.name}` };
            }
        }

        return { success: false, message: "All keys failed validation" };
    }

    reset() {
        this.state = {
            keys: [],
            currentIndex: 0,
            shuffleMode: false,
            totalCalls: 0,
            lastError: null
        };
        this.saveState();
        this.notifyListeners();
    }
}

// Singleton instance
export const keyManager = new ApiKeyManager();
