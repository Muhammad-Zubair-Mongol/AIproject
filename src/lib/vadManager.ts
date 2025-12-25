/**
 * Voice Activity Detection (VAD) Manager
 * Implements smart audio buffering to reduce API costs
 */

export interface VADConfig {
    minSpeechDuration: number;      // Minimum speech to buffer before sending (ms)
    silenceThreshold: number;       // RMS threshold below which is considered silence
    silenceDuration: number;        // Silence duration to trigger chunk send (ms)
    minChunkDuration: number;       // Minimum chunk duration to send (ms)
    maxChunkDuration: number;       // Maximum chunk duration before forced send (ms)
    enableFillerDetection: boolean; // Strip filler words
}

export interface VADState {
    isSpeaking: boolean;
    silenceDuration: number;
    speechDuration: number;
    bufferDuration: number;
    chunksSent: number;
    totalSpeechTime: number;
    totalSilenceTime: number;
    vadConfidence: number;
    status: 'idle' | 'buffering' | 'sending' | 'processing';
}

export interface AudioChunk {
    id: string;
    startTime: number;
    endTime: number;
    duration: number;
    speechRatio: number;
    samples: Float32Array;
}

// Common filler words to detect/strip
const FILLER_WORDS = [
    'um', 'uh', 'uhh', 'umm', 'er', 'err', 'ah', 'ahh',
    'like', 'you know', 'basically', 'actually', 'literally',
    'i mean', 'sort of', 'kind of', 'right', 'okay so'
];

class VADManager {
    private config: VADConfig = {
        minSpeechDuration: 1000,    // Detect even 1s of speech (was 18s) - user wants instant
        silenceThreshold: 0.0001,   // ULTRA sensitive (was 0.002) - catch breaths
        silenceDuration: 2000,      // 2 seconds silence (was 4s) - quicker updates
        minChunkDuration: 1000,     // 1s minimum chunk
        maxChunkDuration: 30000,    // 30s max
        enableFillerDetection: true
    };

    private state: VADState = {
        isSpeaking: false,
        silenceDuration: 0,
        speechDuration: 0,
        bufferDuration: 0,
        chunksSent: 0,
        totalSpeechTime: 0,
        totalSilenceTime: 0,
        vadConfidence: 0,
        status: 'idle'
    };

    private audioBuffer: Float32Array[] = [];
    private bufferStartTime: number = 0;
    private lastSpeechTime: number = 0;
    private lastUpdateTime: number = 0;

    private listeners: Set<(state: VADState) => void> = new Set();
    private chunkListeners: Set<(chunk: AudioChunk) => void> = new Set();

    constructor() {
        this.loadConfig();
    }

    // === CONFIGURATION ===

    setConfig(partial: Partial<VADConfig>) {
        this.config = { ...this.config, ...partial };
        this.saveConfig();
    }

    getConfig(): VADConfig {
        return { ...this.config };
    }

    private loadConfig() {
        try {
            const stored = localStorage.getItem('vad_config');
            if (stored) {
                this.config = { ...this.config, ...JSON.parse(stored) };
            }
        } catch (e) {
            console.error('[VAD] Failed to load config:', e);
        }
    }

    private saveConfig() {
        localStorage.setItem('vad_config', JSON.stringify(this.config));
    }

    // === CORE VAD PROCESSING ===

    /**
     * Process audio samples and determine speech/silence
     * Returns true if speech detected
     */
    processAudio(samples: Float32Array, sampleRate: number = 44100): boolean {
        const now = Date.now();
        const deltaTime = this.lastUpdateTime ? now - this.lastUpdateTime : 0;
        this.lastUpdateTime = now;

        // Calculate RMS (Root Mean Square) for volume
        const rms = this.calculateRMS(samples);

        // Determine if speaking based on RMS threshold
        const wasSpeaking = this.state.isSpeaking;
        this.state.isSpeaking = rms > this.config.silenceThreshold;
        this.state.vadConfidence = Math.min(1, rms / (this.config.silenceThreshold * 3));

        if (this.state.isSpeaking) {
            // Speaking
            this.state.speechDuration += deltaTime;
            this.state.totalSpeechTime += deltaTime;
            this.state.silenceDuration = 0;
            this.lastSpeechTime = now;

            // Add to buffer
            if (this.audioBuffer.length === 0) {
                this.bufferStartTime = now;
            }
            this.audioBuffer.push(samples);
            this.state.bufferDuration = now - this.bufferStartTime;
            this.state.status = 'buffering';
        } else {
            // Silence
            this.state.silenceDuration += deltaTime;
            this.state.totalSilenceTime += deltaTime;

            // Still add to buffer if we're in an active speech session
            if (this.state.bufferDuration > 0) {
                this.audioBuffer.push(samples);
                this.state.bufferDuration = now - this.bufferStartTime;
            }
        }

        // Check if we should send a chunk
        this.checkSendConditions(now);

        this.notifyListeners();
        return this.state.isSpeaking;
    }

    /**
     * Process volume level (simpler - when we don't have raw samples)
     */
    processVolume(volume: number): boolean {
        const now = Date.now();
        const deltaTime = this.lastUpdateTime ? now - this.lastUpdateTime : 16;
        this.lastUpdateTime = now;

        const wasSpeaking = this.state.isSpeaking;
        this.state.isSpeaking = volume > this.config.silenceThreshold;
        this.state.vadConfidence = Math.min(1, volume / (this.config.silenceThreshold * 3));

        if (this.state.isSpeaking) {
            this.state.speechDuration += deltaTime;
            this.state.totalSpeechTime += deltaTime;
            this.state.silenceDuration = 0;
            this.lastSpeechTime = now;

            if (this.state.bufferDuration === 0) {
                this.bufferStartTime = now;
            }
            this.state.bufferDuration = now - this.bufferStartTime;
            this.state.status = 'buffering';
        } else {
            this.state.silenceDuration += deltaTime;
            this.state.totalSilenceTime += deltaTime;

            if (this.state.bufferDuration > 0) {
                this.state.bufferDuration = now - this.bufferStartTime;
            }
        }

        this.checkSendConditions(now);
        this.notifyListeners();
        return this.state.isSpeaking;
    }

    private checkSendConditions(now: number) {
        const shouldSend =
            // Enough speech accumulated AND silence detected (end of utterance)
            (this.state.speechDuration >= this.config.minSpeechDuration &&
                this.state.silenceDuration >= this.config.silenceDuration) ||
            // OR max duration reached (force send)
            (this.state.bufferDuration >= this.config.maxChunkDuration);

        // Don't send if chunk is too short
        if (shouldSend && this.state.bufferDuration >= this.config.minChunkDuration) {
            this.sendChunk();
        }
    }

    private sendChunk() {
        if (this.state.bufferDuration < this.config.minChunkDuration) {
            return; // Too short, skip
        }

        const chunk: AudioChunk = {
            id: `chunk_${Date.now()}`,
            startTime: this.bufferStartTime,
            endTime: Date.now(),
            duration: this.state.bufferDuration,
            speechRatio: this.state.speechDuration / Math.max(1, this.state.bufferDuration),
            samples: this.mergeBuffers()
        };

        this.state.chunksSent++;
        this.state.status = 'sending';

        // Notify chunk listeners
        this.chunkListeners.forEach(l => l(chunk));

        // Reset buffer
        this.resetBuffer();

        console.log(`[VAD] Chunk sent: ${(chunk.duration / 1000).toFixed(1)}s, speech ratio: ${(chunk.speechRatio * 100).toFixed(0)}%`);
    }

    private mergeBuffers(): Float32Array {
        if (this.audioBuffer.length === 0) return new Float32Array(0);

        const totalLength = this.audioBuffer.reduce((sum, buf) => sum + buf.length, 0);
        const merged = new Float32Array(totalLength);
        let offset = 0;

        for (const buf of this.audioBuffer) {
            merged.set(buf, offset);
            offset += buf.length;
        }

        return merged;
    }

    private resetBuffer() {
        this.audioBuffer = [];
        this.state.bufferDuration = 0;
        this.state.speechDuration = 0;
        this.state.silenceDuration = 0;
        this.bufferStartTime = 0;
        this.state.status = 'idle';
    }

    private calculateRMS(samples: Float32Array): number {
        let sum = 0;
        for (let i = 0; i < samples.length; i++) {
            sum += samples[i] * samples[i];
        }
        return Math.sqrt(sum / samples.length);
    }

    // === FILLER WORD DETECTION ===

    stripFillerWords(text: string): string {
        if (!this.config.enableFillerDetection) return text;

        let result = text.toLowerCase();
        for (const filler of FILLER_WORDS) {
            // Match filler words as whole words
            const regex = new RegExp(`\\b${filler}\\b`, 'gi');
            result = result.replace(regex, '');
        }

        // Clean up extra spaces
        return result.replace(/\s+/g, ' ').trim();
    }

    detectFillerWords(text: string): string[] {
        const found: string[] = [];
        const lowerText = text.toLowerCase();

        for (const filler of FILLER_WORDS) {
            const regex = new RegExp(`\\b${filler}\\b`, 'gi');
            if (regex.test(lowerText)) {
                found.push(filler);
            }
        }

        return found;
    }

    // === STATE & CONTROL ===

    getState(): VADState {
        return { ...this.state };
    }

    start() {
        this.reset();
        this.state.status = 'idle';
        console.log('[VAD] Started');
    }

    stop() {
        // Send any remaining buffered audio
        if (this.state.bufferDuration >= this.config.minChunkDuration) {
            this.sendChunk();
        }
        this.state.status = 'idle';
        console.log(`[VAD] Stopped. Total speech: ${(this.state.totalSpeechTime / 1000).toFixed(1)}s, chunks sent: ${this.state.chunksSent}`);
    }

    reset() {
        this.resetBuffer();
        this.state = {
            isSpeaking: false,
            silenceDuration: 0,
            speechDuration: 0,
            bufferDuration: 0,
            chunksSent: 0,
            totalSpeechTime: 0,
            totalSilenceTime: 0,
            vadConfidence: 0,
            status: 'idle'
        };
        this.lastUpdateTime = 0;
    }

    forceFlush() {
        if (this.audioBuffer.length > 0 || this.state.bufferDuration > 0) {
            this.sendChunk();
        }
    }

    // === LISTENERS ===

    subscribe(listener: (state: VADState) => void): () => void {
        this.listeners.add(listener);
        listener(this.state);
        return () => this.listeners.delete(listener);
    }

    onChunk(listener: (chunk: AudioChunk) => void): () => void {
        this.chunkListeners.add(listener);
        return () => this.chunkListeners.delete(listener);
    }

    private notifyListeners() {
        this.listeners.forEach(l => l({ ...this.state }));
    }

    // === UTILITIES ===

    getStats() {
        return {
            totalSpeechTime: this.state.totalSpeechTime,
            totalSilenceTime: this.state.totalSilenceTime,
            speechRatio: this.state.totalSpeechTime / Math.max(1, this.state.totalSpeechTime + this.state.totalSilenceTime),
            chunksSent: this.state.chunksSent,
            avgChunkDuration: this.state.chunksSent > 0 ? this.state.totalSpeechTime / this.state.chunksSent : 0
        };
    }
}

// Singleton instance
export const vadManager = new VADManager();
