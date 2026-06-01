# AGENTS.md

## Product Standard
- This project is a commercial-grade desktop productivity product, not a personal quick prototype.
- Every feature should be designed against paid-app expectations: reliability, performance, polish, maintainability, upgradeability, and clear user recovery paths.
- Do not choose shortcuts only because they are faster to land if they limit product quality, extensibility, or ownership.

## Architecture Principles
- Prefer owned, integrated runtime architecture over opaque external executables when the feature is strategic.
- OCR is strategic. The long-term direction is `YSN OCR Runtime`: integrated ONNX Runtime inference, managed model packs, automatic language/script routing, confidence scoring, retry/fallback paths, and translation-aware text cleanup.
- External OCR executables may remain only as compatibility or temporary fallback paths, not as the product's main architecture.
- FFmpeg may remain an external dependency unless/until video encoding becomes a strategic owned runtime.

## OCR And Translation Requirements
- Source language selection must be automatic. Users should not need to choose source OCR language manually.
- Target language is user-selectable and defaults to Simplified Chinese.
- OCR must be designed for multilingual support including Simplified Chinese, Traditional Chinese, English, French, Japanese, German, Spanish, Portuguese, Italian, Korean, Russian, Arabic, Thai, Turkish, and future languages.
- Use a multi-model OCR pool instead of assuming one recognition model covers every script well.
- Route OCR by script/language detection, recognition confidence, and retry scoring.
- Preserve technical identifiers during OCR cleanup and translation, including paths, commands, flags, package names, `PATH`, `Windows`, `OCR`, `ONNX`, `RapidOCR`, `PaddleOCR-json`, and `.exe`.
- Low-confidence OCR should have a deliberate fallback path, potentially including a stronger OCR model or VLM OCR fallback.

## UI / UX Standard
- UI must feel modern, compact, and product-grade.
- Avoid large, cluttered configuration panels. Prefer status cards, primary actions, progressive disclosure, and advanced sections.
- Model/configuration UX should hide implementation complexity from ordinary users while keeping advanced controls available.
- Critical flows must provide clear status, error messages, and next actions.

## Implementation Standard
- Fix root causes rather than layering fragile patches.
- Keep code modular enough to support replacement of models, runtimes, and providers.
- Avoid hardcoding one-off behavior that blocks future languages or model packs.
- Validate with builds/tests where available before handing off.
- When changing existing code, preserve unrelated behavior and keep changes focused.

## Quality Bar
- Features should be evaluated for correctness, speed, resource usage, offline capability, update strategy, error recovery, and visual polish.
- If a commercial-grade solution requires deeper architecture work, prefer planning and implementing that path over a quick but limiting workaround.
