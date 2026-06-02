import type { OcrBlock } from "../types/screenshot";
import { normalizeForCompare } from "../ocr-processing";
import { shouldRequireTranslation, isChineseTargetLanguage } from "./ocrTranslationRequest";
import translationGlossary from "./translationGlossary.json";

const STORAGE_KEY = "ysn.translationMemory.v1";
const MEMORY_VERSION = "1";
const MAX_ENTRIES = 1000;
const TTL_MS = 30 * 24 * 60 * 60 * 1000;

type TranslationMemoryEntry = {
  key: string;
  sourceText: string;
  translatedText: string;
  sourceLang: string;
  targetLang: string;
  channel: string;
  updatedAt: number;
  lastUsedAt: number;
  hits: number;
};

type TranslationMemoryStore = {
  version: string;
  entries: TranslationMemoryEntry[];
};

export type LocalTranslationHit = {
  translation: string;
  source: "preserved" | "glossary" | "memory";
};

export type TranslationMemoryStats = {
  preservedHits: number;
  glossaryHits: number;
  memoryHits: number;
  stored: number;
  requestedBlocks: number;
  deduplicatedBlocks: number;
};

export type TranslationMemoryStorageStats = {
  entries: number;
  maxEntries: number;
  ttlDays: number;
};

const zhGlossary = new Map(Object.entries(translationGlossary.zh.ui));

const canUseBrowserStorage = () => typeof window !== "undefined" && Boolean(window.localStorage);

const normalizeMemoryText = (text: string) => text.replace(/\s+/g, " ").trim().toLowerCase();

const normalizeChannel = (channel?: string) => channel || "auto";

const makeMemoryKey = (text: string, sourceLang: string, targetLang: string, channel?: string) => (
  [MEMORY_VERSION, normalizeMemoryText(text), sourceLang || "auto", targetLang || "zh", normalizeChannel(channel)].join("|")
);

const readStore = (): TranslationMemoryStore => {
  if (!canUseBrowserStorage()) return { version: MEMORY_VERSION, entries: [] };
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return { version: MEMORY_VERSION, entries: [] };
    const parsed = JSON.parse(raw) as TranslationMemoryStore;
    if (parsed.version !== MEMORY_VERSION || !Array.isArray(parsed.entries)) {
      return { version: MEMORY_VERSION, entries: [] };
    }
    const cutoff = Date.now() - TTL_MS;
    return {
      version: MEMORY_VERSION,
      entries: parsed.entries.filter((entry) => entry.updatedAt >= cutoff && entry.key && entry.translatedText),
    };
  } catch {
    return { version: MEMORY_VERSION, entries: [] };
  }
};

const writeStore = (store: TranslationMemoryStore) => {
  if (!canUseBrowserStorage()) return;
  const entries = [...store.entries]
    .sort((a, b) => b.lastUsedAt - a.lastUsedAt)
    .slice(0, MAX_ENTRIES);
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: MEMORY_VERSION, entries }));
  } catch {
    window.localStorage.removeItem(STORAGE_KEY);
  }
};

export const createTranslationMemoryStats = (): TranslationMemoryStats => ({
  preservedHits: 0,
  glossaryHits: 0,
  memoryHits: 0,
  stored: 0,
  requestedBlocks: 0,
  deduplicatedBlocks: 0,
});

export const lookupLocalTranslation = (
  block: OcrBlock,
  sourceLang: string,
  targetLang: string,
  channel?: string,
): LocalTranslationHit | null => {
  const text = block.text || "";
  if (!shouldRequireTranslation(text, targetLang)) {
    return { translation: text, source: "preserved" };
  }

  if (isChineseTargetLanguage(targetLang)) {
    const glossaryHit = zhGlossary.get(normalizeMemoryText(text));
    if (glossaryHit) {
      return { translation: glossaryHit, source: "glossary" };
    }
  }

  const key = makeMemoryKey(text, sourceLang, targetLang, channel);
  const store = readStore();
  const entry = store.entries.find((item) => item.key === key);
  if (!entry) return null;

  entry.lastUsedAt = Date.now();
  entry.hits += 1;
  writeStore(store);
  return { translation: entry.translatedText, source: "memory" };
};

export const storeTranslationMemory = (
  blocks: OcrBlock[],
  translations: string[],
  sourceLang: string,
  targetLang: string,
  channel?: string,
) => {
  const now = Date.now();
  const store = readStore();
  const byKey = new Map(store.entries.map((entry) => [entry.key, entry]));
  let stored = 0;

  blocks.forEach((block, index) => {
    const translatedText = translations[index]?.trim() || "";
    if (!translatedText || !shouldRequireTranslation(block.text, targetLang)) return;
    if (normalizeForCompare(translatedText) === normalizeForCompare(block.text)) return;

    const key = makeMemoryKey(block.text, sourceLang, targetLang, channel);
    const existing = byKey.get(key);
    byKey.set(key, {
      key,
      sourceText: block.text,
      translatedText,
      sourceLang,
      targetLang,
      channel: normalizeChannel(channel),
      updatedAt: now,
      lastUsedAt: now,
      hits: existing ? existing.hits : 0,
    });
    stored += 1;
  });

  if (stored > 0) {
    writeStore({ version: MEMORY_VERSION, entries: Array.from(byKey.values()) });
  }
  return stored;
};

export const getTranslationMemoryStorageStats = (): TranslationMemoryStorageStats => ({
  entries: readStore().entries.length,
  maxEntries: MAX_ENTRIES,
  ttlDays: Math.round(TTL_MS / (24 * 60 * 60 * 1000)),
});

export const clearTranslationMemory = () => {
  if (!canUseBrowserStorage()) return;
  window.localStorage.removeItem(STORAGE_KEY);
};
