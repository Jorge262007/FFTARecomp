<a id="lingua-del-menu-desktop"></a>

# Desktop menu language

The menu starts in **English**, independently of the system and ROM language.
Use **Game > System > Language** to choose **English** or **Italiano**.
With Italian selected, the same route is **Gioco > Sistema > Lingua**.

Confirm opens the list; arrows select a language and a second confirmation
applies the choice immediately. Cancel closes the list without changing the
preference. The mouse uses the same list and confirmation path.
The normal settings handler saves the choice in `av.cfg`:

```ini
menu.language = en
```

`it` selects Italian. An older configuration without this key, an empty key
or an unsupported language uses English. Other preferences are retained.
In portable mode, the key lives in the same `Data/config/av.cfg` as the other
options. This preference controls the desktop interface: it does not change
game text or ROM data. `AndroidPort/` is a separate tree and does not receive
this implementation automatically.

The catalog covers the shell, navigation, titles, option labels/choices,
keyboard/controller instructions, save and confirmation forms, details and
selector/tool controls. The standard-save timestamp and last undoable-operation
description retain their data and redraw in the current language. Dynamic
messages from normal save operations use the language selected when produced.

File names and paths, bindings, configuration tokens, command syntax, entered
values, imported descriptions, technical references and diagnostic payloads
retain their original text. Results of operations already performed are not
translated retroactively. Translating a choice's presentation does not change
its stored value: for example, Italian **INTERNO** still corresponds to
`video.fps = internal`.

<a id="file-delle-lingue-e-aggiunta-di-una-lingua"></a>

## Language files and adding a language

Each language is **one JSON file** in
[`crates/input-config/languages/`](../crates/input-config/languages/):

- [`en.json`](../crates/input-config/languages/en.json): base English catalog.
- [`it.json`](../crates/input-config/languages/it.json): Italian translations.

Each file contains `code` (the token saved in settings), `name` (the native
language name shown in the selector) and `messages` (keys and their text).
Its filename matches the code: `fr.json`, `pt-br.json`, and so on.

For example, to add French:

1. Create the file by automatically copying all English entries:

   ```sh
   python3 scripts/menu_language.py create fr --name "Français"
   ```

2. Open `crates/input-config/languages/fr.json` and translate only the values
   in `messages`. Keep keys and placeholders such as `{slot}`, `{value}` and
   `{0}` unchanged. Save the JSON as UTF-8. Some existing keys come from older
   Italian text: they are stable identifiers and must not be translated.

3. Check the files and rebuild using the project's normal build procedure:

   ```sh
   python3 scripts/menu_language.py check
   cargo build --locked --release -p recomp -p ffta-play
   ```

**No Rust edits, language lists or manual registration are needed.**
The build automatically discovers every `languages/*.json`, validates the
files and embeds the registry in the program. The new language appears in
settings after rebuilding. Distributed executables already contain their
translations: Python and external JSON files are not needed at runtime.

You can also add JSON files manually by copying `en.json` and changing
`code`, `name` and the text. `create` is a convenience command and refuses
to overwrite existing files. Both the checker and the build reject malformed
JSON, duplicate/unknown keys, duplicate names, empty translations and
placeholders that differ from English, identifying the file to correct.

A language can be partial: missing keys use the text from `en.json`.
Adding new English entries does not require completing every other language
immediately. If the English key is also missing, the renderer shows the source
key. Values identical to English are allowed, such as Audio, Video or ROM.

`menu::i18n::format` substitutes placeholders without reinterpreting inserted
values. Renderers resolve translations before measurement or wrapping;
the language belongs to each menu instance.

The `menu --example preview` example accepts any embedded code as its last
argument, after the optional background path (an empty string omits it).
The `system` and `language` pages show the selector without loading ROMs or saves.

<a id="verifica"></a>

## Validation

Focused tests cover defaults and older configurations, roundtrips for both
languages, cancel/confirm behavior, mouse selection, immediate switching,
reopening and preservation of technical tokens. Observed rendering covers
both languages at 480×640, 854×480 and 1280×720, controller hints, timestamps
and retained Undo descriptions. The build and checker validate unique keys
and matching placeholders; inserted values containing braces remain literal.

Windows GNU/Wine checks, previews without a ROM and the `recomp`/`ffta-play`
compilation check are recorded in the 2026-11-01 change records. These do not
establish native Windows or physical-controller coverage. The earlier,
separate `ContentSlot::ALL` / `Page::ToolInputs` mismatch was subsequently
fixed by [exposing the cheat catalog in tool inputs](changes/2026-11-01-expose-cheat-catalog-in-tool-inputs.json).

The generated registry is also tested with a third synthetic language in a
temporary directory, compiling the real Language implementation without
editing Rust or adding test languages to the distribution. `create`/`check`
tests cover existing files, Unicode, partial translations and refusal of
invalid metadata, keys and placeholders. The English and Italian catalogs
were migrated while preserving all 378 previous entries exactly.
