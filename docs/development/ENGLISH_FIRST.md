# English-first project

English is the primary language of FFTARecomp, in both the root workspace and
the standalone `AndroidPort/` tree. This applies to maintained documentation,
source comments, default user interfaces, command help, prompts, errors,
diagnostics, report templates and development tools. Optional translations
remain supported; English is the default and fallback.

## Authoring and translation

Write new project material in English. Use English for new message keys and
identifiers. Add translated UI text to explicit locale catalogs, rather than
embedding another language as an untranslated default. Preserve existing
persisted codes, command syntax and public paths unless a compatible migration
is implemented and verified.

Translate maintained guides in place so their primary URLs lead to English.
Preserve commands, IDs, links, numbers, source references and validation limits.
Keep legacy heading anchors when needed for existing links. Translating a
validation report must not change its pass/fail state or turn a conditional
result into a broader acceptance claim.

## Material preserved verbatim

Upstream license text, proper names, original game/ROM text, imported user data,
literal test fixtures and immutable historical snapshots retain their original
content. English descriptions should explain their role. Optional locale files
are intentionally written in their target language. Existing machine tokens
and legacy compatibility anchors may retain their spelling.

These exceptions protect data and provenance. They do not exempt current
documentation, live report labels, console instructions or default UI from
English-first authoring. The immutable legacy handoff snapshot must remain
byte-identical; the maintained handoff and its forward publication format are
part of the migration.

## Project-wide migration

The active task is `ENGLISH-FIRST-PROJECT` in
[TASK_HANDOFF.md](TASK_HANDOFF.md). Its scope includes every maintained guide,
development reference and default program/tooling surface across both trees.
The migration is not complete merely because the main README or recent guides
are English. Inventory searches identify candidates; they are not proof that
all remaining natural-language text has been reviewed.

Completion requires an audited inventory, English primary documentation,
English default product/tooling content, a compatible English-first memory
workflow, preserved optional translations/provenance and relevant checks.
No ROM exploration, dataset rebuild or benchmark is needed solely to translate
documentation. Existing accepted evidence remains valid within its original scope.
