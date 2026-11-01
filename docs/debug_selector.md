# Selettore integrato dei contenuti debug

La scheda richiede lo [sblocco persistente di Debug](menu_clan_atlas.md#sblocco-permanente-di-debug):
**↑ ↑ ↓ ↓ ← ← → →** nel menu, oppure `debug = true` in `av.cfg` e riavvio.

DBG2-03 aggiunge **Debug > Content selector**, una pagina riutilizzabile sopra
il [catalogo comune](debug_content.md). La scelta confermata è un riferimento
host: nessun replay, script, caricamento di stato o cheat parte selezionandola.
La migrazione DBG2-04 è in corso; le prove concluse rimangono distinte dalla nuova integrazione.

Aprire la pagina mostra l'indice già disponibile oppure una lista vuota.
**Refresh** aggiorna esplicitamente le radici dichiarate in un worker limitato;
la pagina resta navigabile. **Browse external file…** aggiunge soltanto il file
scelto, senza esaminare i suoi vicini. **Add folder…** aggiunge una cartella
esplicita alle radici della scansione. Solo questi due pulsanti aprono dialoghi
OS; annullarli conserva radici e scelta corrente. Al ritorno il frontend
consuma gli eventi della chiusura e del ripristino del focus per almeno 400 ms e
due poll neutri del controller, evitando che Escape chiuda anche il menu o che
una conferma mantenuta avvii altre azioni. `host_modal_input` nello status indica
questa breve attesa. La CLI equivalente per
indicizzare un singolo file è `debug-content scan --add-file FILE --json`.

La lista offre ricerca per nome/ID/tipo/percorso/descrizione, categoria, filtri
di disponibilità e compatibilità, ordinamento per nome/tipo/recente e raccolte
All content/Favorites/Recent. Due omonimi restano distinti tramite ID e percorso
nei dettagli. L'evidenziazione cambia solo la riga osservata; **Select** conferma
la scelta dopo il ricontrollo del catalogo. File cambiati, mancanti, non ordinari,
di tipo sconosciuto o incompatibili vengono rifiutati. Un cambio di generazione
dell'indice impedisce di usare una riga precedente per un file diverso.

**Favorite** conserva al massimo 512 ID; Recent conserva le ultime 32 conferme.
Le scelte rimangono in memoria nella finestra corrente: persistenza e riferimenti
dopo riapertura sono DBG2-07. Il rientro dal selettore conserva il chiamante e
la scelta già confermata. Una selezione ancora in lavorazione viene scartata se
si annulla la pagina; un risultato tardivo non la riapre e non applica una scelta.

**Details / preview** rende scorrevoli ID completi, percorso, identità, stato,
motivo e messaggi. Per snapshot con immagine incorporata usa il verificatore
readonly già disponibile, senza caricare una Machine; snapshot raw non promettono
un'immagine. Il ricontrollo dell'hash precede la visualizzazione. La raccolta di
metadati e l'anteprima non sostituiscono il preflight dell'azione effettiva. Il
messaggio finale distingue anteprima autenticata, immagine assente ed errore;
una risposta relativa a una riga ormai diversa non sostituisce l'anteprima corrente.

Tastiera: frecce per spostare il focus, Tab per cambiare area, Invio per
entrare nella ricerca o portare la lista sul pulsante Select. Invio su Select
conferma; Escape annulla l'editor/dettaglio o ritorna al chiamante. Ctrl+F apre
la ricerca. Nella ricerca i caratteri, Backspace, Tab, Invio ed Escape vengono
consumati dal menu prima dei binding guest e delle shortcut debugger/globali.
L'editor accetta 128 caratteri Unicode e preedit/commit IME; la copertura effettiva
di layout/composizione e glifi dipende dalla piattaforma e dai font disponibili.

Mouse: hover evidenzia, click sui pulsanti esegue la singola azione host; click
sulla lista non conferma la scelta. La rotella scorre la lista o i dettagli.
Controller: direzioni, South/Confirm e East/Cancel percorrono gli stessi controlli;
le direzioni navigano fra i filtri, Confirm apre la scelta del valore e un
secondo Confirm la applica. Cancel scarta il valore in modifica. Il medesimo
comportamento vale per le scelte nei moduli dei parametri;
la ricerca offre una tastiera a schermo con pagine, spazio, cancellazione e Done.
Una pressione mantenuta non deve produrre conferme multiple. Mentre l'editor ha
focus, le shortcut non raggiungono il guest.

La pagina usa una sola operazione IO alla volta, con generazione dell'indice e
identità della vista. Mantiene i cap del catalogo; i dettagli avvolgono il testo
con gli stessi font, kerning e advance del renderer e limitano la finestra a
16.384 caratteri. `debug-control status` include lo stato host `content_picker`
per leggere conteggio, generazione, operazione pendente e scelta confermata.
Non è ancora un nuovo comando di esecuzione della console DBG2-13/15.

Le verifiche effettive, le piattaforme e gli eventuali criteri ancora aperti
sono registrati nel [TODO DBG2](development/DEBUG_TOOLS_V2_TODO.md) e nel
[passaggio di consegne](development/TASK_HANDOFF.md). La prova GUI usa una fixture
pubblica sintetica ferma al frame iniziale, distinta dalle scene DS e dai dati
personali. I tre vecchi fallback FileDialog sono rimossi da DBG2-04.

DBG2-03 è verificato con build Windows GNU e Wine: tutti i 55 casi menu,
10 catalogo, 4 bridge, 3 contratto, 2 collector UI e 2 ritorno modale nei
rispettivi scope; 3 test Python inventario e 10 packaging. La GUI esercita
ricerca, omonimi, conferma mantenuta singola, preferiti/recenti, XInput simulato,
annullamento di entrambi i dialoghi, importazione singola e anteprima a tre
dimensioni. La prima prova Cancel ha esposto e conservato una regressione che
riprendeva la copia sintetica; dopo la correzione la nuova prova conserva stato
completo/frame0/clock0. Windows nativo, Linux runtime, controller fisici e IME
nativo restano non verificati. [Record completo](changes/2026-10-29-add-integrated-debug-content-selector.json).

DBG2-04 aggiunge **Debug > Tool inputs / current selections**:
profili replay/audit/capture/compare, catalogo salvataggi, sessione, registrazione,
snapshot, sequenza input, dataset, runtime A/B, libreria nativa, label e piano
noto. Le liste sono vincolate al ruolo e le scelte conservano hash, ID e contesto
ROM. Un file non hashato nel budget viene rifiutato con richiesta di restringere
la radice e aggiornare. Un riferimento alterato resta visibile e viene rifiutato
prima dell'uso; non si ripiega su un altro file o sui dialoghi OS.
La stessa pagina include **Cheat reference catalog**, con il selettore interno
vincolato al catalogo cheat. Confermare il riferimento e annullare il selettore
mantiene il cursore sulla relativa riga; non applica né attiva codici.

Choose replay profile e Choose catalog aprono la pagina interna. Replay, Audit,
Capture, Compare e Create issue senza profilo aprono la stessa pagina con il
requisito mancante: scegliere non riprende l'azione automaticamente. Ritornare
al menu e avviare esplicitamente. I riferimenti passati dalla CLI e il catalogo
predefinito vengono autenticati prima dell'uso. Le label scelte alimentano la
traccia codice; gli input per preparare nuovi profili sono separati dai payload
immutabili dei profili già autenticati (editor DBG2-05).

La scelta tipizzata controlla formato e identità usando i servizi readonly
esistenti per profili, cataloghi, sessioni e registrazioni; snapshot e sequenze
sono controllati senza caricarli nella macchina corrente. Le label sono parse
per la ROM; i manifest controllano schema/ROM. Runtime e librerie restano
riferimenti hashati: il worker deve ancora verificare formato, feature e ABI,
senza caricare codice alla semplice selezione.

La verifica Wine esercita i dodici ruoli, i sette ingressi senza input, gli
avvii espliciti, dieci rifiuti di profili alterati/mancanti, ROM diversa e sei
rifiuti Choose/Open del catalogo. La traccia conferma un dialogo per pulsante
esplicito e Cancel conserva tutte le scelte. Il processo principale delle prove
accettate resta al frame zero; worker sintetici e un precedente errore di
navigazione dell'harness sono documentati separatamente nel
[record DBG2-04](changes/2026-10-29-migrate-debug-tool-input-selection.json).

DBG2-05 aggiunge **Debug > Parameters / prepare profiles** aggiunge bozze
per replay, audit, cattura, confronto, passi, ispezione, audio, codice, gameplay,
eventi recenti, grafica e issue. Le righe input usano il selettore tipizzato;
le frecce spostano il focus; Confirm apre la modifica della scelta o l'editor
testuale con tastiera a schermo. Una scelta cambia la bozza solo al secondo
Confirm; Escape scarta il valore ancora in modifica.
Escape annulla l'editor o ritorna dal pannello; Tab cambia campo/azione.

**Verify draft** usa i parser/preflight condivisi senza eseguire. **Save new
preset** conserva soltanto valori scalari nella configurazione portable:
`debug-presets/<tool>/`, massimo otto file da 16 KiB per strumento e 8 KiB di
valori. **Load last preset** ripristina una bozza, senza cambiare riferimenti o
avviare strumenti. Input mancanti/alterati e combinazioni non supportate hanno
un rifiuto leggibile. Le risposte tardive non sovrascrivono una bozza riaperta.

**Prepare NEW profile** crea una nuova copia in `debug-profiles/`, fino a otto,
con parametri e input autenticati; non sovrascrive un profilo esistente e non
avvia replay. **Run prepared profile** esegue quel profilo separatamente fissato:
modificare la bozza richiede una nuova preparazione. Negli strumenti locali,
**Start / apply** passa i parametri all'handler esistente; armare un observer non
avanza automaticamente il gioco. Passi, ripresa e report restano espliciti.
Le opzioni comprendono timeout/frame/input/range/filtri/stream/trigger/modalità
confronto/sintomo; le label selezionate restano riferimenti con ricontrollo.

La preparazione può leggere i pin immutabili della stessa sessione già aperta,
riconoscendone root/PID/lease runtime, senza concedere un altro writer. Le altre
sessioni attive continuano a essere rifiutate. La validazione del profilo non
promette ABI/feature di un eseguibile arbitrario; il worker mantiene il preflight
finale. Le verifiche effettive sono nel
[record DBG2-05](changes/2026-10-29-add-debug-parameter-workbench.json).

DBG2-05 è verificato con 121 casi Rust distinti nei rispettivi scope Windows/
Wine e con la catena GUI registrazione → un passo esplicito → preparazione
replay/audit/cattura/confronto → avvio separato → report. I quattro profili
creati con gli stessi parametri CLI coincidono in JSON ed esito/stato/clock.
La pagina di punteggiatura permette virgole e separatori da controller;
parametri incompleti vengono rifiutati prima dell'esecuzione. Le prime prove
hanno corretto un canvas non pubblicato e il badge sovrapposto; immagini e log
negativi restano distinti dalle verifiche finali a tre dimensioni.

## Script and tool library (DBG2-06)

`Debug > Scripts and tools` separates declared host adapters, readonly command
recipes and guest script metadata. Selection, hover, import and Details never
launch a process. The form has Details / verify, Start explicitly, Cancel
process and scalar presets; Back leaves a running job visible in host status.
Cancel remains available while the worker is busy. Closing the application
requests cancellation and waits for its supervisor.

The v1 adapters call the public Python tools directly, using argument arrays:
report homepage (`--root` selected session, `--stdout`), campaign `report`
(required explicit case ID), artifact `preview` (selected closed session). They never
invoke campaign replay or artifact `apply`. Select an explicit Python3.11+
executable in the internal picker; absence is reported without installing it.
PowerShell/Bash wrappers and Docker are not used by these direct adapters.
Version is checked only at Start; Details hashes the executable without running
it. The runtime identity is the selected executable hash, not a claim about
all installed Python modules/DLLs.

Embedded public scripts, helpers and HTML templates are hashed/versioned in
Details and staged into each new run only on Start. With source=`external`, the host-script
reference must have the exact declared filename and bytes; sibling dependencies
must match too. Modified code is refused even after reselecting it. Source=`embedded` ignores
a previous external selection, so switching adapters does not inherit code. The copied
known bytes are executed, avoiding a changed external script between checking
and launching. Python uses explicit UTF-8, isolated/no-site/no-bytecode mode and a constant
bootstrap with source paths supplied as separate argv.

Outputs are private under `config/debug-tool-runs/<new-id>/`: request/result
JSON, per-process stdout/stderr, and homepage HTML or the existing report folder.
The root homepage is never replaced. Eight runs are retained without automatic
pruning, each has a20..120000ms deadline, and each process stream is capped at
8MiB; overflow stops the process with `log-limit`. Cancellation/timeout retains
partial logs and a distinct outcome. Windows starts suspended, assigns a kill-on-close job before resuming the
process;
Unix uses a dedicated process group. Descendants are stopped before collecting
pipe EOF. Containment failure refuses the launch and stops its child.

The initial recipe schema is `schema:1`, `kind:ffta-debug-recipe`, a printable
label of1..200bytes and1..16 `steps`. A step is either
`{"operation_id":"control.status"}` or
`{"operation_id":"control.inspect","address":"08000000","length":16}`.
Unknown fields, other operations and invalid CLI arguments are refused.
Start invokes the selected recomp runtime against the selected session, with
per-step JSON stdout; this readonly subset advances no guest frames. The
broader recipe engine remains DBG2-17.

The separate Guest scripts view reads bounded schema1 metadata or actual typed
extractor projections (without altering their original files), source
hash and declared entry counts. It makes no new verification claim and never
executes its bytes. Gameplay inspector opens the existing parameter form;
starting its observer still requires authenticated supported ROM/models.
Validation: Windows GNU build passes without warnings. Wine covers80distinct
Rust behavioral cases (including the explicit UTF8 proof) and3Python inventory
checks. Actual GUI exercises four explicit readonly runs, typed selections,
script/dependency/runtime refusals, simulated XInput cancellation and confirmed
descendant stop, timeout, guest catalog/inspector link and three client sizes.
The main full state remains at frame0/clock0. Final encoding proof uses the same
closed public source and a new output only; original reports stay retained as
negative locale-encoding evidence. Final HTML preserves the Unicode path in
UTF8. Source/runtime/input/output preservation passes; all GUI jobs are closed.
Evidence: `target/debug-tools-v2-library/`. Native Windows/Linux, physical
controllers and wrapper/Docker execution remain unverified.

## Persistent choices (DBG2-07)

`Data/config/debug-choices.json` follows the selected config root. It stores
app/config-relative internal locations, explicit external locations and added
file/folder provenance, typed input SHA/kind pins,18scoped views (all plus17roles)
and global favorites/recent/highlight bookmarks. Limits:256KiB,8roots,17input
roles,512favorites and32recent entries, plus per-view highlights. Internal paths
never store the installation directory. External paths retain their explicit
location; moving the app does not silently redirect them.

At the first paused menu context, load bounded metadata and asynchronously
check saved primary files with a64MiB budget; the existing type-specific readonly
preflight limits also apply. Directory scans, OS pickers and tool starts stay
explicit. Missing, changed or incompatible roles remain saved with explanations;
valid references alone become usable. Each action still rechecks them. A new
ROM/context re-preflights the bindings, retaining neutral runtime choices while
refusing incompatible labeled/snapshot/resource inputs. Each isolated storage
root owns its preference file.

Views save committed search text, type/state/collection/sort and highlight.
Favorites/recent are remapped by location when an explicitly added internal
root changes ID after a folder move; restoring them does not confirm or run an
item. Incomplete editor text stays a draft. Debounced paused writes and exit
flush use same-directory atomic publication, preserve detected external edits
and leave unknown/invalid-schema files untouched. Late restore results cannot
replace a newer explicit pin or apply to another ROM/storage context.

Windows GNU build passes without warnings;18distinct Wine cases cover schema,
path/pin/atomic storage, scoped history/root rebasing and picker lifecycle.
Actual GUI saves four roles and internal/external roots, then reopens the same
public seed after a Unicode/spaced folder move from `/tmp`. Runtime/snapshot
remain usable; missing labels and a changed external file are explained while
all four original pins remain saved. Query/favorites/recent/sort and the rebased
highlight return without automatic directory scan, dialog, confirmation or
execution. Full main state/frame0/clock0 remains exact. The initial verbatim
Windows drive-prefix failure and its volatile GUI are retained as negative
proof; the corrected guard checks rooted directories rather than the volume
prefix alone. Native Windows/Linux and physical controllers/IME remain
unverified. Evidence: `target/debug-tools-v2-choices/`.
