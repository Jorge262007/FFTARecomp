# TODO — Debug integrato: selettori, console e cheat

Creato il **29 ottobre 2026**. **Implementazione avviata** dalla richiesta
«esegui il todo debugtools v2»; le spunte sotto indicano le sole unità concluse.
Il [primo piano DBG-01…DBG-22](DEBUG_TOOLS_TODO.md) rimane completato nel suo
ambito; qui si definiscono estensioni con nuove prove, senza riaprire le vecchie.

Obiettivo: usare gli strumenti di debug interamente dal menu integrato,
scegliendo contenuti e parametri senza dover digitare percorsi o aprire finestre
di selezione file. Aggiungere un menu cheat per preparare casi di debug, una
console eventi visibile su richiesta e una console di comandi testuali, utile
anche all'assistente. Windows resta il primo riferimento; Wine, Windows nativo
e Linux hanno risultati distinti. Il frontend attuale è sufficiente: Godot e
Android non sono dipendenze.

**DBG2 identifica uno sviluppo, CH una voce del catalogo cheat, DS una scena.**
Una prova ottenuta con cheat resta una variante modificata con provenienza;
non promuove automaticamente una scena DS o la parità del runtime.

## Basi e lacune osservate

- I [22 strumenti esistenti](../debug_tools.md) offrono copie isolate, catalogo
  DS, registrazioni, replay, audit, confronto, ispettori, report e conservazione.
  Il catalogo del [menu](../../crates/menu/src/debug_catalog.rs) è già navigabile:
  riutilizzarne selezione, anteprima, filtri e gestione degli errori.
- In [play.rs](../../crates/recomp/src/play.rs), start_replay apre un FileDialog
  quando manca il profilo; open_debug_catalog fa altrettanto senza catalogo
  predefinito; CreateIssueReport può richiedere un file automaticamente.
  Choose replay profile e Choose catalog aprono ancora il selettore esterno.
- Diverse azioni menu delle catture usano Settings::default o parametri fissi.
  La CLI espone filtri e limiti che devono diventare configurabili nel menu.
- Il [canale locale DBG-20](../debug_control.md) fornisce identità della sessione,
  query, cancellazione per ID e scadenze. La [traccia UI](../debug_ui.md) osserva
  input/focus/dispatch; il [registro recente](../debug_recent.md) ha già cap,
  contatori di scarto e freeze su errore. Sono basi per le console, non console
  interattive già presenti. Gli hook UI attuali non coprono tutti gli input raw.
- Il [catalogo cheat locale](../cheats.md), in cheats/catalogo.json schema 1,
  contiene 341 blocchi: 23 EUR Codebreaker, 112 Codebreaker con regione ignota,
  20 GameShark USA, 111 GameShark JPN e 75 Action Replay con regione ignota.
  Tutte le voci sono not-runtime-tested. La presenza in elenco non prova che
  formato, versione dispositivo o indirizzi funzionino nel runtime.

Questi riferimenti descrivono il punto di partenza letto il 29 ottobre.
Verificare il codice e il criterio selezionato quando si inizierà lo sviluppo.

## Comportamento richiesto

- **Selezione interna per default.** Replay, script, cataloghi, registrazioni,
  snapshot, dataset, runtime, librerie, label, piani noti, campagne e report
  disponibili si scelgono in liste del menu, con filtri e dettagli.
  Aprire una schermata, incontrare una lista vuota o un file mancante non apre
  mai automaticamente un dialogo del sistema né richiede un percorso testuale.
- **Accesso esterno esplicito.** Soltanto un pulsante chiaramente nominato
  «Sfoglia file esterno…», «Importa…» o «Scegli cartella…» può aprire il dialogo.
  Annullarlo conserva selezione e stato. Selezionare non equivale a eseguire:
  usare azioni distinte «Seleziona», «Avvia», «Applica» e «Mostra report».
- **Tre significati di script.** Distinguere ricette di comandi debug, strumenti
  host Python/PowerShell/Bash e script del gioco ricavati da modelli verificati.
  I primi due hanno avvio esplicito e requisiti; i terzi si selezionano per
  ispezione/provenienza, senza trattare dati ROM come programmi host.
- **Menu, console e assistente condividono le operazioni.** Stessa validazione,
  stessi parametri, stesso controllo della sessione, stessi esiti e limiti.
  Catalogare prima i comandi realmente disponibili; gli esempi di sintassi
  sotto sono proposte, non comandi già utilizzabili.
- **Due console distinte, entrambe opzionali.** Console eventi: sola lettura,
  ogni evento delle categorie attivate e ogni cambio effettivo della selezione
  host, con causa ed esito. Console comandi: editor e risposte testuali/JSON.
  Mostrare esplicitamente hook mancanti, filtri, aggregazione e scarti; «Tutti»
  non può dichiarare osservati eventi CPU/guest non strumentati.
- Operazioni osservative preservano stato/clock. Cheat e comandi che cambiano
  il gioco dichiarano le modifiche e lavorano nella copia di debug. Identità,
  input, salvataggi e log seguono le regole portable e di conservazione esistenti.
  Disabilitare un cheat interrompe le applicazioni future: non promette di
  annullare gli effetti già prodotti. Il ripristino usa un checkpoint esplicito.

## P0 — Selezione e configurazione dal menu

- [x] **DBG2-01 — Inventario delle azioni e contratto comune.**
  Censire ogni azione DBG esistente e ogni parametro CLI, indicando selezione
  richiesta, prerequisiti, effetti, stato consentito e risultato. Individuare
  tutti i percorsi che aprono dialoghi e quelli con impostazioni fisse.
  Definire descrittori riusabili da menu/help/console sopra gli handler attuali,
  senza creare un secondo esecutore del guest.
  **Chiusura:** ogni azione ha una destinazione nel nuovo menu e un esito
  leggibile/strutturato; gli eventuali parametri ancora non esposti sono elencati
  come lavoro aperto, non occultati. **Dipende da:** basi DBG-01/20.
  Contratto e uso: [operazioni debug](../debug_operations.md), inclusi i tre
  siti di dialogo e i parametri ancora fissi/non esposti. Chiuso con build Windows,
  test Wine sintetici, query positive/negative e controllo dei payload pubblici;
  [record](../changes/2026-10-29-add-shared-debug-operation-contract.json).

- [x] **DBG2-02 — Catalogo comune dei contenuti selezionabili.**
  Indicizzare radici portable dichiarate e cartelle aggiunte esplicitamente:
  sessioni, registrazioni, profili, script/ricette, campagne, snapshot, runtime,
  dataset, librerie native, label, manifest noti e report. Conservare ID stabile,
  tipo, nome, descrizione, ROM/regione, hash, compatibilità e disponibilità.
  Scansioni limitate e aggiornamento esplicito; distinguere elenco da verifica
  completa e da esecuzione. Non scandire indiscriminatamente il disco.
  **Chiusura:** catalogo vuoto, duplicati, collegamenti non ordinari, file
  cambiati/mancanti e tipi incompatibili hanno esiti espliciti; nessun elemento
  viene eseguito per indicizzarlo. **Dipende da:** DBG2-01.
  Implementato: [catalogo contenuti](../debug_content.md), `debug-content scan/check`;
  fixture sintetiche Wine, rifiuto di link reali, limiti, omonimi e cartella
  spostata verificati; [record](../changes/2026-10-29-add-bounded-debug-content-catalog.json).

- [x] **DBG2-03 — Selettore riutilizzabile dentro il menu.**
  Lista con ricerca, categorie, filtri, ordinamento, preferiti/recenti e pannello
  di dettagli; anteprima dove già disponibile. Navigazione completa con
  tastiera, mouse e controller; scroll, focus, testo lungo e ritorno coerenti.
  Pulsanti separati per scelta interna, aggiornamento e importazione esterna.
  **Chiusura:** scegliere due elementi omonimi con identità diverse, cambiare
  filtro, annullare e gestire lista vuota senza dialoghi OS automatici né
  azioni involontarie. **Dipende da:** DBG2-02.
  Implementato e verificato: [selettore interno](../debug_selector.md),
  tastiera/mouse/XInput simulato, omonimi, ricerca/filtri, lista vuota,
  preferiti/recenti e anteprima. Corretta la propagazione di Escape dopo Cancel
  dei dialoghi; prova successiva con stato completo/frame0/clock0 invariati.
  [Record](../changes/2026-10-29-add-integrated-debug-content-selector.json).

- [x] **DBG2-04 — Migrare tutte le scelte degli strumenti esistenti.**
  Portare al selettore interno Choose catalog/profile e i fallback di
  Replay/Audit/Capture/Compare/Create issue. Estendere la scelta a sessioni,
  snapshot/input, dataset, runtime A/B, librerie, label e piano noto usati dai
  rispettivi flussi. Conservare la scelta corrente visibile e compatibile.
  Se manca un input, aprire la pagina interna che spiega cosa selezionare.
  **Chiusura:** esercitare ogni ingresso censito in DBG2-01 con input presente,
  assente, alterato e incompatibile; zero dialoghi automatici. Solo il pulsante
  esterno apre un picker, una volta, e Cancel non avvia nulla.
  **Dipende da:** DBG2-03.
  Implementato: [scelte tipizzate](../debug_selector.md), dodici ruoli,
  zero dialoghi automatici e avvio distinto dalla conferma. Verificati ingressi
  mancanti/presenti, file alterati/assenti, ROM e schema incompatibili, dialoghi
  espliciti singoli e Cancel; [record](../changes/2026-10-29-migrate-debug-tool-input-selection.json).

- [x] **DBG2-05 — Parametri, preset e preparazione dei profili nel menu.**
  Esporre frame/timeout, maschere input, range, filtri, stream audio, trigger,
  confronto precedente, sintomo issue e opzioni delle tracce. Creare e salvare
  profili/preset con riepilogo degli input selezionati; distinguere modifica
  della bozza da avvio e non sovrascrivere un profilo autenticato.
  Mostrare feature mancanti e combinazioni non supportate prima di avviare.
  **Chiusura:** preparare registrazione → replay → audit/confronto/report senza
  editor esterno o percorso digitato; parametri ed esiti coincidono con CLI.
  **Dipende da:** DBG2-01/04.
  Implementato il [workbench parametri](../debug_selector.md): bozze/preset,
  input tipizzati, verifica/preparazione/avvio distinti e profili nuovi. Catena
  GUI e quattro profili equivalenti CLI verificati con esiti/stato/clock uguali;
  [record](../changes/2026-10-29-add-debug-parameter-workbench.json).

- [x] **DBG2-06 — Libreria di script e strumenti avviabili.**
  Menu «Script e strumenti» con ricette di debug e adattatori dichiarati per
  gli script host utili, inclusi homepage, campagne e artefatti. Ogni voce ha
  versione/hash, argomenti tipizzati, prerequisiti Python/PowerShell/Bash/Docker,
  output previsto e pulsanti Dettagli/Avvia/Annulla. Passare gli argomenti come
  array, senza costruire comandi shell da nomi o percorsi.
  Inserire anche una vista separata degli script guest noti con collegamento
  all'ispettore gameplay; l'ispezione non li esegue.
  **Chiusura:** selezione e configurazione interne, script modificato o runtime
  host assente segnalati, processo esplicito annullabile e log rintracciabile;
  nessun avvio al solo import/hover/selezione. **Dipende da:** DBG2-04/05.
  Implementato: cinque schede,17ruoli tipizzati, adattatori readonly dichiarati,
  ricette status/inspect e vista guest separata. Wine GUI/frame0, cancel XInput
  con discendente fermato, timeout/rifiuti e correzione UTF8 verificati;
  [record](../changes/2026-10-30-add-declared-debug-script-library.json).

- [x] **DBG2-07 — Scelte persistenti e cartella portable spostata.**
  Salvare preferenze UI e riferimenti relativi alla radice corretta; mantenere
  distinti elementi interni e riferimenti esterni aggiunti dall'utente.
  Dopo cambio ROM/sessione invalidare soltanto le scelte incompatibili.
  Recenti e preferiti non avviano automaticamente replay, script o cheat.
  **Chiusura:** riaprire una copia spostata con spazi/Unicode da un'altra cwd,
  ritrovare le scelte valide e spiegare quelle mancanti senza dialoghi automatici.
  **Dipende da:** DBG2-04/06.
  Implementato: stato limitato con radici relative, provenienza esterna, pin
  e viste/storia; restore readonly senza avvio. Wine GUI riapre la stessa copia
  spostata da `/tmp`, conserva preferiti e spiega mancanti/modificati a frame0;
  [record](../changes/2026-10-30-persist-debug-content-choices.json).

## P1 — Console degli eventi opzionale

- [x] **DBG2-08 — Schema eventi e mappa della copertura.**
  Unificare gli eventi host già osservati con ID di sequenza, tempo host,
  frame/clock guest, sessione, categoria, severità, origine e ID richiesta/job.
  Elencare produttori supportati e mancanti; collegare UI/recent agli stessi
  eventi evitando doppi dispatch o registrazioni ricorsive della console.
  **Chiusura:** ricostruire l'ordine di richiesta, cambi selezione, esecuzione ed
  esito da una sequenza correlata; distinguere ordine host da eventi guest
  aggregati e hook non disponibili. **Dipende da:** DBG2-01, DBG-13/19.

- [x] **DBG2-09 — Traccia completa delle selezioni e delle azioni host.**
  Osservare pagina, voce con ID e label, vecchia/nuova selezione, dropdown,
  valore cambiato, apertura/chiusura, conferma, annullamento, binding catturato
  e comando effettivamente eseguito/rifiutato. Coprire tastiera, mouse e backend
  controller disponibili, mantenendo distinti input raw, rimappato e consumato.
  L'hover entra nel registro quando cambia davvero la selezione.
  **Chiusura:** ogni cambio effettivo del menu ha un evento; pressione tenuta,
  autorepeat, click e conferma controller non producono doppi comandi. Le lacune
  di un backend sono dichiarate. **Dipende da:** DBG2-03/08.
  Implementato: [cambi semantici e input](../debug_events.md), ID/valori/cause,
  editor e binding; hover invariato senza duplicati. Sei catture Wine con
  XInput simulato, comandi singoli/esiti correlati e stato/frame0 invariati;
  [record](../changes/2026-10-30-observe-effective-host-menu-changes.json).

- [x] **DBG2-10 — Eventi del runtime e degli strumenti.**
  Collegare sessione/ROM, pause/passi, Load/Undo/reset, registrazioni/replay,
  catture/report, script, preparazione dataset/cache, errori e progressione job.
  Aggiungere transizioni guest riconosciute, osservazioni codice/letture/audio
  soltanto nello scope realmente strumentato. I futuri cheat pubblicano
  attivazione, applicazione, rifiuto e disattivazione sul medesimo canale.
  **Chiusura:** ciascuna famiglia dichiara origine, frequenza, prerequisiti e
  copertura; errore e richiesta correlata sono rintracciabili, senza I/O nuovo
  nei callback audio o dump continui della macchina. **Dipende da:** DBG2-08.
  Implementato: [eventi runtime](../debug_events.md), origine/frequenza/requisiti/
  scope espliciti e correlazione job/errore. Build Windows GNU e test Wine,
  prove GUI delle famiglie e verifica mirata con il menu corrente; copertura
  dataset/cache non selezionati e cheat futuro dichiarati.
  [Record](../changes/2026-10-30-connect-debug-runtime-lifecycles.json).

- [x] **DBG2-11 — Pannello live della console eventi.**
  Toggle «Console eventi» dal menu e shortcut configurabile, inizialmente
  spento. Vista sovrapposta o affiancata con scroll, ricerca, categorie/livelli,
  pausa della sola vista, autoscroll, copia e salvataggio locale esplicito.
  «Tutti gli eventi disponibili» include tutte le categorie supportate attivate;
  campionamento/aggregazione eventuali sono visibili e disattivabili entro i cap.
  Mostrare finestra conservata, eventi persi e stato del collector anche quando
  la vista è congelata. La vista non modifica il log originale tramite un filtro.
  **Chiusura:** leggere una sequenza menu/comando/errore mentre il gioco è aperto,
  nascondere/riaprire il pannello e esportare la finestra con identità coerenti.
  **Dipende da:** DBG2-09/10.
  Implementato: [console eventi](../debug_events.md#finestra-eventi-live--dbg2-11),
  finestra limitata separata, filtri/pausa vista, copia/esportazione e poll
  regolabili. Test Windows/Wine e GUI con identità coerenti e stato invariato;
  [record](../changes/2026-10-31-begin-live-debug-event-window.json).

- [x] **DBG2-12 — Limiti, prestazioni osservabili e assenza di effetti guest.**
  Fissare cap di memoria/righe/byte, budget per aggiornamento e contatori di
  overflow. Limitare serializzazione/formattazione sul thread che esegue il gioco;
  console spenta senza raccolte pesanti, console aperta senza audio accumulato.
  Tenere distinti pausa del gioco, freeze della raccolta e pausa della vista.
  **Chiusura:** carico sintetico prolungato rispetta i cap e conta le perdite;
  una nuova breve sequenza selezionata conserva stato/clock con console off/on.
  Registrare il costo osservato senza riaprire campagne di benchmark storiche.
  **Dipende da:** DBG2-11.
  Implementati budget e contatori distinti, limiti sulla finestra e snapshot
  condiviso; carico sintetico, confronto off/on e breve osservazione audio Wine
  verificati. Il fallback audio per snapshot senza tap preserva lo stato.
  [Dettagli e limiti](../debug_events.md#limiti-e-costo--dbg2-12),
  [record](../changes/2026-10-31-bound-event-console-update-work.json).

## Audit documentale del 1 novembre 2026

Confrontati criteri, sorgenti attuali e record/prove già esistenti, senza build,
ROM extraction, gioco o nuovi test runtime. Nessuno dei padri DBG2-13..23 ha
prove sufficienti per una chiusura completa: 7 parziali (13/15/17/18/19/20/21),
14 integrato ma con verifica input incompleta, 3 non iniziati nel contratto
richiesto (16/22/23), pur con basi riusabili. Nessuna spunta è stata promossa.
Le note sotto distinguono sottopassi accettati e criteri mancanti. Le vecchie
limitazioni dei record valgono alla loro data, non annullano risultati successivi.

Discrepanze risolte: la checklist non descriveva gli incrementi console/cheat;
la vecchia coda chiedeva ancora selezione profilo, clipboard, cancel/deadline e
retry già verificati dopo quei checkpoint. L'import readonly delle 341 voci è
integrato e provato, ma non chiude l'indice runtime/verifica; decoder e one-shot
esistono, ma non equivalgono a mantenimento/toggle/provenienza.
Inoltre Catalog::summary/menu_entries conservano la motivazione fissa «nessun
decoder/handler implementato», ormai incoerente con source-bound inspection e
manual one-shot: integrazione delle capacità per CH da completare, non corretta
nel codice durante questo audit. Il solo guard
che rifiuta recording/save dopo un one-shot non implementa DBG2-23.
Nessun altro sviluppo selezionato. Per ripresa leggere solo il singolo ID in
[TASK_HANDOFF.md](TASK_HANDOFF.md); storico solo per un dubbio specifico.

## P2 — Console di comandi e ricette per l'assistente

- [ ] **DBG2-13 — Linguaggio di comandi sopra gli handler condivisi.**
  Definire sintassi stabile, tokenizzazione/virgolette, numeri/indirizzi e schema
  degli argomenti. Collegare help, status, pausa/passi, inspect, break/watch,
  catture/report, selezione di profili/cataloghi e job ai contratti DBG2-01.
  Prevedere comandi dichiarati di selezione, per esempio profile select ID,
  senza richiedere percorsi. La console è del debugger, non una shell OS.
  **Chiusura:** un comando menu/CLI/console con gli stessi argomenti produce
  identica validazione/azione/esito; testo malformato, numeri ambigui e comandi
  sconosciuti non mutano la sessione. **Dipende da:** DBG2-01/05/08.

  **Audit 2026-11-01 — parziale.** [Parser/dispatch](../../crates/recomp/src/debug_command.rs)
  riusa Action e il canale locale; ID selection, targeted cancel/deadline,
  discovery, retry e replay condiviso sono presenti e verificati nei sottopassi.
  [Selezione profili](../changes/2026-10-31-prepare-debug-content-id-selection.json)
  include la successiva prova CLI/panel positiva; [replay condiviso](../changes/2026-10-31-connect-shared-replay-commands.json)
  include la copia JSON dalla console. Manca la parità completa del contratto
  menu/CLI/console, non la reimplementazione di questi percorsi.


- [ ] **DBG2-14 — Editor integrato, completamento e aiuto contestuale.**
  Pannello «Console comandi» con invio esplicito, cronologia, cursore/selezione,
  copia/incolla, completamento di nomi/argomenti/ID e help contestuale. Supportare
  testo Unicode, layout tastiera e composizione input; navigazione controller
  verso comandi recenti/preset, tastiera a schermo dove necessaria.
  L'incolla multilinea resta una bozza da esaminare, senza invii impliciti.
  **Chiusura:** digitare, correggere, richiamare e annullare un comando; mentre
  l'editor ha focus i tasti non raggiungono il guest né attivano shortcut globali.
  **Dipende da:** DBG2-13.

  **Audit 2026-11-01 — integrato, verifica incompleta.** [Panel/editor](../../crates/recomp/src/debug_command_panel.rs),
  [completamento](../../crates/recomp/src/debug_command/completion.rs) e tastiera
  controller sono integrati. [Editor/clipboard](../changes/2026-10-31-begin-debug-command-console.json),
  [controller](../changes/2026-10-31-add-debug-command-controller-keyboard.json) e
  [completion](../changes/2026-10-31-add-debug-command-context-completion.json)
  hanno prove Wine/Xvfb; multilinea non pubblica, focus protegge il guest/menu.
  Restano composizione IME reale/layout e guard composizione/capacità in GUI;
  fisico/Windows nativo/Linux non provati. Non ripetere la clipboard già accettata.


- [ ] **DBG2-15 — Risposte strutturate, job e accesso dell'assistente.**
  Mostrare target ROM/sessione e parametri risolti, request ID, progressione,
  risultato breve e JSON copiabile. Riutilizzare il canale locale DBG-20 per
  elenco comandi/capacità, selezione per ID, esecuzione, status e cancel del
  medesimo comando, senza automatizzare coordinate o introdurre un server rete.
  Separare errore di parsing, incompatibilità, rifiuto, timeout non confermato,
  annullamento e completamento; status/cancel restano disponibili durante i job.
  **Chiusura:** l'assistente seleziona un contenuto e ripete una procedura dalla
  stessa API della console; identità errata e cancellazione di un altro ID sono
  rifiutate. Nessuna esecuzione duplicata ritentando una risposta persa.
  **Dipende da:** DBG2-02/13/14, DBG-20.

  **Audit 2026-11-01 — parziale.** [Controllo locale](../../crates/recomp/src/debug_control.rs)
  e [osservazione panel](../../crates/recomp/src/debug_command/observation.rs)
  espongono request/run/target, parametri/progresso ed esiti. Record:
  [job context](../changes/2026-10-31-show-debug-command-job-context.json),
  [errori](../changes/2026-10-31-classify-debug-command-local-errors.json),
  [retry](../changes/2026-10-31-add-debug-command-retry.json),
  [discovery](../changes/2026-10-31-add-debug-content-discovery-commands.json) e
  [replay](../changes/2026-10-31-connect-shared-replay-commands.json).
  pending null dopo cancel è provato; cancel/deadline replay sono su slow child
  sintetico, non interruzione di frame guest. Restano procedura assistente e
  parità complete, live slow-refresh e limiti di presentazione stretta.


- [ ] **DBG2-16 — Ricette ripetibili e batch delimitati.**
  Salvare dalla console una sequenza di comandi tipizzati, selezionabile in
  Script e strumenti: parametri, versioni/pin, attese per esito, assert e stop
  al primo errore. Limiti espliciti di passi, frame, durata e output; anteprima
  prima dell'avvio e cancellazione anche tra comandi. Nessun loop illimitato.
  Le ricette che riusano prove concluse richiedono un motivo di riapertura.
  **Chiusura:** una ricetta breve e una interrotta mantengono ordine, identità,
  log e checkpoint coerenti; riprendere non ripete automaticamente operazioni
  già riuscite. **Dipende da:** DBG2-06/15, DBG-21.

  **Audit 2026-11-01 — non iniziato nel contratto console.** [Workbench](../../crates/recomp/src/debug_workbench.rs)
  e [libreria dichiarata](../changes/2026-10-30-add-declared-debug-script-library.json)
  hanno già ricette readonly status/inspect delimitate (base DBG2-06).
  Il [parser console](../../crates/recomp/src/debug_command.rs) non salva/esegue
  batch tipizzati con assert, attese e checkpoint; l'editor conserva multilinea
  come bozza. Mancano serializzazione console, ripresa senza riesecuzione e
  prove breve/interrotta richieste qui. Non duplicare il runner host esistente.


## P3 — Menu cheat utile alla preparazione del debug

- [ ] **DBG2-17 — Import locale del catalogo e stati di compatibilità.**
  Leggere il catalogo esistente preservando CH, ordine completo dei blocchi,
  formato/regione dichiarati, alias, note, provenienza e master della fonte.
  Costruire un indice runtime separato per ROM esatta/versione dispositivo,
  decoder e verifica; l'originale resta consultabile senza riscriverlo.
  Per default tutto disattivato: sconosciuto, non supportato e non verificato
  hanno stati distinti, con motivazione visibile.
  **Chiusura:** le 341 voci sono consultabili dove il catalogo locale è presente;
  una copia senza cheats mostra elenco vuoto e Importa esplicito. Nessun
  payload di terzi entra nei pacchetti pubblici. **Dipende da:** DBG2-02/03.

  **Audit 2026-11-01 — parziale, import integrato e verificato.** [Catalogo](../../crates/recomp/src/debug_cheats.rs)
  preserva ordine/provenienza/master; [indice CLI](../changes/2026-10-31-index-local-cheat-catalog-readonly.json),
  [menu](../changes/2026-10-31-browse-local-cheat-catalog-in-menu.json),
  [target/guida/esclusione pacchetti](../changes/2026-10-31-describe-cheat-target-and-public-guide.json)
  e [canale locale](../changes/2026-10-31-expose-cheat-snapshot-local-control.json)
  hanno prove 341 voci/empty/import/query/stato invariato. Hash EUR/USA significa
  confronto regione dichiarata, non compatibilità. Manca l'indice runtime di
  decoder/verifica per dispositivo/ROM: Catalog::summary/menu_entries dichiarano
  ancora in modo fisso decoder unsupported e nessun handler implementato,
  malgrado inspect_source e prepare_once/apply_once esistano e abbiano prove.
  Versione dispositivo ignota; distinguere capacità e verifica del comportamento. I sette
  source/master statici non verificano tutte le voci o il loro comportamento.


- [ ] **DBG2-18 — Decoder dei formati e operazioni effettive.**
  Partire dal sottoinsieme EUR Codebreaker documentabile; definire operazioni
  tipizzate per scritture, condizioni, ripetizioni e continuazioni supportate.
  Verificare larghezze, allineamento, endianness, master/enable e ordine.
  Versioni/cifrature GameShark/Action Replay ignote restano non eseguibili fino
  a decodifica e prove dedicate; non trattare le stringhe come scritture RAM.
  Placeholder richiedono parametri validati, mai sostituzioni indovinate.
  **Chiusura:** fixture pubbliche sintetiche verificano ogni opcode supportato
  e rifiuto atomico dei blocchi incompleti/sconosciuti; ciascun CH candidato
  riporta esattamente ciò che può eseguire. **Dipende da:** DBG2-17.

  **Audit 2026-11-01 — sottoinsieme verificato, padre parziale.** [Decoder](../../crates/recomp/src/debug_cheat_decoder.rs)
  implementa write/modify/condition/slide delimitati con rifiuto atomico;
  [plaintext](../changes/2026-10-31-inspect-assumed-plaintext-codebreaker.json),
  [seed esplicito](../changes/2026-10-31-inspect-explicit-seeded-codebreaker.json) e
  [source/master](../changes/2026-10-31-inspect-source-bound-codebreaker-masters.json)
  registrano fixture sintetiche e confronto mGBA/CRC per sette priorità EUR.
  Manca il raccordo per CH tra capacità realmente eseguibili e indice che
  dichiara ancora tutti i decoder unsupported (criterio «ciascun CH candidato
  riporta esattamente ciò che può eseguire»). Opcode/formati ignoti e placeholder
  possono restare rifiutati come previsto dal piano; il sottoinsieme già
  verificato non va ampliato soltanto per dichiarare chiusura. Il manual one-shot successivo è distinto
  dall'ispezione non eseguibile. Non implementare un secondo decoder.


- [ ] **DBG2-19 — Applicazione controllata nella copia di debug.**
  Riutilizzare la sessione isolata: armare, applicare una volta, mantenere
  attivo e disarmare sono operazioni diverse, con timing deterministico.
  Mostrare operazioni/aree interessate prima dell'attivazione e rilevare
  sovrapposizioni tra cheat. Le scritture seguono il percorso memoria corretto,
  inclusi eventuali effetti I/O; non usare accessi grezzi per aggirarlo.
  Bloccare ROM/cache native non supportate o gestirne esplicitamente
  invalidazione e backend; nessuna dichiarazione di compatibilità implicita.
  **Chiusura:** abilitare/disabilitare e applicare durante pausa/passi ha effetto
  definito, errore senza blocco parzialmente applicato e nessuna scrittura sulla
  partita originale. **Dipende da:** DBG2-18, DBG-02/10.

  **Audit 2026-11-01 — parziale.** [One-shot](../changes/2026-10-31-apply-guarded-cheat-once.json)
  e handler CheatPrepare/CheatApply in [play.rs](../../crates/recomp/src/play.rs)
  verificano source/stato/ROM e preflight prima delle scritture bus; prove
  sintetiche Wine e CH-0334 delimitato. Mancano mantenimento arm/disarm,
  timing periodico e conflitti; backend nativo, I/O write, ROM e mirror sono
  rifiutati. Disattivazione e ripristino checkpoint restano operazioni distinte.


- [ ] **DBG2-20 — Pagina Cheats con valori, gruppi e profili.**
  Ricerca per CH/nome/categoria, filtri ROM/formato/stato, descrizione dell'uso
  debug, target effettivo e note; selezione dei valori da elenco/range ammesso.
  Toggle per codice verificato, Applica una volta, Disattiva tutti, preset
  nominati e indicatore sempre visibile dei cheat attivi.
  I master sono dipendenze del relativo decoder, non codici arbitrari scelti
  automaticamente. Ogni voce non eseguibile spiega cosa manca.
  **Chiusura:** svolgere il flusso interamente da menu/controller o comando
  testuale equivalente, senza picker implicito; conflitti e codici modificati
  impediscono un'attivazione silenziosa. **Dipende da:** DBG2-04/15/19.

  **Audit 2026-11-01 — parziale.** La [vista catalogo](../changes/2026-10-31-browse-local-cheat-catalog-in-menu.json)
  offre ricerca/dettagli/filtri readonly; comandi prepare/apply-once non sono
  toggle nella pagina. Mancano valori/gruppi/profili, attivi/disattiva tutti,
  conflitti e flusso controller completo; attendere DBG2-19/15, conservando la
  vista integrata invece di rifarla.


- [ ] **DBG2-21 — Prima selezione EUR utile al debug.**
  Priorità, subordinata alla decodifica/verifica: accesso a Card Shop/Item Shop/
  Monster Bank (CH-0323/0324/0325) per raggiungere UI e transizioni; fondi/risorse
  (CH-0331/0334/0336) per ripetere azioni; HP/MP/statistiche (CH-0321) per
  mantenere vivo il caso. CH-0321 è un blocco combinato: non presentarlo come
  semplice HP senza provare una variante distinta. Riconoscere il target
  osservato, senza equiparare «primo personaggio» a un'identità globale.
  **Chiusura:** almeno un accesso UI e un modificatore di risorse hanno prova
  isolata di attivazione, effetto, disattivazione e ripristino; copertura di ogni
  altro CH indicata singolarmente, nessun intero gruppo marcato verificato.
  **Dipende da:** DBG2-19/20.
  Avanzamento: CH-0334 applicato una sola volta in una copia EUR isolata Wine;
  entrambe le scritture RAM corrispondono al piano e il riavvio dal seed
  ripristina lo stato. Effetto visibile, toggle/disattivazione e prova UI
  restano aperti; [dettagli](../cheats.md#manual-one-shot-in-an-isolated-session--dbg2-19-in-progress).

  **Audit 2026-11-01 — parziale.** [Source/master statico](../changes/2026-10-31-inspect-source-bound-codebreaker-masters.json)
  copre sette priorità, [applicazione](../changes/2026-10-31-apply-guarded-cheat-once.json)
  include CH-0334: due valori RAM e ripristino esatto dal seed. Il risultato
  locale target/debug-tools-v2-cheat-eur-resource/result.json è stato riletto:
  nessuna prova di effetto visibile/gameplay, toggle/disattivazione o accesso UI.
  Restano tutti questi criteri, subordinati a 19/20; non promuovere il gruppo.


- [ ] **DBG2-22 — Cheat avanzati per abilità, regole e transizioni.**
  Pianificare a gruppi piccoli abilità/AP/livello (CH-0320/0330/0340), regole
  battaglia/carte (CH-0338/0339) e fine missione/vittoria (CH-0329/0341).
  Solo dopo valutare sblocchi globali, missioni completate, inventario/mostri,
  statistiche clan e velocità presenti negli altri CH EUR.
  La nota di Clear Mission manca nella fonte: conservare l'incertezza.
  Annotare effetti persistenti e shortcut che saltano la scena da diagnosticare.
  **Chiusura:** per ciascun CH scelto, stato prima/dopo ed effetto collaterale
  controllati; gli altri restano apertamente non verificati. Una vittoria
  forzata non è prova della sequenza ordinaria. **Dipende da:** DBG2-21.

  **Audit 2026-11-01 — non iniziato.** Le voci nel catalogo e il decoder
  generico non sono prove di abilità/AP/regole/transizioni. Nessun record di
  accettazione prima/dopo/effetti collaterali dei gruppi richiesti; DBG2-21 è
  ancora aperto. Conservata l'incertezza originale della nota Clear Mission.


- [ ] **DBG2-23 — Cheat, checkpoint, replay e provenienza.**
  Registrare set/hash/parametri dei cheat, decoder, ordine e frame di ogni
  applicazione in snapshot/profili/registrazioni/report compatibili.
  Definire Load/Undo/reset/cambio ROM e replay con set differente: ripristino
  esplicito o rifiuto motivato, mai toggle persistente applicato per sorpresa.
  «Ripristina checkpoint» resta distinto da «Disattiva tutti».
  Gestire eventuale versionamento dello stato mantenendo la compatibilità
  prevista; non salvare puntatori host nel guest.
  **Chiusura:** una nuova breve registrazione con cheat riproduce stato/clock
  esatti; un set incompatibile viene rifiutato. Il report segnala la variante,
  gli originali restano invariati e nessun caso DS ordinario viene promosso.
  **Dipende da:** DBG2-19/21, DBG-04/05/16.

  **Audit 2026-11-01 — non iniziato nel contratto di provenienza.** Il
  [one-shot](../changes/2026-10-31-apply-guarded-cheat-once.json) rifiuta nuove
  registrazioni/save dopo applicazione (guard last_cheat_once in play.rs).
  [Recording](../../crates/recomp/src/debug_recording.rs) e [replay](../../crates/recomp/src/debug_replay.rs)
  non registrano set/decoder/ordine/frame né provano replay cheat stato-clock,
  Load/Undo/reset/ROM e rifiuto set incompatibile. Il rifiuto preventivo non
  equivale alla funzionalità; nessuna nuova registrazione creata nell'audit.


## P4 — Ulteriori potenziamenti e chiusura

- [ ] **DBG2-24 — Editor di trigger e punti di arresto nominati.**
  Gestire dal menu/console più definizioni salvate di break/watch/capture:
  nome, enabled, PC/modo, range/valore, numero di hit, finestra di frame e azione
  supportata (pausa, cattura, freeze eventi). Consentire combinazioni semplici
  dichiarate; mostrare granularità interprete/native e conflitti tra observer.
  **Chiusura:** un trigger raggiunto e uno non raggiunto rispettano limiti,
  identità e confine sicuro, con condizione valutata spiegabile nel report.
  **Dipende da:** DBG2-05/15, DBG-08/11.

- [ ] **DBG2-25 — Valori osservati e differenze fissati a schermo.**
  Consentire pin di registri, finestre di memoria readonly e campi di
  task/script/unità già decodificati, con label/owner e valore precedente.
  Rinfrescare a cadenza limitata o al passo; invalidare indirizzi/contesto dopo
  Load/reset/cambio ROM. Colori e filtri evidenziano i cambi senza inventare
  identità persistenti o leggere registri con effetti I/O.
  **Chiusura:** spiegare un cambiamento noto tra due punti con fonte per campo,
  stato completo invariato dall'ispezione e contesto obsoleto riconoscibile.
  **Dipende da:** DBG2-03/15, DBG-11/17.

- [ ] **DBG2-26 — Timeline del problema e ricetta esportabile.**
  Collegare selezione menu → comando/script → evento guest osservato →
  cattura/differenza/errore, con bookmark e accesso al report corrispondente.
  Aggiungere al report issue ricetta, ID selezionati e cheat eventualmente
  attivi; copia del comando ripetibile e distinzione tra contesto disponibile
  e collegamento causale effettivamente provato.
  **Chiusura:** utente e assistente ricostruiscono lo stesso caso senza cercare
  manualmente percorsi; eventuali buchi/scarti e allegati privati sono espliciti.
  **Dipende da:** DBG2-11/16/23, DBG-09/16.

- [ ] **DBG2-27 — Verifiche mirate di integrazione e regressione.**
  Esercitare nuova selezione, console off/on, macro e cheat con fixture
  sintetiche pubbliche e pochi casi reali dichiarati. Testare tastiera/mouse/
  controller, dialogo solo da pulsante, cap, cancel, chiusura, errori e cartella
  spostata; confrontare gli stessi comandi da menu/console/canale locale.
  Riusare le prove DBG già concluse scegliendo nuovi controlli solo per il
  comportamento modificato. Cheat attivi e diagnostica readonly hanno attese
  di equivalenza diverse, dichiarate prima del test.
  **Chiusura:** matrice Wine/Windows nativo/Linux e controller simulato/fisico
  con esiti effettivi, originali protetti, nessun errore scambiato per successo
  o ripetizione automatica di tutti i casi. **Dipende da:** incrementi selezionati
  DBG2-04…26; iniziare i controlli insieme a ciascun incremento.

- [ ] **DBG2-28 — Guida, distribuzione e compatibilità delle versioni.**
  Aggiornare help e guida con percorsi completi da menu e console, comandi
  realmente supportati, prerequisiti degli strumenti host e migrazione degli
  indici locali. Distribuire soli adattatori/schemi/fixture pubbliche; catalogo
  cheat di terzi, macro personali, cronologia e dati di gioco restano esclusi.
  Aggiornare la whitelist del packager e i contesti Docker solo per i nuovi
  file necessari; dichiarare le capacità della build e del canale locale.
  **Chiusura:** bundle pulito, empty state comprensibili, comandi dalla cartella
  spostata e log accessibili; una versione vecchia riceve incompatibilità chiara.
  Checklist/record/stato/handoff descrivono solo ciò che è stato provato.
  **Dipende da:** DBG2-27 per le funzioni distribuite.

## Riferimenti e anomalie dei cheat da conservare

Per i contenuti usare localmente cheats/README.md, cheats/catalogo.json,
cheats/controllo-qualita.md e cheats/tabelle-modificatori.md. La documentazione
versionata riporta ID e categorie utili, senza copiare blocchi di codici.

Restano da risolvere per le rispettive voci: master diversi delle fonti CB/XP,
etichette P3/P4 di CH-0048/0049, possibile refuso di CH-0236, placeholder e
versioni dispositivo ignote. Le 25 sezioni EUR vuote nella pagina salvata sono
contenuti mancanti, non cheat importati; non inventarli né scaricarli
automaticamente. Le istruzioni di sblocco senza codice dispositivo restano
guide, non toggle eseguibili. JPN e regioni ignote possono essere consultate
anche quando non sono supportate dalla ROM selezionata.

## Ordine di lavoro e regola della spunta

Primo gruppo: **DBG2-01 → 02 → 03 → 04 → 05**, così gli strumenti esistenti
diventano utilizzabili senza picker obbligatori. Poi libreria/persistenza
**06/07**, console eventi **08…12**, console comandi **13…16** e cheat **17…23**.
L'import readonly **17** può iniziare dopo il selettore; l'attivazione aspetta
decoder e isolamento. Gli ulteriori potenziamenti **24…26** riusano questi
contratti. **27/28** accompagnano ogni incremento distribuibile.

Ogni spunta richiede implementazione, flusso utente, prova positiva e negativa,
limiti/cap dichiarati, conservazione degli input e record di memoria. Se una
voce comprende più CH o backend, registrare i sottoesiti: un parent incompleto
non riapre i figli conclusi. Eseguire i controlli nuovi necessari, non vecchie
campagne per ricostruire il contesto. Registrare la selezione effettiva nel
[TASK_HANDOFF.md](TASK_HANDOFF.md) secondo il [workflow](MEMORY_WORKFLOW.md).
La creazione di questa checklist non avvia automaticamente nessuna voce.
