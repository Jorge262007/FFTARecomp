# TODO — Bug fix e funzionalità desktop

Analisi e avanzamento del 21 ottobre 2026, basati sul codice e sulla cattura
fornita dall'utente (immagine rimossa dalla storia Git dopo la diagnosi).
Le caselle distinguono il lavoro verificato dalle attività ancora aperte;
le proposte delle sezioni successive restano da sviluppare.

Ambito: workspace desktop, con priorità di verifica su Windows.
`AndroidPort/` è un albero separato. Per riprendere lo sviluppo consultare
[TASK_HANDOFF.md](TASK_HANDOFF.md): riutilizzare le verifiche già concluse nel
[PERFORMANCE_TODO.md](PERFORMANCE_TODO.md), senza riaprire esperimenti scartati.

**BUG-01-LAWS** è corretto per il layout osservato; **BUG-02-FPS** è spiegato
per il caso Fast confermato. **BUG-03-FAST-AUDIO** ha reset, consumo effettivo
e Normal limitato; restano ascolto/eventi/hardware. **FEAT-01-MUTE** e
**FEAT-02-FULLSCREEN** sono implementati, con limiti di verifica espliciti.
**FEAT-03-TACTICAL-OVERLAY** resta aperto (P3, prima dati verificati e
visualizzazione, poi previsione). L'ultima richiesta aggiunge il piano
**FEAT-04-ROM-DATA**: estrazione, decodifica e uso diretto dei dati, poi mod.
La richiesta attuale riguarda il TODO; l'implementazione resta da avviare.
Un genitore aperto non riapre figli conclusi.

## BUG-01-LAWS — Schermata delle leggi in battaglia

**Stato:** correzione implementata e verificata per il layout osservato;
resta aperta la copertura estesa sotto. **Priorità:** P1. **Complessità:** media.

**Causa verificata.** La tabella leggi usa BG2=5C08/BG3=4A09, come il menu
azioni, ma non attiva WIN0 per l'aiuto. Il vecchio riconoscimento `Notice`
non la ammetteva. Senza HUD a bordo schermo la separazione era saltata;
con HUD poteva scattare il layout azioni e il ripiego a X=160, producendo
il contenuto diviso e il pannello destro vuoto della segnalazione.

**Soluzione implementata.** `laws_bounds` in
[ppu.rs](../../crates/gba-core/src/ppu.rs) riconosce cornice, righe e divisore
della tabella, senza leggere le parole tradotte. Riutilizza il livello
centrale `Notice` per tabella e aiuto, comprensivo di selettore e indicatore
di continuazione; l'HUD restante rimane separato. Il sottotipo host
`laws_layout` in [ffta_dialogue.rs](../../crates/gba-core/src/ffta_dialogue.rs)
è ricalcolato a ogni frame e non viene serializzato. La geometria completa
non subisce Length o il clamp dei pannelli laterali. Restano i controlli
esistenti di ROM, terreno e registri; firme incomplete conservano il vecchio
percorso. Vedi [documentazione e verifiche](../ffta_laws_layout.md).

- [x] Riprodurre lista e aiuto con ROM EUR e salvataggi locali, in italiano
  e inglese; osservare BG, finestre e sprite. Conservare dati e catture sotto
  `target/laws-fix/`, ignorato da Git.
- [x] Correggere classificazione, proprietà dei pixel e dei due indicatori,
  riutilizzando layout e modalità mouse dei pannelli modali.
- [x] Verificare testo su entrambi i lati del vecchio taglio, HUD presente o
  assente, tre larghezze, ricomposizione nativa esatta e VRAM/OAM/framebuffer
  invariati; la regressione fallisce disattivando il riconoscimento.
- [x] Verificare 72 casi da snapshot: lista/aiuto, 3:2/16:9/ultrawide, zoom
  1 e 2, Size 0,5/1/2 e Length 1/2. Ricomposizione nativa: zero pixel diversi.
  Parità GPU/software: zero differenze in 24 casi sintetici, incluso resize
  fra dimensioni pari/dispari e variazioni di Size.
- [x] Eseguire suite core e runtime native, suite core Windows/Wine e
  regressioni finali sui byte degli snapshot; ispezionare lista/aiuto nel
  frontend Linux e Windows/Wine e verificare caricamento e uscita normale.
- [ ] Completare la copertura aggiuntiva: Windows nativo, ROM USA, altre
  disposizioni/quantità di leggi, scena tutorial esatta della cattura,
  zoom oltre 2 e un ciclo load/Undo specifico nella schermata leggi.

**Criterio di chiusura:** il layout verificato mantiene lista, testo e
indicatori integri e coerenti con il framebuffer originale. Le prove
aggiuntive elencate restano esplicite e non riaprono la correzione già
validata. Non sono stati aggiornati i pacchetti portable.

## BUG-02-FPS — Differenza tra framerate interno ed esterno

**Stato:** diagnosi conclusa per il caso segnalato: l'utente conferma la
differenza solo con Fast attivo; il comportamento è previsto. Spiegazione
aggiunta alla voce FPS del menu. **Priorità:** P1 per la diagnosi conclusa.

**Evidenza e analisi.** [fps.rs](../../crates/recomp/src/fps.rs) conta i frame
guest completati (**INT**) e le presentazioni host riuscite (**EXT**) sul tempo
reale. EXT non misura il refresh fisico del monitor. Il ciclo in `play.rs`
può simulare più frame prima di presentarne uno: in fast-forward la differenza
è quindi prevista. A 4×, se il carico lo consente, circa 239 INT e 60 EXT
sono coerenti come esempio teorico. Le catture locali a 4× mostrano
128,9 INT / 32,2 EXT e 130,0 / 32,5, tornando a circa59,7 / 59,7 senza Fast.
Il rapporto resta4:1 anche quando il PC non raggiunge239 INT. Anche pacing
audio a blocchi e presentazioni saltate possono separare i due contatori.

La cattura allegata mostra **59,7 INT / 59,7 EXT**: documenta il difetto grafico,
non la divergenza segnalata. Esistono già test del contatore,
[documentazione FPS](../fps_overlay.md) e diagnostica di cadenza completata in
`PERF-10-PRESENTATION-CADENCE`. Il pacing `clock` è già predefinito su
Linux/Windows; non va proposto come funzionalità mancante.

**Soluzione implementata.** La riga Video > FPS counter spiega simulazione e
presentazioni e indica che in Fast le frequenze possono differire. Il caso
segnalato non richiede correzioni al pacing o al clock GBA.

- [x] Precisare il caso: l'utente conferma «Solo con Fast attivo». Le catture
  e i contatori delle prove audio4× esistenti mostrano il rapporto4:1 atteso.
- [x] Riutilizzare test del contatore e prove PERF-10/audio già concluse:
  frame multipli per presentazione, reset pausa e presentazioni saltate;
  nessuna nuova matrice di cadenza necessaria per questa segnalazione.
- [x] Aggiungere una spiegazione accessibile da Video > FPS counter:
  «INT: simulazione; EXT: presentazioni; in Fast possono differire».
- [x] Chiudere la diagnosi del caso previsto con spiegazione e catture.
  Non è stata segnalata una divergenza a 1× e non si forza l'uguaglianza.
- [x] Riutilizzare il test indipendente Fast/reset pausa/frame non presentati.
  Confronti VSync/refresh sul dispositivo e Windows nativo restano copertura
  hardware generale in PERF-10, non una correzione richiesta da questo caso.

**Criterio di chiusura:** ogni divergenza riprodotta ha una spiegazione
misurabile; quelle patologiche sono corrette senza rallentare la simulazione
o alterare la definizione dei contatori. L'uguaglianza numerica non è un requisito.

## BUG-03-FAST-AUDIO — Audio fuori sincrono in modalità Fast

**Stato:** caso segnalato precisato (Fast audio, 4×); reset delle transizioni e
consumo sulla produzione effettiva e limite del backlog Normal implementati.
Restano aperti ascolto e copertura hardware/eventi.
**Priorità:** P1. **Complessità:** medio-alta. Collegato a PERF-09/10.

**Evidenza e analisi.** `AvConfig` e il menu Fast forward espongono già
`silent`, `fast` e `normal`. `set_fast_forward` svuotava già le code al cambio
modalità, ma lasciava attive le memorie dei filtri e l'ultimo campione nel
callback. La regressione sintetica conferma l'uscita residua a code vuote.
Il nuovo epoch host invalida anche questi stati; pausa e perdita del device
scartano inoltre il backlog Normal. Vedi [comportamento e prove](../fast_forward_audio.md).
`fast` conserva il moltiplicatore selezionato come limite richiesto. Il callback
in [main.rs](../../crates/recomp/src/main.rs) ora ne adatta il consumo alla
produzione misurata dal clock del device e alla riserva in coda; il frontend
continua a eseguire il gruppo di frame indipendentemente dall'audio.

`normal` usa invece [NormalAudio](../../crates/recomp/src/ff_audio.rs): riproduce
a 1× il PCM prodotto dal gioco accelerato. Il vecchio file temporaneo non
aveva un limite di durata o dimensione; ora una coda preallocata limita il
PCM a 2 secondi/512 KiB e salta all'audio recente con fade quando è piena.
Questo è distinto dal difetto di `fast`: se il PC
non sostiene il moltiplicatore richiesto, consumo nominale e produzione reale
possono divergere. Il caso dell'utente riguarda Fast audio a 4×, non Normal.
Nella prova locale Linux/CPAL/llvmpipe, Faithful ed Enhanced raggiungono solo
circa 122–128 INT: tutti i 497/499 callback Fast hanno holds, con zero drop
del produttore e circa 47% delle richieste PSG/HLE vuote. Il consumo nominale
supera quindi la produzione; a 1× gli intervalli playing non hanno holds.
Hold/release, toggle e pausa/ripresa sono stati eseguiti con catture ispezionate.
Volume zero e scena inattiva non provano la qualità sonora né il ritardo di un
evento sul dispositivo dell'utente. Il solo reset non chiudeva questa seconda
causa: è stata quindi aggiunta la correzione del consumo descritta sotto.

**Soluzione implementata per Fast.** `fast_audio_rate::Clock` stima la produzione
su circa200ms, corregge la riserva con risposta250ms e varia un consumo comune
ai cinque percorsi su circa100ms. Riserva obiettivo60ms oltre al buffer del
callback, intervallo0,05×..moltiplicatore+2%; a 1× resta il vecchio controllo.
Il filtro sinc usa cutoff frazionari continui; priming/riempimenti emettono
silenzio esplicito, contato da `FFTA_FAST_AUDIO`, senza ripetere campioni vecchi.
Nessun nuovo lock, I/O, allocazione o clock host nel callback.

**Soluzione implementata per Normal.** Menu «1× ritardato», coda da 2 secondi,
salto agli ultimi 250 ms con fade stereo di 10 ms e coda di transizione fissa.
Nessun file temporaneo né I/O nel callback. Il budget riguarda il PCM host,
non la latenza aggiunta dal backend. Un'eventuale
modalità accelerata che preservi il pitch richiederebbe time-stretch dedicato:
è una scelta ulteriore, non una proprietà della coda 1×.

- [x] Identificare modalità e velocità della segnalazione: l'utente conferma
  **Fast audio a 4×**, sospettando riproduzione di audio arretrato.
- [ ] Riprodurre a 2×/4×/8× con Faithful/Enhanced, transizioni hold/toggle e un
  evento audio-visivo riconoscibile. Misurare ritardo iniziale e sua evoluzione.
- [ ] Riutilizzare i profili audio/cadenza esistenti: correlare frame guest,
  consumo, riempimento, holds/drops e tempo dei callback; tenere distinti
  priming, pausa e Fast. Non usare un benchmark senza callback come prova sonora.
- [x] **BUG-03-EFFECTIVE-RATE:** correggere il consumo nominale quando la
  velocità richiesta non è raggiunta: clock comune smussato, riserva, cutoff
  frazionario e priming osservabile. Regressione callback4×/produzione2×,
  matrice72casi a2×/4×/8×, cambi di produzione/arresto/ripresa e filtraggio.
  Nessuna modifica al tempo guest o alla serializzazione.
- [x] Verificare il caso4× con callback Linux reali e finestra guest9400:11400:
  Faithful357/Enhanced359callback, zero holds/PSG-HLE empty/drops/riempimenti
  e zero silenzio di priming. Consumo medio2,2032/2,1906× segue produzione
 2,2030/2,1906×; coda stimata massima107,3/108,0ms. Volume zero e scena
  inattiva: non è una misura di latenza sonora su Windows nativo.
- [x] **BUG-03-NORMAL-BOUNDS:** budget 2 secondi/512 KiB, salto agli ultimi
  250 ms e fade stereo di 10 ms. Menu esplicito, consumo 1× Faithful anche
  con Enhanced configurato, reset invariati, nessun file/I/O nel callback.
  Suite native78runtime/33menu e Wine79runtime/33menu, build senza warning;
  callback Linux reali365/376, massimi130287/130783coppie sotto131072,
  10salti per modalità senza holds/drop/over-period. Persistenza, pausa,
  ritorno a 1×, slot invariati e uscita verificati; volume zero.
- [x] **BUG-03-TRANSITION-RESET:** invalidare anche DAC, interpolatori, filtri e
  crossfade all'ingresso/uscita Fast e al cambio moltiplicatore/modalità;
  svuotare l'audio in pausa e alla perdita del device. Load/Undo conserva la
  ricreazione dello stream. Regressioni su uscita residua e nuovi campioni
  confrontati con un callback appena inizializzato, native e Windows/Wine.
- [ ] Completare ascolto e misure sul dispositivo: hold/toggle, pausa, load/Undo
  e riconnessione fisica; il reset host non ritira i campioni già nel backend.
- [x] Test sintetici di consumo, limite del backlog, fade e transizioni:
  6000tick a 2×/4×/8× senza crescita di capacità e callback osservato/non
  osservato identico, con uscita sintetica udibile e consumo1× senza holds.
- [ ] Verificare su Windows con audio reale. Fissare prima del confronto una soglia
  di latenza coerente con il buffer del device e controllare che non cresca
  durante una sessione prolungata; verificare stabilità di memoria e file.

**Criterio di chiusura:** `fast` non accumula deriva rispetto al video nel caso
riprodotto, e il ritorno a 1× non trascina audio arretrato. `normal` ha ritardo
dichiarato e limitato; `silent` resta silenzioso. Nessun cambio al mix guest,
alla serializzazione o ai timer è giustificato dalla sola segnalazione.

## FEAT-01-MUTE — Comando Mute nel menu Audio

**Stato:** implementato e verificato nei limiti sotto. **Priorità:** P2.

**Soluzione implementata.** Audio > Mute On/Off salva `audio.mute`, predefinito
Off per le configurazioni vecchie. Volume resta indipendente e può cambiare
mentre Mute è attivo. Il callback applica l'unione del mute utente e di quello
temporaneo Fast Silent/fallback all'uscita finale, continuando il consumo;
la rampa esistente chiude a zero la coda sotto -100 dB. Configurazione audio
condivisa fra avvio e modifiche live: anche il volume salvato si applica subito.
Gli edit solo audio non ricreano la storia video. Nessun cambio a input guest,
timer, formato dei salvataggi o pacing. Vedi [controlli audio](../audio_controls.md).

- [x] Default, parsing, serializzazione e compatibilità di `audio.mute` in
  AvConfig, con percorsi installed/portable invariati.
- [x] Riga Mute, valore e gestione menu condivisa tastiera/gamepad/mouse;
  modifiche volume durante Mute e ripristino al livello scelto.
- [x] Separare stato utente e temporaneo; applicare con rampa senza fermare
  consumo/stream/pacing. Unmute non annulla Fast Silent/fallback; i reset
  pausa/Fast/load conservano la scelta utente.
- [x] Regressione callback Faithful/Enhanced/Fast/Normal: code e consumo uguali
  al riferimento, rampa verificata, silenzio esatto a regime, uscita sintetica
  udibile al ritorno e volume modificato conservato. Native27config/34menu/
 79runtime e Wine27/34/80 passano,4runtime opzionali ignorati per target.
- [ ] Ascolto su Windows nativo e controller fisico restano copertura hardware;
  i test menu condivisi e Wine non costituiscono quella verifica.

**Criterio di chiusura funzionale soddisfatto:** mute accessibile e persistente,
volume conservato, silenzio a regime e consumo invariato. Volume0 resta distinto.

## FEAT-02-FULLSCREEN — Fullscreen On/Off nel menu Video

**Stato:** implementato e verificato su Linux e Windows/Wine; copertura fisica
separata. **Priorità:** P2. **Complessità:** bassa-media.

**Soluzione implementata.** Video > Fullscreen (F11) e F11 condividono il setter
borderless e `video.fullscreen`, default Off e applicato al successivo avvio.
Il controller riconcilia lo stato riferito da winit, attendendo fino a 2 secondi
le richieste asincrone, senza leggere clock quando non c'è una richiesta pendente.
Winit/WM conservano geometria e massimizzazione; un fallback riposiziona la
finestra se la vecchia area non interseca più un monitor disponibile.

Il test reale ha inoltre trovato e corretto una corsa al resize del menu:
upload GPU e lettura software devono usare le dimensioni del canvas già
disegnato, non quelle di una finestra ridimensionata nel frattempo. Vedi
[comportamento, prove e limiti](../fullscreen.md).

- [x] Configurazione default/vecchi file/roundtrip, riga Video, navigazione,
  mouse e scorrimento fino a Back dopo l'aumento a 11 voci.
- [x] Unica richiesta startup/F11/menu; auto-repeat ignorato, preferenza e voce
  sincronizzate con lo stato winit. Risposta ritardata/rifiuto/cancellazione
  testati; cambiamenti esterni non esposti dal backend restano un limite API.
- [x] Ripristino finestra640x360 con coordinate invariate e geometria normale/
  massimizzata verificati su X11/Openbox e Windows/Wine. Test puro su monitor
  rimosso, coordinate negative e limiti della finestra; hotplug fisico aperto.
- [x] Riutilizzare resize/DPI/cursore/presenter, senza ricreare macchina guest,
  storia video o audio, né cambiare aspect/zoom/velocità. Correggere la corsa
  fra canvas e finestra rilevata dal test, preservando la prova negativa.
- [x] Menu e F11 durante pausa, pressione prolungata singola, riavvio fullscreen,
  VSyncOn, minimize/restore fullscreen e ritorno windowed. Suite native28config/
 35menu/81runtime e Wine28/35/82,4runtime opzionali ignorati per target; build
  senza warning, slot invariati e uscite normali. Matrice PERF-04 riutilizzata.
- [ ] Windows nativo, controller fisico, Wayland/macOS, hotplug/DPI/refresh e
  Alt-Tab fisico restano copertura hardware; Wine usa il frontend interprete.

**Criterio funzionale soddisfatto:** menu, F11 e stato riferito dalla finestra
concordano; entrata/uscita e ripristino preferenza funzionano senza riavviare
il gioco o perdere la geometria della sessione. Nessun pacchetto aggiornato.

## FEAT-03-TACTICAL-OVERLAY — Riepilogo battaglia e previsione turni

**Stato:** da progettare; fonti dei dati ancora da verificare.
**Priorità:** P3. **Complessità:** alta, soprattutto per la previsione.

**Richiesta preservata.** Quadrato mostra leggi, numero di turni, missione e
ordine dei turni; la pressione prolungata rende l'overlay fisso. Prima della
conferma di un'azione l'overlay fisso mostra eventuali modifiche all'ordine.

**Evidenza e analisi.** Il progetto ha binding host, livelli UI e profili ROM
EUR/USA, ma nei percorsi ispezionati non emerge un lettore verificato che
esponga insieme questi dati o un predittore dell'ordine. La schermata leggi
può aiutare a confrontare i valori, ma i pixel non sono una fonte strutturata.
Il token fisico da valutare per Quadrato è `West` (posizione X su Xbox);
l'attuale UI dei binding usa etichette Xbox. Serve anche un binding tastiera
e va gestito il conflitto con rimappature già salvate.

**Soluzione proposta, prima fase: dati e overlay consultivo.** Introdurre un
lettore tipizzato e in sola lettura dello stato di battaglia, abilitato per
profili ROM verificati. Esporre al frontend uno snapshot coerente con validità
e provenienza dei dati; renderizzarlo su un livello host indipendente dallo
zoom. Non dedurre indirizzi EUR/USA per semplice analogia.

- [ ] Identificare e verificare leggi attive, missione/obiettivo, unità,
  contatore di turno e dati dello scheduler; definire che cosa conta come
  «turno» rispetto alle attivazioni individuali. Confrontare con schermate e
  sequenze reali; mostrare «non disponibile» per dati non verificati.
- [ ] Progettare il lettore vicino a
  [ffta_rom.rs](../../crates/gba-core/src/ffta_rom.rs) e ai moduli battaglia,
  con controlli di limiti, identità e fase; aggiornare a fine frame/evento
  senza scrivere nella RAM guest o modificare RNG, input e clock.
- [ ] Aggiungere un'azione host rimappabile in Input, con Quadrato/`West` come
  proposta iniziale; risolvere i conflitti senza sovrascrivere i binding GBA.
  Verificare XInput, DirectInput e Gilrs e rendere comprensibile l'etichetta
  per un controller PlayStation.
- [ ] Implementare una macchina a stati per la pressione. Proposta da validare:
  visibile alla pressione, nascosto al rilascio breve; tenere premuto almeno
  400 ms lo fissa anche dopo il rilascio; una nuova pressione breve lo chiude.
  Usare tempo reale, così Fast non modifica la soglia; ignorare auto-repeat.
- [ ] Disegnare leggi, missione, turno e ordine corrente in un pannello
  leggibile che non copra conferme e informazioni essenziali. Riutilizzare
  [ui_layer](../../crates/screen/src/ui_layer.rs) e la composizione software.
  Gestire resize, DPI, fullscreen e testo lungo; mostrare lo stato «fisso».
- [ ] Nascondere durante i menu host e fuori battaglia; su perdita focus,
  disconnessione pad, load/Undo, cambio ROM o fine battaglia azzerare pressioni
  e dati obsoleti. Ricalcolare solo da una scena valida prima di riapparire.

**Soluzione proposta, seconda fase: previsione prima della conferma.**
Prima verificare come il gioco determina l'ordine e quali azioni lo cambiano.
Preferire dati di previsione già preparati dal guest, se esistono; altrimenti
estrarre una funzione pura su uno snapshot. Non eseguire azioni speculative
sulla macchina attiva e non consumare RNG per costruire l'anteprima.

- [ ] Individuare selezione azione/bersaglio, fase di conferma e regole reali
  dello scheduler, inclusi spareggi ed effetti di stato; delimitare le azioni
  per cui una previsione deterministica è possibile.
- [ ] Calcolare ordine corrente e previsto su dati immutabili; evidenziare
  solo gli spostamenti quando l'overlay è fisso. Per esiti casuali o dati
  insufficienti indicare previsione condizionale/non disponibile, senza
  presentare un ordine ipotizzato come certo.
- [ ] Aggiornare l'anteprima al cambio di azione/bersaglio e rimuoverla su
  annullamento; dopo la conferma sostituirla con il nuovo stato autorevole.
- [ ] Confrontare previsione e ordine osservato per azioni che lo lasciano
  invariato e che lo modificano, unità rimosse e stati rilevanti; aggiungere
  regressioni dopo aver verificato le regole, non prima.
- [ ] Confrontare replay overlay acceso/spento: stesso stato guest, RNG,
  frame nativi e audio deterministico a parità di input GBA; misurare il costo
  della lettura/renderizzazione e verificare che la pressione non confermi
  contemporaneamente un'azione di gioco.

**Criterio di chiusura:** prima fase utilizzabile e verificata separatamente;
la funzionalità completa si chiude solo con anteprima attendibile nei casi
supportati, limiti visibili e nessun effetto dell'overlay sull'esecuzione guest.
La correzione BUG-01 aiuta il confronto delle leggi, ma non sostituisce la
verifica delle strutture dati e dello scheduler.

## FEAT-04-ROM-DATA — Estrazione e uso diretto dei dati, poi mod

**Stato:** TODO redatto; implementazione non avviata. **Complessità:** alta.
La priorità interna è estrarre e decodificare i dati della ROM selezionata,
prepararli al primo avvio e farli usare effettivamente dal runtime. Il supporto
alle mod e alle modifiche della logica segue questa consegna principale.

Il [TODO dedicato](ROM_DATA_TODO.md) contiene dipendenze e criteri per:

- [ ] ROMDATA-01 — inventario dei formati, identità e contratti delle risorse.
- [ ] ROMDATA-02 — estrazione e decodifica verificate per famiglia.
- [ ] ROMDATA-03 — preparazione automatica, riuso e ripristino del dataset.
- [ ] ROMDATA-04 — consumo diretto nel runtime, con prova della provenienza.
- [ ] ROMDATA-05 — equivalenza, salvataggi e misure delle prestazioni.
- [ ] ROMDATA-06 — mod come override delle risorse, con conflitti e compatibilità.
- [ ] ROMDATA-07 — estensioni dei contenuti e modifiche alla logica.

**Criterio principale:** dataset realmente utilizzato dal gioco per le famiglie
previste, con estrazione automatica, equivalenza senza mod e dipendenze residue
dalla ROM esplicite. La ROM è già letta in RAM: un vantaggio prestazionale va
misurato. La sola esportazione di file o ricomposizione di una ROM non basta.

## Consegna di ciascuna attività futura

- [ ] Registrare nel handoff l'ID selezionato, ciò che è già concluso e il
  criterio mancante; chiudere ogni sottofase senza riaprirla al successivo resume.
- [ ] Eseguire verifiche mirate al cambiamento; suite core in serie quando
  condividono globali. Annotare separatamente Windows nativo, Wine e Linux.
- [ ] Aggiornare documentazione pertinente e record in `docs/changes/`,
  rigenerare `CHANGELOG.md` ed eseguire `python3 scripts/project_memory.py check`.
  Conservare ROM, salvataggi e registrazioni fuori da commit e pacchetti.
