# Task handoff

## Stato operativo

Aggiornato il 2026-11-01. LINUX-MENU-CHEAT-INPUT concluso: riga CheatCatalog
aggiunta; build/export Docker Linux completa PASS,758 test/0fail/356ignored.
Android e ROM restano attività separate con i loro handle, non riprese qui.

Leggere questo file per intero e il passaggio finale; poi soltanto le sezioni
pertinenti indicate sotto. Non caricare automaticamente archivio, CHANGELOG,
PROJECT_STATE completo o grandi TODO. Le prove accettate restano valide anche
se gli artefatti locali sono stati ritirati. Regole: [MEMORY_WORKFLOW](MEMORY_WORKFLOW.md#resume-and-repeat-decisions).

Stato di esecuzione ed esito sono distinti: `attivo`, `sospeso`, `interrotto`,
`fallito`, `in_attesa`; esiti aperti `parziale`, `integrato_non_verificato`,
`non_iniziato`. Un task completo/respinto si chiude nell'archivio, dopo verifica
dei criteri; nessun padre aperto riapre figli conclusi. `sospeso` qui significa
fuori dalla selezione di questa sessione, non autorizzazione a fermare processi
altrui né modifica dello stato dei loro goal.

## Task in esecuzione o sospesi

### ROMDATA-03-04-E105-SENARY-COMPACT-OBJ

- Stato: attivo
- Esito: non_iniziato
- Verificato: sorgente e modello sparso 756 byte PASS Wine, due span e gap2; dataset e105 accettato con95 risorse.
- Manca: pubblicazione, autenticazione loader e composizione runtime che preservi gap, input e risorse precedenti.
- Ripresa: integrazione e test scritti; build Windows isolata attiva sessione 51589, log `target/rom-data-compact-obj-senary-runtime/`. Attendere poi test Wine predisposto se PASS; fallback81 verificato staticamente, gap preservato.

### ROM-DATA-FULL-EUR

- Stato: sospeso
- Esito: parziale
- Verificato: e105-d104, 95 risorse/96 file; CLI create/reuse/startup e 98 file protetti PASS nel risultato `target/rom-data-compact-obj-quinary-cli/result.json`, letto in questo audit. Neutro 24755→24756 con nuovi contatori zero: non prova di uso ordinario. Source/model/runtime/CLI e le negative concluse non si ripetono.
- Manca: risorse/consumatori non migrati, prove ordinarie ancora mancanti, ABI/global extent/indiretti e fasi mod ROMDATA-06/07; Linux/USA/Windows nativo/new32:9 restano distinti. EUR, Wine ridotto, niente misure prestazioni/spazio; originali e salvataggi personali immutati.
- Ripresa: [ROM_DATA_TODO](ROM_DATA_TODO.md), solo la sezione del criterio scelto (ROMDATA-01/02/03/04/05/06/07); [inventario](ROM_DATA_INVENTORY.md). Non leggere il TODO intero. I seguenti figli conservano ID e limiti, senza nuovi tentativi in questa sessione:
  - `ROMDATA-04-PARAMETER-RECORDS27-ORDINARY`: source/model/runtime/CLI accettati; 59 record osservati, dominio totale non provato; mancano uso normale positivo e coppia completa. Undici catture negative chiuse; archivio vecchie righe 17624..17672 e `docs/rom_data_gameplay.md`, sezione ordinary sparse parameter record reachability.
  - `ROMDATA-04-LOCALIZED-TABLE-GRAPHICS-ORDINARY`: 40 banchi/8 tabelle source/model/runtime/CLI accettati, manca trigger normale positivo e coppia; i tentativi negativi restano chiusi. `docs/rom_data_ui_graphics.md`, sezione ordinary table extension reachability; archivio 17844..17864.
  - `ROMDATA-05-DYNAMIC-RAW-ORDINARY-REACHABILITY` e `ROMDATA-01-LOCALIZED-RAW-DYNAMIC-DOMAIN`: frammenti/percorsi condizionali accettati, dominio globale e invocazione ordinaria non provati. Unit320 negativo chiuso; successiva diagnosi state2/state4/selector-witnesses chiusa senza scena. Archivio 59..75 e 789..866; nessuna navigazione alla cieca.
  - `ROMDATA-01-OBJ-REMAINING-DRAW-ROOTS`, `ROMDATA-01-OBJ-REMAINING-CONSTRUCTOR-SOURCES`, `ROMDATA-01-OBJ-GRAPHICS-PAYLOADS`: incrementi autenticati accettati, censimento globale/consumatori residui aperti; non ripetere l'esperimento ampio con 61 troncamenti. Archivio 2853..2874, poi soltanto il record della famiglia selezionata.
  - `ROMDATA-05-OBJ-VALIDATION`: equivalenza/costi locali, GUI, archivio e initializer USA hanno esiti successivi accettati; rimangono gates Windows nativo, ulteriori cataloghi/scene e abilitazione normale. Archivio 3945..4087. I vecchi vincoli artificiali RSS/spazio e `ROMDATA-05-BG-SCENES-COSTS` non autorizzano altre misure: superati dallo steering EUR/no-metrics.
  - Compact OBJ e101..e105: prefissi condizionali/modello/runtime/CLI accettati, uso ordinario ancora aperto; record222/candidate-table e selector33 hanno sole condizioni statiche e negative limitate. Archivio 11..65; non inferire una scena dal nome.

## Task incompleti e criteri aperti

Audit DBG2 del 2026-11-01: tutti gli undici padri 13..23 restano senza spunta;
8 hanno lavoro parziale/integrato, 3 non hanno l'implementazione richiesta.
Le verifiche sotto sono preesistenti, consultate senza rieseguirle. I criteri
originali e le note di audit sono in [DEBUG_TOOLS_V2_TODO](DEBUG_TOOLS_V2_TODO.md#p2--console-di-comandi-e-ricette-per-lassistente).
DBG2-01..12 restano conclusi nel loro ambito. Dettagli storici solo su dubbio
specifico: archivio vecchie righe 179..744, oppure il record nominato sotto.

### DBG2-13

- Stato: sospeso
- Esito: parziale
- Verificato: parser tipizzato, selezione ID catalogo/profilo, cancel/deadline, retry, discovery e replay condiviso hanno prove Windows GNU/Wine. `2026-10-31-prepare-debug-content-id-selection.json` e `2026-10-31-connect-shared-replay-commands.json` in docs/changes; profile-result e replay-panel/gui-result confermano selezione/panel, senza replay personale.
- Manca: parità completa menu/CLI/console di validazione, azioni e risultati per tutte le operazioni richieste, non solo quelle selezionate.
- Ripresa: confrontare contratto e handler del solo gruppo mancante in debug_command.rs/debug_control.rs/play.rs; non rifare selezione profilo, cancel o replay già accettati.

### DBG2-14

- Stato: sospeso
- Esito: integrato_non_verificato
- Verificato: editor Unicode, history/selezione, completamento contestuale, tastiera XInput simulata, focus/hide, clipboard isolata e incolla multilinea senza invio accettati. `docs/changes/2026-10-31-begin-debug-command-console.json`, add-debug-command-context-completion e add-debug-command-controller-keyboard; clipboard/gui-result letto e positivo.
- Manca: composizione IME reale e layout pertinenti; guard composizione/capacità non verificati in GUI. Windows nativo/Linux/controller fisico non verificati, da distinguere dal Wine già accettato.
- Ripresa: selezionare solo il criterio input ancora mancante; editor/panel/completion e focus in play.rs. Non ricreare la prova clipboard.

### DBG2-15

- Stato: sospeso
- Esito: parziale
- Verificato: JSON esiti/target/progresso, ownership, cancel/deadline, idempotenza, retry testuale, discovery e replay/panel verificati nei record show-debug-command-job-context, classify-debug-command-local-errors, add-debug-command-retry e connect-shared-replay-commands del 31 ottobre. terminal-result conferma pending null; la copia JSON replay coincide.
- Manca: procedura assistente completa e parità dell'intera API console; cancel/deadline su replay che sta realmente eseguendo frame guest (finora slow child sintetico), live refresh lento e presentazione completa in finestra stretta.
- Ripresa: solo il flusso mancante; non riaprire step retry, wrong-ID, deadline sintetica o selezione già chiusi. Un timeout client è consegna/esito non confermato, non processo bloccato.

### DBG2-16

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: runner host/ricette osservative DBG2-06 e replay supervisionato esistono; non costituiscono batch console con checkpoint.
- Manca: sequenza comandi tipizzati salvata, pin, assert/attese/cap, cancellazione e ripresa senza ripetere passi riusciti.
- Ripresa: dipende da DBG2-15; nessun nuovo sviluppo durante l'audit.

### DBG2-17

- Stato: sospeso
- Esito: parziale
- Verificato: indice readonly 341 voci, import vuoto/esplicito, menu/controller simulato, metadati/provenienza/master, contesto hash EUR/USA, canale locale e guida/package exclusion. Record index-local-cheat-catalog-readonly, browse-local-cheat-catalog-in-menu, describe-cheat-target-and-public-guide, expose-cheat-snapshot-local-control del 31 ottobre. menu-gui2 conferma 341 sintetiche/import/stato esatto.
- Manca: indice runtime/verifica per ROM/dispositivo/decoder; summary/menu_entries dichiarano ancora decoder unsupported e nessun handler, pur esistendo source-bound inspection/one-shot verificati. Raccordare capacità per CH; non promuovere comportamento/dispositivo a verificato dai sette piani statici.
- Ripresa: [cheats.md](../cheats.md#readonly-index-and-cli--dbg2-17-in-progress), debug_cheats.rs e contratto; import/browsing sono conclusi, non da reimplementare.

### DBG2-18

- Stato: sospeso
- Esito: parziale
- Verificato: sottoinsieme CodeBreaker con fixture sintetiche, ispezione plaintext esplicita, seed esplicito, source/master/CRC; sette priorità EUR confrontate staticamente con mGBA. Record inspect-assumed-plaintext-codebreaker, inspect-explicit-seeded-codebreaker e inspect-source-bound-codebreaker-masters del 31 ottobre.
- Manca: integrazione capacità per CH eseguibile coerente con l’indice ancora fisso unsupported (criterio ciascun CH riporta ciò che può eseguire). Il sottoinsieme è verificato; altri opcode/formati possono restare rifiutati. Master decodificato non è hook eseguito, limite distinto dal decoder.
- Ripresa: [cheats.md](../cheats.md#assumed-plaintext-codebreaker-inspection--dbg2-18-in-progress), debug_cheat_decoder.rs; non duplicare il decoder già verificato.

### DBG2-19

- Stato: sospeso
- Esito: parziale
- Verificato: preview source/state-bound e one-shot manuale nel bus di una copia interprete in pausa, rifiuti atomici, retry senza duplicazione; record apply-guarded-cheat-once del 31 ottobre. Non è mantenimento periodico.
- Manca: arm/mantieni/disarma distinti, timing deterministico, conflitti/sovrapposizioni e copertura memoria/backend supportata; I/O write, native/ROM e mirror non supportati.
- Ripresa: [cheats.md](../cheats.md#manual-one-shot-in-an-isolated-session--dbg2-19-in-progress); non ripetere la prova one-shot accettata.

### DBG2-20

- Stato: in_attesa
- Esito: parziale
- Verificato: ricerca/dettagli/filtri readonly e comandi prepare/apply-once esistono; menu resta readonly, senza attivazione implicita.
- Manca: valori/range, gruppi/preset, toggle, disattiva tutti, indicatore attivi, controlli conflitti e flusso controller completo. Dipende da DBG2-19 e DBG2-15.
- Ripresa: soltanto dopo la dipendenza richiesta, senza duplicare la vista catalogo o confondere CLI one-shot con pagina Cheats completa.

### DBG2-21

- Stato: in_attesa
- Esito: parziale
- Verificato: CH-0321/0323/0324/0325/0331/0334/0336 decodificati staticamente; CH-0334 cambia due valori RAM e il seed riaperto ripristina stato esatto. `target/debug-tools-v2-cheat-eur-resource/result.json` letto: true per due scritture/ripristino/originali invariati, nessun payload privato.
- Manca: almeno un accesso UI e un modificatore risorse con effetto di gioco, attivazione/disattivazione/ripristino isolati; CH-0321 combinato non è semplice HP.
- Ripresa: dipende da DBG2-19/20; non equiparare RAM statica o nuovo seed a toggle reversibile o prova UI.

### DBG2-22

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: catalogo/decoder generico non provano effetti dei gruppi avanzati.
- Manca: prove per ciascun CH scelto su abilità/AP/regole/transizioni con prima/dopo ed effetti persistenti; nota Clear Mission incerta.
- Ripresa: dopo DBG2-21, con gruppo piccolo esplicito; niente sviluppo o attivazione in questa sessione.

### DBG2-23

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: play.rs rifiuta nuove registrazioni/salvataggi dopo one-shot, in attesa di provenienza; debug_recording.rs/debug_replay.rs non implementano il contratto cheat richiesto.
- Manca: set/hash/parametri/ordine/frame/decoder nei checkpoint e report, semantica Load/Undo/reset/ROM, replay stato-clock e rifiuto set incompatibile.
- Ripresa: dopo DBG2-19/21; il guard di rifiuto è una protezione, non completamento di provenienza/replay.

### DBG2-24

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: basi precedenti accettate; nessuna chiusura registrata del padre 24, fuori audit 13..23.
- Manca: trigger nominati; dipendenze DBG2-05/15 e DBG-08/11.
- Ripresa: solo il criterio DBG2-24 nella sezione P4 del TODO, su richiesta pertinente; nessuno sviluppo selezionato ora.

### DBG2-25

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: basi precedenti accettate; nessuna chiusura registrata del padre 25, fuori audit 13..23.
- Manca: valori osservati e differenze fissati a schermo; dipendenze DBG2-03/15 e DBG-11/17.
- Ripresa: solo il criterio DBG2-25 nella sezione P4 del TODO, su richiesta pertinente; nessuno sviluppo selezionato ora.

### DBG2-26

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: basi precedenti accettate; nessuna chiusura registrata del padre 26, fuori audit 13..23.
- Manca: timeline e ricetta esportabile; dipendenze DBG2-11/16/23 e DBG-09/16.
- Ripresa: solo il criterio DBG2-26 nella sezione P4 del TODO, su richiesta pertinente; nessuno sviluppo selezionato ora.

### DBG2-27

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: basi precedenti accettate; nessuna chiusura registrata del padre 27, fuori audit 13..23.
- Manca: matrice mirata di integrazione/regressione; dipendenze criteri precedenti e piattaforme/input distinti.
- Ripresa: solo il criterio DBG2-27 nella sezione P4 del TODO, su richiesta pertinente; nessuno sviluppo selezionato ora.

### DBG2-28

- Stato: in_attesa
- Esito: non_iniziato
- Verificato: basi precedenti accettate; nessuna chiusura registrata del padre 28, fuori audit 13..23.
- Manca: guida, distribuzione e compatibilità delle versioni; dipendenze DBG2-27 per le funzioni distribuite.
- Ripresa: solo il criterio DBG2-28 nella sezione P4 del TODO, su richiesta pertinente; nessuno sviluppo selezionato ora.

### PERFORMANCE-TODO

- Stato: sospeso
- Esito: parziale
- Verificato: infrastruttura/replay/baseline e ottimizzazioni accettate PERF-03/05/06/08; timer/audio, frame-boundary hardened, event allowance e timing selezionati hanno risultati terminali. Esperimenti compact-ID/18-ROM-seeds/post-wait/row-bounds restano respinti.
- Manca: raro lag originale, attribuzione reale/overhead realtime, hardware/Windows nativo/Wayland, lifecycle fisico e criteri PERF-01/02/04/07/09/10; PERF-11/12/13 condizionali. `PERF-07-EXACT-ROM-HINTS` conserva l'ID della vecchia integrazione: metadati/cache, equivalenza default/explicit, replay/GUI/export non risultano chiusi in quel checkpoint. Nessuna sua esecuzione selezionata ora.
- Ripresa: solo su richiesta performance; [PERFORMANCE_TODO](PERFORMANCE_TODO.md#stato-dellesecuzione) per stato sintetico, archivio vecchie righe 4998..5049 e record 2026-10-21-preserve-native-event-boundaries per integrazione. Non rifare hardened/allowance/timing già accettati.

### DEBUG-SAVES-EXPANSION

- Stato: sospeso
- Esito: parziale
- Verificato: import/copie/boot e scene accettate restano tali; vecchio inventario 6 valide/9 parziali/11 scoperte non è il nuovo stato globale. Incrementi successivi e DS-20 utente chiusi; un'interruzione GUI non prova normal-close.
- Manca: soltanto sottocasi scena ancora scoperti/parziali nel catalogo corrente; IN05 File2, IN19 wrapper EUR e IN18 conversione rimangono piste non accettate, da riconciliare prima di agire.
- Ripresa: solo su richiesta pertinente; docs/development/debug saves.md e catalogo locale, record 2026-10-23-audit-imported-debug-saves, archivio 2551..2573 e 2643..2736. Non ricreare fixture/import o esplorare scene ora.

### LOCAL-ARTIFACT-HYGIENE

- Stato: interrotto
- Esito: parziale
- Verificato: applied.json del 26 ottobre documenta pulizia applicata; sweep successivi hanno ricevute distinte e chiuse. Nessuna cancellazione in questa riorganizzazione.
- Manca: nel task originale chiusura finale validazione/memoria non attestata; le pulizie successive non provano automaticamente tutti i suoi criteri.
- Ripresa: solo su richiesta cleanup, controllare ricevute e limiti del task prima di eventuale sola chiusura documentale; archivio 2486..2496 e `target/local-artifact-cleanup-2026-10-26/applied.json`. Nessuna nuova sweep per ricostruire contesto.

## Blocchi, dipendenze e risorse condivise

- Asincroni: il 2026-11-01 l'inventario collaborazione di questa sessione mostra solo `/root`; ps è limitato al namespace sandbox. Nessun job build/Wine/runtime è stato avviato, terminato o segnalato da questa sessione. Stato host/altri thread non osservabile: non dichiarare assenza globale di processi. I risultati storici letti attestano chiusura dei loro figli, non lo stato host odierno. Prima di riprendere controllare handle/processo, output e proprietario; assenza di risposta o handle perso non è fallimento.
- Risorse condivise: checkout, handoff/archivio/CHANGELOG, target canonici, prefix Wine, display, dataset e fixture. Non fermare processi altrui, non ripristinare sorgenti/binary senza ownership e non eseguire cleanup dei task vivi/falliti. Audit Wine retention storico: eliminazione prefix richiede coordinamento/visibilità host; non deriva da ps sandbox. Disco quasi pieno osservato nel vecchio sweep: disponibilità attuale non misurata qui.
- Scritture memoria: un solo writer coordinato; proposte con hash di entrambe le preimmagini e lock comune tramite scripts/task_handoff.py update. Se cambiano, rileggere e fondere prima di pubblicare. Nessun background writer e nessun lock autorizza a interferire con altri job. Dettagli in MEMORY_WORKFLOW.
- Storico integrale: [TASK_COMPLETED_ARCHIVE](TASK_COMPLETED_ARCHIVE.md#legacy-handoff-snapshot), snapshot di provenienza, non coda operativa. Include vecchie descrizioni parziali per non perderle: i task aperti restano qui, non sono promossi a chiusi. `TASK_HANDOFF_MIGRATION.json` dà sezioni, hash e indice degli esiti terminali registrati. Cercare un ID e leggere solo l'intervallo necessario.
- Godot: assessment pianificato concluso, implementazione differita, non dipendenza corrente. AndroidPort è separato; originali/ROM/BIOS/cataloghi privati/save/recording non entrano in commit/pacchetti.

## PASSAGGIO DI TESTIMONE

- Task: LINUX-MENU-CHEAT-INPUT concluso; riga CheatCatalog aggiunta, test originale invariato.
- Ultimo risultato verificato: Docker Linux build/export completo exit0,758PASS/0FAIL/356ignored/0warning; menu106PASS/1ignored e precedente BG PASS.1365 input esatti; archivio14281636B/SHA583ad0fdea8f05eab9f6daf061a00dc45b4d9c841748d575e3aa528c33ebeccc,82payload e BUILD-INFO verificati.
- Da completare: nulla in questa correzione. Android/ROM e casi ignorati restano nei rispettivi scope.
- Errori o impedimenti: nessun errore build. Fullmemorycheck segnala11path concorrenti Android/export senza record, preservati; metadata/struttura/Atlas/diff PASS. Modello/xhigh non esposti.
- Prossima azione: nessuna; usare export isolato verificato o attendere nuova richiesta, senza riprendere altri task.
- Riferimenti: docs/changes/2026-11-01-expose-cheat-catalog-in-tool-inputs.json; target/linux-menu-cheat-input/{result.json,docker-build.log,source-hashes.json,export/FFTARecomp-linux-portable.tar.gz}; docs/debug_selector.md.
