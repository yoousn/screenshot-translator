import type { OcrBlock } from "../types/screenshot";
import { normalizeForCompare } from "../ocr-processing";
import {
  shouldRequireTranslation,
  isChineseTargetLanguage,
  type TranslationRequirementOptions,
} from "./ocrTranslationRequest";
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
let cachedStoreRaw: string | null = null;
let cachedStore: TranslationMemoryStore | null = null;
let cachedEntryMap: Map<string, TranslationMemoryEntry> | null = null;

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
    if (!raw) {
      cachedStoreRaw = null;
      cachedStore = null;
      cachedEntryMap = null;
      return { version: MEMORY_VERSION, entries: [] };
    }
    if (cachedStore && cachedStoreRaw === raw) return cachedStore;
    const parsed = JSON.parse(raw) as TranslationMemoryStore;
    if (parsed.version !== MEMORY_VERSION || !Array.isArray(parsed.entries)) {
      return { version: MEMORY_VERSION, entries: [] };
    }
    const cutoff = Date.now() - TTL_MS;
    const store = {
      version: MEMORY_VERSION,
      entries: parsed.entries.filter((entry) => entry.updatedAt >= cutoff && entry.key && entry.translatedText),
    };
    cachedStoreRaw = raw;
    cachedStore = store;
    cachedEntryMap = null;
    return store;
  } catch {
    return { version: MEMORY_VERSION, entries: [] };
  }
};

const writeStore = (store: TranslationMemoryStore) => {
  if (!canUseBrowserStorage()) return;
  const entries = [...store.entries]
    .sort((a, b) => b.lastUsedAt - a.lastUsedAt)
    .slice(0, MAX_ENTRIES);
  const nextStore = { version: MEMORY_VERSION, entries };
  const serialized = JSON.stringify(nextStore);
  try {
    window.localStorage.setItem(STORAGE_KEY, serialized);
    cachedStoreRaw = serialized;
    cachedStore = nextStore;
    cachedEntryMap = null;
  } catch {
    window.localStorage.removeItem(STORAGE_KEY);
    cachedStoreRaw = null;
    cachedStore = null;
    cachedEntryMap = null;
  }
};

const getEntryMap = (store: TranslationMemoryStore) => {
  if (!cachedEntryMap || cachedStore !== store) {
    cachedEntryMap = new Map(store.entries.map((entry) => [entry.key, entry]));
  }
  return cachedEntryMap;
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
  options: TranslationRequirementOptions = {},
): LocalTranslationHit | null => (
  lookupLocalTranslations([block], sourceLang, targetLang, channel, options)[0] || null
);

export const lookupLocalTranslations = (
  blocks: OcrBlock[],
  sourceLang: string,
  targetLang: string,
  channel?: string,
  options: TranslationRequirementOptions = {},
): Array<LocalTranslationHit | null> => {
  const hits: Array<LocalTranslationHit | null> = [];
  let store: TranslationMemoryStore | null = null;
  let entryMap: Map<string, TranslationMemoryEntry> | null = null;
  let touchedMemory = false;
  const now = Date.now();

  for (const block of blocks) {
    const text = block.text || "";
    if (!shouldRequireTranslation(text, targetLang, options)) {
      hits.push({ translation: text, source: "preserved" });
      continue;
    }

    if (isChineseTargetLanguage(targetLang)) {
      const glossaryHit = zhGlossary.get(normalizeMemoryText(text));
      if (glossaryHit) {
        hits.push({ translation: glossaryHit, source: "glossary" });
        continue;
      }
    }

    if (!store) {
      store = readStore();
      entryMap = getEntryMap(store);
    }

    const key = makeMemoryKey(text, sourceLang, targetLang, channel);
    const entry = entryMap?.get(key);
    if (!entry) {
      hits.push(null);
      continue;
    }

    entry.lastUsedAt = now;
    entry.hits += 1;
    touchedMemory = true;
    hits.push({ translation: entry.translatedText, source: "memory" });
  }

  if (store && touchedMemory) {
    writeStore(store);
  }
  return hits;
};

export const storeTranslationMemory = (
  blocks: OcrBlock[],
  translations: string[],
  sourceLang: string,
  targetLang: string,
  channel?: string,
  options: TranslationRequirementOptions = {},
) => {
  const now = Date.now();
  const store = readStore();
  const byKey = new Map(store.entries.map((entry) => [entry.key, entry]));
  let stored = 0;

  blocks.forEach((block, index) => {
    const translatedText = translations[index]?.trim() || "";
    if (!translatedText || !shouldRequireTranslation(block.text, targetLang, options)) return;
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
  cachedStoreRaw = null;
  cachedStore = null;
  cachedEntryMap = null;
};
