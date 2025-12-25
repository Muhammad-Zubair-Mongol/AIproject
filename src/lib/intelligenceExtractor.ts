/**
 * Intelligence Extractor - Extracts structured insights from transcripts
 * Uses Gemini API with the enabled filters to extract Tasks, Decisions, etc.
 */

import { keyManager } from './keyManager';

export interface ExtractedInsights {
    tasks: Array<{ text: string; assignee?: string; priority?: string }>;
    decisions: Array<{ text: string; context?: string }>;
    deadlines: Array<{ text: string; date?: string; owner?: string }>;
    actionItems: Array<{ text: string; assignee?: string; dueDate?: string }>;
    risks: Array<{ text: string; severity?: 'low' | 'medium' | 'high' }>;
    urgency: Array<{ text: string; level: 'low' | 'medium' | 'high' | 'critical' }>;
    sentiment: { overall: string; shifts: string[] };
    keyTopics: string[];
    speakers: Array<{ id: string; name: string; turns: number }>;
}

export interface TranscriptWithSpeaker {
    id: string;
    timestamp: string;
    speaker: string;
    speakerId: number;
    text: string;
    tone?: string;
    sentiment?: 'positive' | 'negative' | 'neutral';
    fillers?: string[];
    isPartial?: boolean;
}

export interface IntelligenceFilters {
    tasks: boolean;
    decisions: boolean;
    deadlines: boolean;
    actionItems: boolean;
    risks: boolean;
    urgency: boolean;
    sentiment: boolean;
    interruptions: boolean;
    agreement: boolean;
    disagreement: boolean;
    emotionShifts: boolean;
    topicDrifts: boolean;
}

const DEFAULT_FILTERS: IntelligenceFilters = {
    tasks: true,
    decisions: true,
    deadlines: true,
    actionItems: true,
    risks: true,
    urgency: true,
    sentiment: false,
    interruptions: false,
    agreement: false,
    disagreement: false,
    emotionShifts: false,
    topicDrifts: false
};

class IntelligenceExtractor {
    private filters: IntelligenceFilters = DEFAULT_FILTERS;
    private insights: ExtractedInsights = this.getEmptyInsights();
    private listeners: Set<(insights: ExtractedInsights) => void> = new Set();
    private isProcessing = false;

    constructor() {
        this.loadFilters();
    }

    private getEmptyInsights(): ExtractedInsights {
        return {
            tasks: [],
            decisions: [],
            deadlines: [],
            actionItems: [],
            risks: [],
            urgency: [],
            sentiment: { overall: 'neutral', shifts: [] },
            keyTopics: [],
            speakers: []
        };
    }

    setFilters(filters: Partial<IntelligenceFilters>) {
        this.filters = { ...this.filters, ...filters };
        localStorage.setItem('intelligence_filters', JSON.stringify(this.filters));
    }

    getFilters(): IntelligenceFilters {
        return { ...this.filters };
    }

    private loadFilters() {
        try {
            const stored = localStorage.getItem('intelligence_filters');
            if (stored) {
                this.filters = { ...this.filters, ...JSON.parse(stored) };
            }
        } catch (e) {
            console.error('[Intelligence] Failed to load filters:', e);
        }
    }

    getInsights(): ExtractedInsights {
        return { ...this.insights };
    }

    reset() {
        this.insights = this.getEmptyInsights();
        this.notifyListeners();
    }

    /**
     * Build the Gemini prompt based on enabled filters
     */
    private buildExtractionPrompt(transcriptText: string): string {
        const enabledFilters: string[] = [];

        if (this.filters.tasks) {
            enabledFilters.push('- "tasks": array of {text, assignee?, priority?} for any tasks mentioned');
        }
        if (this.filters.decisions) {
            enabledFilters.push('- "decisions": array of {text, context?} for decisions made');
        }
        if (this.filters.deadlines) {
            enabledFilters.push('- "deadlines": array of {text, date?, owner?} for deadlines mentioned');
        }
        if (this.filters.actionItems) {
            enabledFilters.push('- "actionItems": array of {text, assignee?, dueDate?} for action items');
        }
        if (this.filters.risks) {
            enabledFilters.push('- "risks": array of {text, severity: "low"|"medium"|"high"} for risks/concerns');
        }
        if (this.filters.urgency) {
            enabledFilters.push('- "urgency": array of {text, level: "low"|"medium"|"high"|"critical"} for urgent items');
        }
        if (this.filters.sentiment) {
            enabledFilters.push('- "sentiment": {overall: string, shifts: string[]} describing overall tone and emotional shifts');
        }

        if (enabledFilters.length === 0) {
            enabledFilters.push('- "keyTopics": array of main discussion topics');
        }

        // Always include speakers
        enabledFilters.push('- "speakers": array of {id, name, turns} for each speaker detected (use "Speaker 1", "Speaker 2", etc.)');

        return `Analyze this meeting transcript and extract structured insights.

TRANSCRIPT:
${transcriptText}

EXTRACT THE FOLLOWING (return valid JSON only, no markdown):
{
${enabledFilters.join(',\n')}
}

RULES:
- Be concise but specific
- Include actual names/dates if mentioned
- For speakers, try to identify distinct voices based on context
- If nothing found for a category, return empty array
- Return ONLY valid JSON, no explanation`;
    }

    /**
     * Extract insights from a transcript chunk
     */
    async extractFromTranscript(transcripts: TranscriptWithSpeaker[]): Promise<ExtractedInsights | null> {
        if (transcripts.length === 0) {
            return null;
        }

        // Check if any filters are enabled
        const hasEnabledFilters = Object.values(this.filters).some(v => v);
        if (!hasEnabledFilters) {
            console.log('[Intelligence] No filters enabled, skipping extraction');
            return null;
        }

        this.isProcessing = true;

        const transcriptText = transcripts
            .map(t => `[${t.timestamp}] ${t.speaker}: ${t.text}`)
            .join('\n');

        const prompt = this.buildExtractionPrompt(transcriptText);

        try {
            const apiKey = keyManager.getCurrentKey()?.key;
            if (!apiKey) {
                throw new Error('No API key available');
            }

            const response = await fetch(`https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent?key=${apiKey}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    contents: [{ parts: [{ text: prompt }] }],
                    generationConfig: { temperature: 0.2 }
                })
            });

            if (!response.ok) {
                if (response.status === 429) {
                    keyManager.handleError(429, 'Rate limit');
                }
                throw new Error(`API Error: ${response.status}`);
            }

            const data = await response.json();
            const text = data.candidates?.[0]?.content?.parts?.[0]?.text || '';

            // Parse JSON from response
            const jsonMatch = text.match(/\{[\s\S]*\}/);
            if (jsonMatch) {
                const extracted = JSON.parse(jsonMatch[0]);

                // Merge with existing insights
                this.mergeInsights(extracted);
                keyManager.reportSuccess();

                console.log('[Intelligence] Extracted:', extracted);
                return this.insights;
            }
        } catch (error: any) {
            console.error('[Intelligence] Extraction failed:', error.message);
        } finally {
            this.isProcessing = false;
        }

        return null;
    }

    private mergeInsights(newInsights: Partial<ExtractedInsights>) {
        // Append new items to existing arrays (avoid duplicates)
        if (newInsights.tasks) {
            this.insights.tasks = [...this.insights.tasks, ...newInsights.tasks];
        }
        if (newInsights.decisions) {
            this.insights.decisions = [...this.insights.decisions, ...newInsights.decisions];
        }
        if (newInsights.deadlines) {
            this.insights.deadlines = [...this.insights.deadlines, ...newInsights.deadlines];
        }
        if (newInsights.actionItems) {
            this.insights.actionItems = [...this.insights.actionItems, ...newInsights.actionItems];
        }
        if (newInsights.risks) {
            this.insights.risks = [...this.insights.risks, ...newInsights.risks];
        }
        if (newInsights.urgency) {
            this.insights.urgency = [...this.insights.urgency, ...newInsights.urgency];
        }
        if (newInsights.sentiment) {
            this.insights.sentiment = newInsights.sentiment;
        }
        if (newInsights.keyTopics) {
            const existingTopics = new Set(this.insights.keyTopics);
            newInsights.keyTopics.forEach(t => existingTopics.add(t));
            this.insights.keyTopics = Array.from(existingTopics);
        }
        if (newInsights.speakers) {
            this.insights.speakers = newInsights.speakers;
        }

        this.notifyListeners();
    }

    subscribe(listener: (insights: ExtractedInsights) => void): () => void {
        this.listeners.add(listener);
        listener(this.insights);
        return () => this.listeners.delete(listener);
    }

    private notifyListeners() {
        this.listeners.forEach(l => l({ ...this.insights }));
    }

    isExtracting(): boolean {
        return this.isProcessing;
    }
}

// Singleton
export const intelligenceExtractor = new IntelligenceExtractor();
