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


## Planning Document Standard
- Long-term execution must follow `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` as the single source of truth for product goals, phase order, architecture direction, red lines, and acceptance criteria.
- Code chapter history must be recorded in `docs/IMPLEMENTATION_CHAPTERS.md` only. Each chapter should list goals, added files, modified files, deleted files, explicit non-goals, validation status, and the next recommended chapter.
- Keep the long-term documentation set intentionally small: `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md` for direction and `docs/IMPLEMENTATION_CHAPTERS.md` for execution history. Do not recreate separate OCR, recording, model, release, or UI plan documents unless the master plan explicitly indexes and justifies them first.
- Do not create scattered long-term planning documents by default. Merge product plans, OCR runtime specs, model-pack specs, i18n/control-panel specs, and release checklists into the master plan unless a separate document is absolutely necessary.
- If a new long-term document becomes necessary, first add its purpose and relationship to the master plan inside `docs/COMMERCIAL_CLOSED_LOOP_MASTER_PLAN.md`.
- When resuming unattended work, read the master plan first, then continue from the latest chapter in `docs/IMPLEMENTATION_CHAPTERS.md`.

## Unattended Execution Standard
- Treat the project as a long-running commercial build: continue chapter-by-chapter from the master plan without waiting for micro-confirmations when the next step is clear.
- At the start of each work session, inspect `git status`, read the current master-plan priorities, and continue from the last completed chapter in `docs/IMPLEMENTATION_CHAPTERS.md`.
- At the end of each completed chapter, update `docs/IMPLEMENTATION_CHAPTERS.md` with actual changes, validation results, known risks, and the next recommended chapter.
- Do not mark strategic systems as ready until they pass real end-to-end behavior. In particular, do not set OCR runtime readiness to true until integrated ONNX inference, decode, postprocess, and self-test all work.
- Do not commit, push, create branches, or tag releases unless the user explicitly asks for that action in the current context.

## Code Organization Standard
- Prefer one focused component, hook, service, utility, or domain module per file instead of piling multiple responsibilities into a large file.
- Prefer every project feature, UI component, page section, dialog, toolbar, panel, hook, service, data adapter, and domain model to live in its own focused file when it has independent responsibility.
- Avoid creating or extending thousand-line files. If a file is growing toward broad responsibility, split it early by feature, subcomponent, hook, command, service, or data model.
- Treat large files as a design smell. Do not wait until a file reaches thousands of lines before splitting; proactively extract when a file mixes layout, state orchestration, API calls, rendering details, and helper logic.
- UI pages should compose smaller components rather than containing all cards, forms, modals, state logic, and helpers inline.
- Page files should mainly orchestrate layout and data flow. Complex cards, panels, toolbars, result windows, menus, and form sections should be separate components.
- Keep business logic out of presentation components when practical; move reusable logic into hooks, services, or utilities.
- Keep constants, type definitions, dictionary data, model manifests, and command adapters in separate files when they are reused or likely to grow.
- Keep platform/backend commands grouped by domain modules where possible instead of placing unrelated commands in one monolithic file.
- New features should start with a scalable folder structure so future work does not require a large split/refactor just to continue development.
- If touching an already large file, prefer extracting a coherent piece as part of the change when it reduces future maintenance risk and does not introduce unrelated behavior changes.
- Refactors should preserve behavior while improving boundaries. Avoid creating one-off extracted files that simply move clutter; each extracted module should have a clear name, purpose, and ownership.

## Implementation Standard
- Fix root causes rather than layering fragile patches.
- Keep code modular enough to support replacement of models, runtimes, and providers.
- Avoid hardcoding one-off behavior that blocks future languages or model packs.
- Validate with builds/tests where available before handing off.
- When changing existing code, preserve unrelated behavior and keep changes focused.

## Quality Bar
- Features should be evaluated for correctness, speed, resource usage, offline capability, update strategy, error recovery, and visual polish.
- If a commercial-grade solution requires deeper architecture work, prefer planning and implementing that path over a quick but limiting workaround.
