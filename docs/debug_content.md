<a id="catalogo-comune-dei-contenuti-debug"></a>

# Shared debug content catalog

DBG2-02 introduces a readonly service shared by menu and console. The CLI
explicitly refreshes the index and rechecks a selected item. The
[DBG2-03 visual selector](debug_selector.md) reuses the service; DBG2-04 covers
migration of existing actions. Indexing does not execute files, import saves
or repair datasets.

```sh
recomp debug-content scan --root PORTABLE_DIRECTORY --json
recomp debug-content scan --add-root EXPLICIT_DIRECTORY --json
recomp debug-content scan --add-file EXPLICIT_FILE --json
recomp debug-content scan --root PORTABLE_DIRECTORY --rom-sha256 ROM_HASH --json
recomp debug-content check SAVED_INDEX.json ID --kind replay-profile --json
```

`--root` declares an app directory: it reads recognized files directly in the
root and the `Data`, `scripts`, `cheats`, `reports` trees. Within `Data`, allowed
directories are `debug-sessions`, `config`, `saves`, `cache`, `resources`, `dataset`.
`--add-root` adds an explicitly chosen directory, allowing bounded traversal
outside those names. It can be repeated within eight total roots. When they are
the only root options, `--add-root` and `--add-file` inspect only explicit
references. `--add-file` adds one file without enumerating its directory or
neighbors; the optional `file` field preserves readability of older schema 1
roots without it.

Without root options, scanning uses the portable directory when its marker is
present; otherwise only declared app locations: executable directory,
configuration, actual resources and cache. The resource resolver preserves
`FFTA_RESOURCES_DIR`. There is no disk-wide search or refresh at game startup.
Missing directories produce index reasons without creating directories or
opening dialogs.

Items include sessions, recordings, `.input` sequences, replay profiles, recipes,
host tools, guest-script catalogs, campaigns, snapshots, runtimes, datasets,
native libraries (including `native.bin`), labels (including `debug-labels.toml`),
known manifests, save/cheat catalogs and reports. Label v1/v2 is not confused
with schema 1 catalog metadata; the typed role checks labels with their parser
before use. Recognition uses names, extensions and declared metadata. An `.exe`
listed as a runtime has not been loaded or declared compatible; a cheat catalog
has not been decoded. Host scripts and guest-script data retain distinct types
without an inferred executable adapter.

Each item retains ID, kind, name, description, root/relative path, selection path,
size/hash where available, declared ROM/region, presence, compatibility,
verification level and reason. ID depends on logical root, kind and path,
separately from bytes: two same-named items retain different identities, and an
altered selection can be recognized as stale. A portable root preserves IDs
after moving the whole directory. External roots remain distinct explicit
references; DBG2-07 covers persisted choices and reference migration.

Equal hashes produce duplicate links without merging IDs or starting content.
Each item shows the total other duplicates and up to eight linked IDs. Duplicate
root declarations are reported and scanned once. Symbolic links, junctions,
special files and hard-linked files are refused using isolated-session checks;
linked directories are not traversed.

`scan` returns `schema: 1`, `kind: ffta-debug-content-index`, roots, items,
problems, counters and limits. Missing hashes/metadata are not converted into
successful verification. `compatibility` distinguishes `unknown`,
`declared-match`, `incompatible` and `unsupported`; `declared-match` compares only
the **declared** ROM hash with `--rom-sha256`. Pins, payloads, formats, features,
ABI and backends always require handler validation before use.

`check` reads an explicitly saved index, finds the exact ID and rechecks only
the selected file. Changed, missing, nonordinary, unreadable and incompatible-kind
files remain distinct. Parent traversal `..`, inconsistent roots and unsupported
versions are refused. Without a hash in the original selection, it does not
promise unchanged bytes. Exit 0 means only item listed/rechecked; exit 3 means
refusal; exit 1 means invalid query/index. It never starts the selected action.

<a id="limiti-e-verifica"></a>

## Limits and validation

A scan retains at most 512 items, visits 8,192 nodes within 12 levels and hashes
64 MiB total. Structured metadata is limited to 1 MiB per file and 8 MiB total;
hashing is chunked, without retaining complete runtimes/snapshots in memory.
An index to recheck is limited to 8 MiB. Up to 64 problems are retained, with
a counter for additional drops. `.git`, `.retention`, `target` and `node_modules`
directories encountered within a root are not traversed. Such a directory can
still be an explicitly added root chosen by the user.

Saturated traversal marks `truncated`: unvisited content remains unknown.
Hash/metadata limits are explained per item; the list can be complete without
reading every payload. A new query is an explicit refresh, not automatic
verification or execution.

Checks also compare each canonical component with its parent: Wine may hide
host-filesystem links in ordinary Windows attributes. Wine fixtures use actual
host links; successful creation through Windows APIs alone is not accepted as
proof of the fixture.

Public synthetic fixtures cover empty lists, same-named items, duplicates,
portable moves, recognized kinds, scripts never executed, invalid metadata,
future versions, changed/missing/wrong-kind files, duplicate/missing roots,
links, hard links, limits and malformed queries. No ROMs, personal saves or
reruns of closed DBG checks are needed. Actual outcomes and platform limits
remain in the record linked from the
[DBG2 TODO](development/DEBUG_TOOLS_V2_TODO.md).
