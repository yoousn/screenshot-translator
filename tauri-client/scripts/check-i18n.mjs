import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const LANGUAGES = ["zh-CN", "en-US"];
const DAMAGED_PATTERNS = [
  /\?{3,}/,
  /\uFFFD/,
  /[鎴缈姝澶璇鍥褰榛涓瀹鐩鈥锛銆]/,
];
const CJK_PATTERN = /[\u3400-\u4dbf\u4e00-\u9fff]/;

const fail = (message) => {
  console.error(`[i18n] ${message}`);
  process.exitCode = 1;
};

const findLanguageBlock = (language) => {
  const filename = resolve(root, `src/i18n/${language}.ts`);
  const content = readFileSync(filename, "utf8");
  const marker = " = {";
  const start = content.indexOf(marker);
  if (start < 0) throw new Error(`Missing dictionary language block in: ${filename}`);
  let depth = 0;
  let end = -1;
  for (let index = start + marker.length - 1; index < content.length; index += 1) {
    const char = content[index];
    if (char === "{") depth += 1;
    if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        end = index + 1;
        break;
      }
    }
  }
  if (end < 0) throw new Error(`Unclosed dictionary language block in: ${filename}`);
  return content.slice(start + marker.length - 1, end);
};

const parseLanguage = (language) => {
  const block = findLanguageBlock(language);
  const stack = [];
  const keys = new Set();
  const values = [];
  const lines = block.split(/\r?\n/);

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line || line.startsWith(`"${language}"`)) continue;
    if (line === "}," || line === "}") {
      stack.pop();
      continue;
    }

    const objectMatch = line.match(/^([A-Za-z0-9_]+):\s*\{$/);
    if (objectMatch) {
      stack.push(objectMatch[1]);
      continue;
    }

    const valueMatch = line.match(/^([A-Za-z0-9_]+):\s*"((?:\\.|[^"])*)",?$/);
    if (valueMatch) {
      const path = [...stack, valueMatch[1]].join(".");
      keys.add(path);
      values.push({ path, value: valueMatch[2] });
    }
  }

  return { keys, values };
};

const parsed = new Map(LANGUAGES.map((language) => [language, parseLanguage(language)]));
const zh = parsed.get("zh-CN");
const en = parsed.get("en-US");

for (const key of zh.keys) {
  if (!en.keys.has(key)) fail(`en-US is missing key: ${key}`);
}
for (const key of en.keys) {
  if (!zh.keys.has(key)) fail(`zh-CN is missing key: ${key}`);
}

for (const [language, data] of parsed) {
  for (const { path, value } of data.values) {
    for (const pattern of DAMAGED_PATTERNS) {
      if (pattern.test(value)) fail(`${language}.${path} contains damaged text: ${value}`);
    }
    if (language === "en-US" && CJK_PATTERN.test(value)) {
      fail(`${language}.${path} contains CJK text: ${value}`);
    }
  }
}

if (!process.exitCode) {
  console.log(`[i18n] OK: ${zh.keys.size} zh-CN keys match ${en.keys.size} en-US keys`);
}
