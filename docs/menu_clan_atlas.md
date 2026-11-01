# Atlante del Clan — menu desktop nativo

Il menu usa il canvas Rust, tiny-skia e i font Cinzel/Inter già incorporati.
La cornice blu minerale e ottone, lo stemma geometrico originale, la sidebar
sinistra e i footer sono indipendenti dallo scroll del contenuto. Il minimo
tecnico del canvas è 480×360 pixel fisici; sotto questa misura appare una
richiesta di ingrandire la finestra con Escape di recupero.

La sidebar mantiene esattamente otto sezioni per scheda. Debug è inizialmente
nascosto e compare solo dopo lo sblocco descritto sotto. **Gioco** contiene
Panoramica, Salvataggi, Fast Forward, Finestre di testo, Audio, Video, Controlli
e Sistema. **Debug** contiene Panoramica, Sessioni e casi, Replay e confronti,
Esecuzione e memoria, Tracce e ispettori, Console e script, Cheat, Report e
archivio. Cambiare scheda conserva sezione, selezione e scroll del contenuto.
Le impostazioni lunghe hanno due colonne quando il contenuto dispone di almeno
750 pixel; le liste Debug hanno dettagli adiacenti, sovrapposti internamente
quando il contenuto è più stretto di 470 pixel. La sidebar rimane a sinistra.
La scala dei font segue il canvas anche da HD a 4K. Catalogo DS, selettore
contenuti, tastiera virtuale e parametri degli strumenti usano la stessa palette
e ridispongono i propri controlli nel corpo disponibile. Le anteprime conservano
i pixel originali; un'anteprima assente è dichiarata esplicitamente.

La Panoramica offre Torna al gioco, slot rapido, Salva/Carica/Annulla, volume
e Fast Forward. Lo slot selezionato rimane tale alla riapertura del menu;
selezionarlo non salva. Un file occupato richiede la conferma già esistente,
inizialmente su Annulla. Un file vuoto non si carica. La data dello stato è
il suo mtime; **Ultima modifica file .sav** è separatamente il mtime del vero
salvataggio cartuccia. Le date sono esplicitamente UTC. File assenti o date
illeggibili sono indicati senza date simulate.

Salvataggi raccoglie Save states, Extended saves (9/99 slot), salvataggi
cartuccia numerati e backup. Le forme e gli handler precedenti conservano
anteprime, conferme e Undo. Sistema raccoglie lingua, riavvio, cambio ROM e uscita.
I binding e i tre profili di input rimangono in Controlli.

Il menu parte in **English**. **Game > System > Language** offre **English** e
**Italiano**, con applicazione immediata alla conferma e persistenza in
`menu.language = en|it` dentro `av.cfg`. Le etichette italiane usate in questa
guida corrispondono alla selezione Italiano. Configurazioni precedenti senza
lingua usano l'inglese; la lingua della ROM resta indipendente. Dettagli e
istruzioni per aggiungere un singolo JSON con registrazione automatica:
[lingua del menu](menu_localization.md).

**Gioco > Video > Trasparenza menu** regola gli sfondi dallo 0% (opachi)
al 100% (trasparenti), in passi del 10%. Il valore iniziale è **10%**. Testi, bordi e
anteprime restano opachi. La preferenza viene salvata come
`menu.transparency = 0.0..1.0` in `av.cfg`, anche in modalità portable.
Un file precedente senza questa chiave usa il 10%; un valore esplicito già
salvato, compreso 0%, mantiene la scelta dell’utente.
Il gioco visibile sotto il menu resta fermo; ogni ridisegno riparte dal frame
congelato, senza accumulare la trasparenza del disegno precedente.

## Sblocco permanente di Debug

Nel menu aperto premere, con pressioni separate delle frecce o della croce:

**↑ ↑ ↓ ↓ ← ← → →**

L'ultima direzione rende visibile Debug e salva **`debug = true`** in `av.cfg`
(`Data/config/av.cfg` in modalità portable). Basta una volta: chiudere il menu,
cambiare ROM o riavviare mantiene lo sblocco nella stessa configurazione.
In alternativa aggiungere direttamente `debug = true` al file e riavviare.
Il valore iniziale è `false`; impostarlo manualmente a `false` nasconde di
nuovo le schermate al riavvio senza cancellare dati o preferenze Debug.

Prima dello sblocco sono assenti la scheda Debug, i controlli previsti Eventi/CMD,
i riferimenti Debug e le opzioni **Extended saves / Game saves** di Salvataggi.
I normali slot, backup e controlli di gioco restano disponibili. La navigazione
continua normalmente durante la sequenza; una direzione errata riparte dal
prefisso ancora valido. Chiusura/riapertura, Conferma/Annulla, cambio focus,
Dettagli, click e rotella interrompono la sequenza. Cattura dei binding,
editor e dropdown non contribuiscono. I comandi F6/F7/F8 della copia isolata
richiedono anche il flag sbloccato. I comandi CLI e i controlli di proprietà
delle copie isolate conservano i propri contratti: questa è una preferenza di
visibilità del menu.

## Comandi e Fast Forward

Tab passa fra sidebar e contenuto; nei selettori/editor esistenti Tab mantiene
la navigazione locale e Ctrl+Tab passa alla sidebar. Sul gamepad X/West cambia
focus. D-pad/frecce spostano il cursore senza cambiare le opzioni: **OK/Conferma**
apre la scelta, le direzioni evidenziano il valore, un secondo **OK** applica
e **Annulla** scarta. Vale anche per slider, slot, filtri e scelte nei moduli
Debug. OK usa il comando condiviso della periferica (per esempio South sul
gamepad), senza dipendere dal solo tasto Invio. Un click su una riga apre la
scelta; un click sull'opzione la conferma, il click destro annulla.
Sinistra dal margine del contenuto raggiunge la sidebar; su una sezione,
Destra torna al suo contenuto. Il cursore laterale ha fondo ottone pieno,
testo scuro e un indicatore `›`, distinti dalla sola sezione attiva; resta
riconoscibile anche al 100% di trasparenza. Nei layout a due colonne sinistra/destra
raggiungono prima il controllo adiacente. In Controlli, Su dal primo binding
raggiunge le linguette dei dispositivi: le direzioni evidenziano, OK seleziona.
Shift+Tab o Y/North cambia la vista Debug fra strumenti e riferimenti V2;
sono disponibili anche le due linguette cliccabili. Nei riferimenti ↑/↓
scorre la lista, ←/→ scorre i dettagli; la rotella agisce sull'area puntata.
Escape ritorna al contesto chiamante o riprende dalla Panoramica.
Tenere premuto Escape annulla una sola volta: non può prima chiudere un dropdown
e poi riprendere il gioco con gli eventi di ripetizione.

**Dettagli**, F1 oppure R3 apre una vista di sola lettura con valori completi,
binding, date, messaggi e metadati del contesto selezionato. Il testo scorre
internamente e il ritorno conserva selezione e scroll. È una fotografia dei
valori all'apertura. Cattura binding, editor, dropdown e operazioni host ancora
occupate hanno priorità: in questi casi Dettagli è disabilitato. Le utility
lasciano precedenza a un tasto del binding menu configurato, purché faccia parte
del vocabolario di cattura già supportato (per esempio Tab).

A menu aperto L1/R1 cambiano il volume di 10 punti, L1+R1 cambia Mute senza
cambiare il volume. L2/R2 cambiano la velocità di 20 punti, L2+R2 attiva o
disattiva Fast Forward senza cambiare la preferenza. Una finestra di 80 ms dà
precedenza alla pressione quasi simultanea; il rilascio parziale non ripete
la chord. La cattura di un binding e l'editing di testo o di una scelta nei
moduli host restano esclusivi. Questi pulsanti dorsali restano shortcut dedicate.
Questi controlli globali non consumano pulsanti quando il menu è chiuso.
I suggerimenti seguono **l’ultimo input usato**: tastiera o mouse mostrano
Frecce, Invio, Esc, Tab e F1; il gamepad mostra D-pad, A/B, X, RS CLICK e i
dorsali Xbox già previsti dal progetto. Le linguette Debug mostrano Shift+Tab
o Y e Dettagli mostra F1 o RS CLICK. La connessione di un pad e una pressione
statica già rilevata non sostituiscono un successivo comando da tastiera.
Le etichette non cambiano il profilo o i binding di gioco. Con la tastiera,
volume e Fast Forward si regolano scegliendo la voce e premendo Invio; i
suggerimenti non pubblicizzano scorciatoie da tastiera inesistenti.
Il footer sinistro mostra il binding attuale della periferica usata,
compreso il binding appena acquisito. Escape resta sempre un recupero.

La velocità effettiva è 100–800%, in passi del 20%. Un accumulatore razionale
produce frame interi per tick: per esempio 120% produce 1,1,1,1,2 frame ogni
cinque tick. Ogni frame mantiene il proprio clock GBA e le proprie operazioni;
non viene cambiata la durata di un frame emulato. Disattivare Fast Forward
ripristina 100%, conservando la velocità preferita. Il binding momentaneo
continua a invertire il latch della sessione.

`av.cfg` accetta `ff.percent = 120`. I vecchi `ff.speed = 2 | 4 | 8` conservano
il significato 200/400/800% e il loro formato al roundtrip. Le nuove percentuali
sono salvate con `ff.percent`. Audio `silent`, `fast` e `normal` mantengono le
semantiche esistenti: silenzio temporaneo, riproduzione alla velocità scelta
(anche 1.2×/1.4×), oppure backlog limitato a velocità normale.
Anche a 100% il toggle conserva uno stato ON distinto da OFF: da ON si applica
la modalità audio Fast Forward, pur eseguendo un solo frame per tick.

## Debug e inventario

Gli strumenti reali conservano selettori, parametri, preset, preparazione,
validazione e protezioni per le copie isolate. **Riferimenti Menu / CLI** legge
lo stesso registro di 97 operazioni di `recomp debug-operations`: ID, percorso,
handler, comando, parametri, prerequisiti, effetti e limiti. È sempre di sola
lettura e non interpreta i comandi mostrati. Una capacità solo CLI conserva
questa etichetta. Le schede degli strumenti esistenti rimangono gli ingressi
eseguibili; non ci sono pulsanti Esegui fittizi nei riferimenti.

La vista V2 contiene esattamente DBG2-01…28. La checklist corrente ha già chiuso
01…09: questi sono indicati completati, mentre 10…28 restano pianificati.
Eventi e CMD nell'intestazione sono esplicitamente previsti, senza un backend
live simulato. Cheat conserva la descrizione delle dipendenze mancanti.

[L'inventario precedente](development/MENU_ATLAS_BASELINE.json) conserva i
contratti prima della migrazione; [la mappa nuova](development/MENU_ATLAS_INVENTORY.json)
associa tutte le 190 occorrenze di righe, le 54 azioni precedenti e le 97
operazioni ai percorsi attuali. Include sottomaschere, dropdown e conferme;
ToggleFastForward è l'unica nuova azione frontend. Gli ID operativi, i
parametri, i contratti di sicurezza e gli handler CLI non cambiano.

Verificare gli asset e la mappa dopo un cambiamento con:

```sh
python3 scripts/menu_atlas_tasks.py --check
python3 scripts/menu_atlas_inventory.py --check
```

## Verifica della migrazione

Baseline Windows GNU/Wine: 29 test input-config e 72 test menu passati prima
delle modifiche, sul commit `945966c`; la specifica citava la precedente
`999d0a6`. La mappa include quindi anche i backend DBG2-01…09 già presenti.

La verifica Windows GNU release eseguita **solo tramite Wine** comprende:

- 31 test input-config e 86 test menu; nessun test preesistente rimosso.
- Un test di esportazione esplicita separato: 45 viste native, nove contesti
  per 1280×720, 1024×768, 854×480, 640×480 e 480×640. Fixture grafiche sintetiche
  dichiarate, distinte dai dati reali del gioco.
- 11 test frontend mirati: stato completo e clock del guest equivalenti a
  esecuzione sequenziale a 100/120/140/200/400/800%, audio frazionario e invalidazione
  della cronologia, coda audio normale, cattura XInput simulata e registro operazioni.
- GUI EUR con mouse, tastiera e XInput simulato: riferimenti di sola lettura,
  dropdown annullato senza scritture, chord quasi simultanee e rilascio parziale,
  slot rapido, overwrite su Annulla, Undo, 99 slot, maschere cartuccia/backup,
  rimappatura e binding dinamici. Le operazioni mantenute usano gli handler
  precedenti: la migrazione non riesegue indiscriminatamente tutti i tool CLI.
- Presentazione OpenGL e Vulkan tramite llvmpipe e fallback GDI software reale;
  Escape tenuto non riprende il gioco. Prova distinta del toggle 100% con audio
  silent/fast/normal e ritorno OFF, senza misure di durata o velocità.
- Dettagli da F1/R3/mouse, precedenza del binding Tab, cattura esclusiva,
  editor annullabili e schermate native di selettore, parametri e catalogo vuoto
  esplicitamente sintetico. Stato/clock del guest e originali ROM/seed/.sav
  rimangono invariati nelle prove di sola navigazione.
- Ridimensionamento effettivo Win32 tramite SetWindowPos, con GetClientRect
  verificato prima delle catture: Panoramica nelle cinque dimensioni sopra;
  selettore, parametri e catalogo a 854×480, 640×480 e 480×640. Le prime immagini
  ottenute ridimensionando soltanto la superficie X sono diagnostiche e non
  costituiscono la prova del layout adattativo.

I risultati dettagliati sono nel
[record della modifica](changes/2026-10-30-implement-native-clan-atlas-menu.json).
Log, eseguibili autenticati e screenshot restano locali in
`target/menu-clan-atlas/`; non sono fixture distribuibili.
Linux, Windows nativo, controller fisici e ROM americana rimangono da
verificare. Nessuna misura di prestazioni fa parte di questa migrazione.
AndroidPort è un albero separato e non viene modificato.

## File e avvio

`clan_atlas.rs` calcola geometria e navigazione condivise con l'hit test;
`atlas_theme.rs` disegna cornice e righe. `save_forms.rs` conserva le maschere
salvataggi, `information.rs` i testi completi e `shortcuts.rs` l'arbitraggio
delle chord. I moduli esistenti gestiscono catalogo, contenuti e parametri.
`input-config::fast_forward` possiede percentuali/configurazione/accumulatore;
`recomp::play` applica azioni, clock, audio e metadati reali. L'elenco completo
dei file cambiati è nel campo `files` del record; la mappa prima/dopo è
nell'inventario collegato sopra.

Con il target Rust Windows GNU e il linker MinGW configurati:

```sh
cargo build --locked --release --target x86_64-pc-windows-gnu -p recomp -p ffta-play
```

Avviare `target/x86_64-pc-windows-gnu/release/ffta-play.exe` nell'ambiente
Windows/Wine predisposto dal progetto, scegliere la propria ROM EUR e aprire
il menu con Escape o con la combinazione configurata (default Start+Back).
Per una copia isolata seguire [Debug tools](debug_tools.md); non sostituire
l'eseguibile di una sessione già autenticata per provarne una build diversa.


## Trasparenza e modifica dopo OK

La verifica mirata Windows GNU/Wine della modifica del 30 ottobre comprende
32 test input-config e 92 test menu passati (un'esportazione opzionale ignorata),
più i casi mirati di aggiornamento del selettore, cattura XInput frontend e
passaggio dal focus delle linguette al click del mouse. I test precedenti sono
stati adattati alla conferma esplicita, conservando i controlli di overwrite,
Undo, profili e cattura esclusiva.

La GUI EUR con XInput simulato verifica navigazione fra sidebar e contenuto
con la sola croce, volume e trasparenza senza modifiche prima di OK, annullamento,
sfondi a 0/50/100%, scelta nei parametri Code e riapertura al 50% persistito.
Le immagini sono state ispezionate. Stato completo, frame, clock e input guest
rimangono invariati; ROM e salvataggi originali conservano i propri hash.
Un'ultima correzione isolata del focus dopo click su una linguetta è verificata
con il solo caso Wine interessato, senza ripetere la GUI già accettata.

Evidenze locali: `target/menu-transparency-navigation/` (log di build e test,
`gui2/accepted-prefix.json`, `gui3/acceptance.json`, immagini e pin del runtime).
Due errori dell'harness relativi al puntatore e al prefisso delle etichette sono
conservati; i prefissi già riusciti non sono stati ripetuti. Nessun benchmark.
Linux, Windows nativo, controller fisici e versione USA restano **da verificare**.


## Default e suggerimenti contestuali (31 ottobre 2026)

Quattro casi mirati Windows GNU/Wine passano: default/compatibilità di
`menu.transparency`, sorgente dei suggerimenti senza modifiche a binding o
cattura, evidenziazione dopo Sinistra anche con sfondi trasparenti e il caso
pixel precedente reso esplicitamente opaco. Build di test e produzione senza
warning. La GUI EUR con XInput simulato verifica alternanza tastiera/pad,
focus laterale distinto, etichette non tagliate a 854×480 e 640×480, tastiera
usata mentre il pad resta premuto e scelta iniziale 10% senza chiave nel file.
Le catture sono state ispezionate. Configurazione e binding della copia restano
identici; stato completo, frame, clock e input guest e i tre originali privati
restano invariati. Evidenze locali: `target/menu-context-controls/`.
Linux, DirectInput/Gilrs reali, Windows nativo, controller fisici e USA restano
da verificare. Nessun benchmark o ripetizione delle prove di salvataggio/ROM.


## Verifica dello sblocco (31 ottobre 2026)

Windows GNU/Wine: un caso del parser e 97 casi del menu passano, con una prova
opzionale di esportazione ignorata. I test preesistenti dei percorsi Debug usano
una fixture esplicitamente sbloccata; tre nuovi casi usano il vero costruttore
bloccato e verificano visibilità/ingressi, sequenza/persistenza e reset/esclusività.
Il primo giro aveva un solo test di salvataggi numerati ancora senza lo sblocco;
è stato corretto e ripetuto da solo, conservando i 96 risultati già riusciti.
Le correzioni finali sono solo nelle fixture di test e non modificano il codice
della produzione. Anche gli otto casi del selettore host e il caso di bozza
Workbench interessati passano su Wine, senza ripetere l’intera suite runtime.

Build di produzione e test senza warning. La GUI EUR con XInput simulato
verifica schermate nascoste, assenza delle opzioni Debug in Salvataggi,
sblocco all'ottava direzione su tastiera e pad, scrittura `debug = true`,
riapertura sbloccata e rimozione con `debug = false` al riavvio della stessa copia.
Le immagini sono ispezionate; stato completo/frame/clock/input e i tre originali
privati rimangono invariati. Evidenze locali: `target/menu-debug-unlock/`.
Linux, Windows nativo, pad fisici, DirectInput/Gilrs e USA restano da verificare.
Nessun benchmark, passo guest o nuova prova di ROM/salvataggi personali.
