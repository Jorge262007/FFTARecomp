# TODO — Estrazione ROM, dati decodificati e mod

Richiesta aggiornata il 21 ottobre 2026. **FEAT-04-ROM-DATA** è in esecuzione;
inventario e contratto iniziale sono definiti. Le migrazioni runtime restano aperte.
Ambito: workspace desktop, Windows come piattaforma principale di verifica;
Linux come controllo aggiuntivo. `AndroidPort/` richiede lavoro separato.

## Obiettivo e ordine di lavoro

L'obiettivo principale è scompattare la ROM selezionata in file di risorse,
decodificarli in formati utilizzabili e far leggere al runtime quei dati.
Al primo avvio si prepara il dataset; agli avvii successivi si riutilizza la
versione valida. Il supporto alle mod e a modifiche più ampie viene dopo.

Priorità utente aggiornata: per ora **non misurare le regressioni di performance**;
l'ottimizzazione verrà dopo. Non avviare benchmark, profili o campagne di avvio/
RSS, né comprimere o ritoccare formati per rispettare vecchi budget di spazio.
Le misure già raccolte restano storiche; non bloccano il lavoro funzionale.
Restano necessarie le prove di equivalenza, incluso il clock guest, l'uso reale
dei file e la compatibilità di native/save/Load.

Ordine: **inventario → estrazione e decodifica → preparazione all'avvio →
consumo diretto → verifica → mod → modifiche estese**.
Creare file che il gioco non usa non soddisfa l'obiettivo. Anche ricomporre
soltanto una ROM dai file estratti è un passaggio eventualmente utile al
confronto, ma non completa il percorso di lettura dei dati decodificati.

Il caricamento dei file può avvenire all'avvio o prima del loro utilizzo, con
cache in memoria: «lettura dai file» non richiede accessi al disco per pixel,
campione audio o istruzione. I miglioramenti di velocità restano da misurare.

## Punto di partenza verificato e vincoli

- [play.rs](../../crates/recomp/src/play.rs) legge già tutta la ROM con
  `std::fs::read` e la passa alla macchina. In
  [mem.rs](../../crates/gba-core/src/mem.rs), `MemMap.rom` è un `Vec<u8>`;
  le normali letture guest accedono a quel buffer in RAM.
- Una ROM contiene codice, tabelle, puntatori e risorse anche compresse o
  condivise. Non è un archivio con un filesystem da estrarre genericamente:
  formati e confini vanno individuati per ciascuna famiglia di risorse.
- [ffta_rom.rs](../../crates/gba-core/src/ffta_rom.rs) riconosce profili EUR/USA
  tramite hash esatti. [ensure_native](../../crates/recomp/src/main.rs) lega
  la cache di traduzione all'immagine e agli altri parametri di compilazione.
  Non basta cambiare un hash dichiarato per rendere compatibili nuovi dati.
- Gli [stati macchina](../../crates/gba-core/src/save_state.rs) e gli
  [slot desktop](../../crates/recomp/src/save_state.rs) usano l'identità ROM.
  Il nuovo dataset dovrà restare coerente con salvataggi, load e Undo.
- Il codice eseguibile guest e le strutture non ancora migrate possono
  richiedere l'immagine originale. Eliminare del tutto la dipendenza dalla
  `.gba` è un criterio separato, da valutare dopo aver censito tali dipendenze;
  non è una conseguenza automatica dell'estrazione delle risorse.

## ROMDATA-01 — Inventario e contratto dei dati

**Dipendenze:** nessuna. **Priorità:** prima fase obbligatoria.

- [x] Censire grafica, palette, tile/mappe, sprite/animazioni, testi, tabelle
  di gioco e audio. Per ogni famiglia annotare formato, compressione, confini,
  puntatori, riferimenti incrociati, routine di lettura e grado di verifica.
  Distinguere dati statici della ROM e stato dinamico in RAM.
- [ ] Verificare separatamente EUR/USA e lingue. Riutilizzare i lettori già
  presenti come evidenza parziale; non dedurre offset per sola analogia.
- [x] Definire ID stabili delle risorse, schema versionato e manifest con
  hash della ROM sorgente, versione estrattore/decoder, file e hash dei contenuti,
  dipendenze e copertura. Conservare la provenienza dagli intervalli originali.
- [x] Scegliere una prima famiglia verificabile dall'estrazione fino al gioco,
  in base alle routine individuate. Separare contenuti decodificati e metadati
  necessari: per esempio una sola immagine non conserva palette indicizzate,
  priorità, trasparenza e animazioni; un PCM non conserva il sequenziamento audio.
- [x] Pubblicare la matrice di migrazione: per ogni famiglia indicare estrattore,
  decoder, lettore runtime, verifiche e dipendenze residue dalla ROM.

**Chiusura:** contratto e prima famiglia definiti con evidenza; famiglie ancora
ignote elencate esplicitamente. L'inventario non è la migrazione completa.

**ROMDATA-01-CONTRACT completato; ROMDATA-01 parziale:**
[inventario e contratto](ROM_DATA_INVENTORY.md).
Prima famiglia: `graphics.obj-dimensions`, metadati statici verificati sui due
profili esatti. L'inventario distingue le fonti note da confini/cataloghi/lingue
non verificati; non dichiara un catalogo completo delle risorse del gioco.
La verifica EUR/USA è completa per la prima famiglia; cataloghi regionali e
linguistici delle altre famiglie restano da individuare e verificare.

**ROMDATA-01-GRAPHICS-LZ77 completato per i produttori iniziali osservati:**
quattro sorgenti LZ77 per regione verificate su avvii puliti1200frame Linux e
Windows/Wine; confini dei flussi e uscite guest coincidono. Individuati anche
due root regionali di descrittori da88byte. Limiti del catalogo, campi interni,
interpretazione dei pacchetti e lingue restano aperti.
[Contratto e prove](../rom_data_graphics.md).

**ROMDATA-01-BG-DESCRIPTOR-PREFIX completato per il blocco contiguo verificato:**
162record/14256byte, tutti i riferimenti entro i limiti, primo payload distinto
dai record e metadati identici nei due profili. Selettori condizionati verificati
con le routine originali per ogni record, sentinelle e fallback. Riferimenti
relativi, triple di trasferimento, puntatore variante e byte non interpretati
sono distinti. La famiglia grafica01resta parziale per asset/programmi collegati,
altri cataloghi e lingue. [Contratto](../rom_data_graphics.md#authenticated-descriptor-metadata).


**ROMDATA-01-BG-PALETTES completato per il catalogo base dichiarato:** tutte
le162voci puntano a127sorgenti per regione,125LZ77e2grezze. Intervalli, alias e
buffer decodificati coincidono tra EUR/USA. Quattro oracoli originali Linux/Wine
passano1296chiamate/72792istruzioni ciascuno, output/guardie completi e bit15
preservato. I flussi per palette alternative sono classificati separatamente sotto; le
precondizioni dei cinque indici FFrestano da chiarire. Modello tipizzato e consumo delle palette sono completati
separatamente nelle unità02/04sotto.
[Contratto e prove](../rom_data_graphics.md#base-background-palette-catalog).

**ROMDATA-01-BG-ALTERNATE-PALETTE-PROGRAMS completato per gli indici validi:**
due cataloghi da157chiavi0..156per regione,314flussi con header BE e formato
LZ/RLE distinto dal BIOS; ciascuno produce256B. Tabelle, sorgenti e output
coincidono tra EUR/USA; quattro oracoli originali Linux/Wine passano1256chiamate
ciascuno, output/guardie completi ed EWRAM/SP invariati. Cinque record con FF
restano distinti: nessuna dereferenziazione o fallback inventato, precondizioni
dei chiamanti ancora da verificare. Parser tipizzato/estrazione/consumo02/04
sono i prossimi passi. Nessuna misura prestazionale.
[Contratto](../rom_data_graphics.md#alternate-background-palette-catalogs).

## ROMDATA-02 — Estrattori e decoder

**Dipendenze:** ROMDATA-01. Procedere per famiglie senza dichiarare finito il
resto del catalogo quando funziona il primo prototipo.

- [x] Implementare parser con controlli su lunghezze, offset, puntatori,
  dimensioni decodificate e riferimenti; rifiutare input troncati o incoerenti.
- [x] Estrarre e decodificare la prima famiglia nel formato concordato,
  mantenendo le informazioni necessarie a riprodurre il comportamento originale.
  Blocchi sconosciuti possono restare grezzi, marcati come non decodificati.
- [x] Rendere l'output deterministico e ispezionabile: gli stessi input e la
  stessa versione producono manifest e dati equivalenti, senza percorsi assoluti.
- [x] Verificare con fixture sintetiche e confronti locali con il decoder
  originale o con dati osservati nel gioco. Quando il formato lo consente,
  verificare anche il roundtrip, senza richiedere compressione byte-identica
  se più codifiche rappresentano gli stessi dati.
- [ ] Estendere estrattori e decoder alle altre famiglie della matrice,
  registrando per ciascuna contenuti supportati e limiti.

**Chiusura per famiglia:** file decodificati corretti, metadati sufficienti e
input errati gestiti. La sola esportazione di blocchi binari non basta.

**ROMDATA-02-OBJ-DIMENSIONS completato:** estrattore e decoder tipizzato,
CLI `rom-data extract|verify`, manifest e controlli di provenienza.
Le prime quattro voci sono completate per questa famiglia (senza puntatori o
compressione); parser delle altre famiglie rimangono da implementare.
Test Linux e Windows/Wine passano anche con le due ROM locali. Dettagli e
limiti in [risorse decodificate](../rom_data.md). Il consumo diretto della prima
famiglia è ora collegato nel percorso opt-in04.

**ROMDATA-02-LZ77-DECODER completato come componente offline:** input e memoria
limitati, riferimenti sovrapposti, consumo esatto senza padding e rifiuto di
input malformati. Quattro test sintetici per target e confronto con le uscite
guest già catturate passano su Linux/Wine. Non aggiunge ancora una famiglia al
manifest: servono prima i contratti semantici del catalogo grafico.

**ROMDATA-02-BG-PACKETS completato come componente offline:** parser tipizzati
dei pacchetti a16bit e delle coppie indicizzate, con limiti di destinazione,
scritture e numero di pacchetti. Tre test sintetici per target e confronti delle
routine originali EUR/USA passano su Linux/Wine:7invocazioni/87160istruzioni
per caso, tutta l'EWRAM confrontata e continuazione a blocchi verificata.
Il ramo con coppie letterali conserva la sintassi, con guardia sulle destinazioni
dopo una prova originale contraria all'ipotesi iniziale.
[Contratto, prove e limiti](../rom_data_graphics.md#typed-background-packets).
Catalogo e integrazione delle nuove risorse restano aperti.

**ROMDATA-02-BG-DESCRIPTOR-METADATA completato come componente offline:** decoder
tipizzato, controlli sui limiti, autenticazione della sorgente, roundtrip esatto
e TOML deterministico. Tre test sintetici per target e quattro casi EUR/USA su
Linux/Wine passano:2612chiamate originali/134629istruzioni per caso. I file di
prova da106309byte coincidono sui quattro percorsi. Integrazione nel manifest,
preparazione e consumo runtime sono stati selezionati separatamente come segue.

**ROMDATA-02-BG-MANAGED completato per estrazione/verifica/preparazione:**
nuovo file descrittori nel manifest, schema1/estrattore3/decoder2 e directory
v1/e3-d2, con versioni precedenti conservate. Limiti16KiB manifest/OBJ,
256KiB BG e272KiB totali; pubblicazione atomica di tre file, riuso completo,
recupero, identità dell'intero dataset e attivazione autenticata. Passano25test
Linux/23Wine,18controlli dei veri CLI per target e scenari privacy tar/zip.
[Prove e limiti](../rom_data_graphics.md#managed-descriptor-extraction-and-preparation).
Questa unità prova la preparazione; il successivo consumo diretto04è descritto sotto.


**ROMDATA-02-BG-BASE-PALETTE-DATA completato:** modello di banchi da16valori
u16, bit15conservato, parser limitato raw0/LZ7710e intervalli esatti senza
padding. Tutte le162voci sono estratte in JSON leggibile; manifest con tutte
le provenienze, quattro file e revisione1/5/4, vecchie generazioni conservate.
Passano39test Linux/37Wine e22controlli CLI per target, inclusi oracoli originali,
integrità/riparazione/rilocazione e compatibilità delle risorse già consumate.
[Contratto e limiti](../rom_data_graphics.md#typed-palette-extraction-and-preparation).
Questa unità copre la preparazione; il successivo consumo04è completato sotto
con revisione1/6/5e metadati di compatibilità. Nessun lavoro di compressione/spazio.

## ROMDATA-03 — Preparazione automatica e riuso all'avvio

**Dipendenze:** ROMDATA-02 per la prima famiglia.

- [x] Integrare la preparazione dopo selezione/verifica della ROM e prima
  dell'avvio del gioco: estrarre quando il dataset manca o la versione cambia;
  riutilizzarlo quando identità e contenuti sono validi.
- [x] Definire i percorsi attraverso lo storage installed/portable esistente.
  Proposta da confermare: `Data/resources/<rom-hash>/<schema>/` in portable;
  rispettare [le regole di rilocazione](../portable.md). Tenere separati dati
  base rigenerabili, future mod utente, cache native e salvataggi.
- [x] Scrivere in una directory temporanea, validare e pubblicare soltanto un
  dataset completo; gestire annullamento, spazio insufficiente, avvio concorrente
  e crash senza sostituire l'ultima versione valida con un risultato parziale.
- [x] Mostrare avanzamento ed errori comprensibili; prevedere la rigenerazione
  di dati mancanti/corrotti dalla ROM sorgente senza sovrascrivere mod o save.
- [x] Verificare primo avvio, riavvio senza riestrazione, cambio ROM/regione,
  aggiornamento decoder e spostamento della cartella portable. Definire come
  riconoscere una sorgente cambiata prima di riutilizzare il suo dataset.

**Chiusura:** un primo avvio prepara le risorse; quelli successivi riusano un
dataset valido. L'estrazione resta locale e non modifica la ROM sorgente.

**ROMDATA-03-OBJ-STARTUP completato nel percorso opt-in della prima famiglia:**
`rom-data prepare` e `play --rom-data`, generazioni immutabili, selezione atomica,
lock OS, riuso/verifica e recupero. Le voci sopra si riferiscono alla famiglia
`graphics.obj-dimensions`; le altre famiglie non sono implicitamente migrate.
Test Linux e Windows/Wine, compresi arresti forzati e due avvii GUI prima/dopo
lo spostamento portable, passano. Esaurimento spazio simulato; power-loss fisico
e Windows nativo non verificati. Dettagli in [risorse decodificate](../rom_data.md).
Il consumo diretto è ora collegato in04; l'abilitazione normale attende i gate05.

## ROMDATA-04 — Uso diretto dei dati estratti nel runtime

**Dipendenze:** ROMDATA-01/02/03. **Cuore dell'obiettivo principale.**

- [x] Introdurre un accesso alle risorse tramite ID e strutture tipizzate,
  con caricamento/precaricamento dai file decodificati. Definire budget di memoria,
  durata dei dati e invalidazione; evitare I/O nei callback audio e nei percorsi
  per pixel o istruzione.
- [ ] Per ogni famiglia seguire tutti i consumatori: codice nativo tradotto,
  interprete, letture CPU/DMA e percorsi host come rendering e audio. Individuare
  dove il gioco richiede byte grezzi, decomprime, copia o interpreta le risorse.
- [x] Collegare la prima famiglia al caricatore e sostituire il consumo della
  corrispondente risorsa ROM con quello dei dati estratti. Se occorre trasferire
  dati in RAM/VRAM guest, preservarli con layout, registri ed effetti osservabili
  corretti; non applicare lookup generici a ogni lettura del bus.
- [ ] Quando si sostituiscono routine guest, preservare cicli, IRQ, DMA, RNG e
  ordine degli eventi. Dimostrare come si conserva il tempo guest se una
  decompressione viene anticipata all'estrazione: saltarla senza contabilizzarne
  gli effetti non è una sostituzione equivalente.
- [x] Distinguere identità della ROM base, dataset e codice compilato. Rivedere
  costanti incorporate, hook e cache native: nessuna libreria deve riutilizzare
  dati o ipotesi incompatibili con il dataset effettivamente caricato.
- [x] Aggiungere diagnostica di provenienza e contatori di uso/fallback per
  famiglia. Nei test delle risorse migrate rendere rilevabile un accesso al
  vecchio percorso; verificare anche che file mancanti non vengano ignorati
  silenziosamente a favore della ROM.
- [ ] Migrare progressivamente tutte le famiglie inventariate. Registrare
  separatamente codice, dati ancora grezzi e letture ROM residue; un fallback
  temporaneo mantiene la compatibilità ma lascia aperta la relativa migrazione.

**Chiusura per famiglia:** il gioco usa il file decodificato e la provenienza è
verificata. **Chiusura complessiva:** tutte le famiglie di dati previste dalla
matrice sono migrate e verificate; le esclusioni motivate sono dichiarate.
Un solo esempio funzionante è una consegna parziale, non il completamento.

**ROMDATA-04-OBJ-CONSUMPTION completato per implementazione e prove iniziali:**
la prima famiglia viene letta dai file in `play/frames/runc --rom-data`;
proiezioni fisse per CPU/DMA/API native, letture host tipizzate, RAM modificata
preservata, contatori e risorse non serializzate mantenute in Load/Undo/backup.
Budget misurato432B; nessuna istruzione o decompressione saltata. Corretto anche
il lettore host EUR, verificando la sorgente ROM contro le istruzioni originali.
L'estrattore2 registra l'ulteriore alias EUR e mantiene le vecchie generazioni.
Dettagli e prove in [consumo runtime](../rom_data_runtime.md).
Le voci spuntate coprono l'infrastruttura e la prima famiglia, non le altre.
La matrice di routine originali copre48casi EUR/USA con letture guest effettive;
il confronto nativo600frame prova uso host e parità di stati/immagini/audio,
senza attribuire letture guest non eseguite in quella schermata iniziale.
La nuova tranche05 verifica scene/transizione EUR e costi locali descritti sotto;
il trace USA è ora verificato separatamente come descritto sotto. Gli altri
gate e le altre famiglie restano aperti.

**ROMDATA-04-BG-CONSUMPTION completato per metadati e prove iniziali:**
162descrittori autenticati conservati come dati tipizzati immutabili; proiezioni
CPU/DMA/API native e contatori BG separati. Schema/estrattore/decoder1/3/2,
identità ROM/save/native e ABI invariati. Misurati440Bcondivisi/OBJ+28672BBG,
entro i limiti1KiB/64KiB dichiarati prima dei test. Linux/Wine204core e98/97
frontend passano; quattro casi regionali da2608chiamate confrontano originale,
interprete e TinyCC anche con ROM sorgente in memoria alterata dopo attivazione.
3867letture BG native per caso; Load/Undo/backup e file mancante verificati.
Due avvii EUR nativi1200frame coincidono per tutti gli stati/immagini/OAM/audio,
con53letture BG effettive. [Prove e limiti](../rom_data_graphics.md#direct-descriptor-consumption).
Scene/GUI/costi05BG, asset/programmi collegati e altre famiglie restano aperti.

**ROMDATA-04-BG-BASE-PALETTE-CONSUMPTION completato per il catalogo base:**
162palette conservate nel proprietario immutabile; banchi decodificati per HLE
LZ77e rappresentazione sorgente esatta per CPU/DMA/API native/BIOS guest.
Revisione1/6/5, e5conservata; scritture, clock e identità ROM/save/native invariati.
Linux/Wine210core e101/100frontend passano. Quattro casi originali/native da
1296chiamate/72792istruzioni coincidono anche alterando tutti i vecchi intervalli
ROM dopo attivazione;1272chiamate LZ77leggono i banchi per caso. Load/Undo/
backup/file rimossi passano. Nuovi avvii nativi EUR1200frame coincidono per
stati/immagini/OAM/audio con una palette160B effettivamente letta dai file;
CLI e5refusal/retention e coppie interprete/native passano. Proprietario123833B
su entrambi i target; nessuna ottimizzazione di spazio.
[Prove e limiti](../rom_data_graphics.md#direct-base-palette-consumption).
Programmi alternativi, altre famiglie/lingue e ulteriori gate05restano aperti.

## ROMDATA-05 — Equivalenza, salvataggi e prestazioni

**Dipendenze:** ogni integrazione ROMDATA-04, prima di abilitarla normalmente.

- [ ] Confrontare originale e dataset estratto senza modifiche, con stessi
  input: stato guest a fine frame, RNG, clock, RAM/VRAM/OAM, immagini e audio
  deterministico. Coprire intro, mappa, battaglia, menu e transizioni pertinenti;
  riutilizzare le prove già valide senza confondere copertura e completezza.
- [ ] Verificare percorsi nativi e interprete, Fast, pausa, load/Undo e ripristino
  backup. Ricostruire risorse host e cache dopo il caricamento senza serializzare
  puntatori o duplicare tutti i file estratti negli stati macchina.
- [ ] Definire identità e compatibilità di `.ffstate` e `.sav`: il dataset base
  equivalente deve preservare gli stati esistenti quando possibile. Se cambiano
  layout o semantica, versionare e rifiutare combinazioni incompatibili prima di
  modificare la macchina o scrivere il salvataggio.
- [ ] Misurare separatamente estrazione iniziale, avvio con dataset pronto,
  spazio su disco, RAM e tempi di frame/caricamento/audio rispetto al percorso
  attuale in memoria. Fissare i limiti di regressione prima del confronto.
  Registrare anche un risultato neutro o peggiore: l'estrazione non prova velocità.
- [ ] Verificare Windows nativo; usare Windows/Wine e Linux come prove distinte.
  Aggiornare launcher/documentazione/packager senza includere ROM, risorse estratte,
  save personali o codice generato nei repository e nei pacchetti distribuiti.

**Consegna principale:** estrazione automatica, riuso all'avvio e consumo diretto
verificati per la matrice dichiarata, equivalenza senza mod, costi misurati e
dipendenze residue documentate. Le fasi mod seguenti non bloccano questa consegna.

**ROMDATA-05-OBJ-SCENES-COSTS completato nel perimetro locale dichiarato:**
[protocollo e risultati](ROM_DATA_VALIDATION.md),
[record](../changes/2026-10-21-validate-decoded-obj-scenes-and-costs.json).
Confronti Linux e Windows/Wine su mappa240, battaglia/menu3530 e avvio fino alla
battaglia8476frame: ogni stato completo, immagine/OAM, conteggio e flusso audio
coincide; lettura effettiva dell'alias EUR al frame5400. Compatibilità base degli
stati e dei save preservata, senza nuova versione. Cinque coppie Linux contro il
legacy corretto passano i limiti fissati prima della misura: +1,0081%/-0,2355%
di costo medio; risultato neutro. Sette primi avvii/riusi: mediane32,416/21,019ms
Linux e109,222/91,891ms Wine; massimo riuso Wine183,130ms conservato, con gate
sulla mediana. RSS Linux e spazio disco rispettano i budget. Test sintetico
archivi tar/zip esclude dati privati e include le nuove istruzioni.

**ROMDATA-05-OBJ-GUI completato nel perimetro EUR Linux/Windows-Wine:**
Fast tenuto e commutato, pausa/ripresa mantenendo Fast, ritorno1x, Load/Undo/
ricaricamento e ripristino backup dal menu passano con uso delle risorse,
catture ispezionate, veri callback44,1/48kHz e file protetti invariati.
[Record e limiti](../changes/2026-10-21-validate-decoded-resource-gui-lifecycle.json).
Il gain finale è0: nessuna prova di udibilità o prestazioni callback A/B.
**ROMDATA-05-OBJ-ARCHIVES completato nel perimetro dichiarato:** corrette le
copie/allowlist Docker per includere le guide richieste dal packager; archivi
reali con395test Linux/392Windows-Wine passati (24opzionali ignorati ciascuno).
Privacy, PE/ELF, TinyCC automatico e GUI del runtime effettivamente estratto con
nuova preparazione portable/Load/Undo/ricaricamento passano.
[Record e limiti](../changes/2026-10-21-validate-decoded-resource-portable-archives.json).
Archivi/binari precedenti invariati; variazione di un metadato BUILD-INFO
registrata esplicitamente.

**ROMDATA-05-OBJ-USA-INITIALIZER completato su Linux e Windows/Wine:** avvio
pulito senza save né PC forzato fino al copier originale USA, frame791. Tutte
le13istruzioni coincidono per stato completo; CpuSet legge24halfword dal file
e copia correttamente3648byte, inclusi48di metadati. Anche la prova con il solo
vecchio alias alterato dopo l'attivazione autentica passa.
[Prova e limiti](ROM_DATA_VALIDATION.md#original-usa-initializer).
Non è una verifica completa delle scene/audio USA. ROMDATA-05 resta parziale
per Windows nativo, non disponibile all'utente: per ora solo Linux/Wine.
Abilitazione normale e altre famiglie/mod restano aperte.

**ROMDATA-05-BG-SCENES-COSTS parziale:** parità completa Linux/Wine su mappa240,
battaglia/menu3530e transizione8476frame; letture BG53nella transizione. Cinque
coppie Linux passano i limiti di frame (+1,3600%/-0,6017%mediana media), così
come avvio/riuso e spazio disco. Il picco RSS invece supera il limite2MiB:
+2864KiBmediana/+2944KiBmassimo. Risultato conservato, nessuna abilitazione
normale. L'attribuzione individua la preparazione/parsing TOML; i prototipi
TOML alternativo e inline sono respinti. Il prototipo JSON ha individuato il parsing diretto con22528Bdi heap; la
revisione successiva adotta JSON standard leggibile, verificato nel runtime,
seguendo la nuova preferenza utente descritta sotto.
[Protocollo e risultati](ROM_DATA_VALIDATION.md#bg-metadata-e3-results-and-rss-failure).
Le prove scene/tempi concluse non vanno ripetute per ricostruire il contesto;
RSS, GUI/archivi BG e Windows nativo restano aperti.


**ROMDATA-02-BG-JSON completato:** revisione1/4/3, BG in JSON leggibile standard
con parsing diretto nelle strutture tipizzate; manifest/OBJ restano TOML.
Generazioni e3precedenti conservate; autenticazione, guardie sugli input,
riuso/riparazione e consumo nativo/interprete passano su Linux/Wine.
[Prove](../rom_data_graphics.md#readable-json-datasets).
**Preferenza aggiornata:** nessun lavoro per comprimere o risparmiare spazio
nei dati estratti. I149261byte BG/150818del dataset sono accettati per
leggibilità; limiti del parser proteggono da input malformati. I risultati dei
costi e dei prototipi restano informativi, senza nuovi cicli per centrare il
precedente limite disco. Prossima priorità: payload e cataloghi ancora in ROM.

**ROMDATA-05-BG-BASE-PALETTE-SCENES completato per le scene EUR dichiarate:**
nuovi confronti con dataset e6passano240frame mappa,3530battaglia/menu e8476
transizione su Linux/Wine. Tutti gli stati completi, immagini/OAM, dispatch e
flussi audio coincidono. Le scene stabili non caricano palette; la transizione
legge una palette decodificata da160B e53descrittori BG. Fixture e cache precedenti
riusate e protette. [Risultati](ROM_DATA_VALIDATION.md#base-palette-e6-scene-acceptance).
Costi/GUI/archivi e Windows nativo restano gate separati; nessuna abilitazione
normale o migrazione di altre famiglie è dedotta da queste scene.

## ROMDATA-06 — Mod tramite sostituzione delle risorse

**Dipendenze:** completare la consegna principale ROMDATA-01…05.

- [ ] Definire una directory `mods/` nello storage applicativo e un manifest
  versionato con ID, versione, profili/dataset compatibili, dipendenze e risorse
  sostituite. Risolvere percorsi relativi entro il pacchetto, con limiti su
  dimensioni e formati; preservare i file utente durante la rigenerazione base.
- [ ] Definire ordine deterministico e gestione dei conflitti. Validare gli
  override con gli stessi contratti delle risorse base prima di avviare il gioco;
  mostrare errori di dipendenze/versione invece di caricare una combinazione parziale.
- [ ] Aggiungere attivazione/disattivazione persistente e indicazione delle mod
  attive. Per la prima versione applicare i cambi al riavvio della sessione;
  nessuna necessità di ricaricamento a caldo per completare il supporto iniziale.
- [ ] Collegare hash dei contenuti e ordine delle mod a cache e salvataggi;
  isolare gli slot incompatibili e proteggere i `.sav` originali quando la mod
  cambia dati di gioco. Disattivare le mod deve ripristinare le risorse base.
- [ ] Provare mod originali/minime per le famiglie supportate, modifica visibile
  nel gioco, conflitti, file mancanti e ripristino. Distribuire soltanto esempi
  originali o descrizioni/patch, non gli asset estratti dalla ROM dell'utente.

**Chiusura:** mod caricate davvero dal percorso risorse, selezione riproducibile,
base recuperabile, compatibilità ed errori verificati.

## ROMDATA-07 — Modifiche estese a contenuti e logica

**Dipendenze:** ROMDATA-06 e verifica delle strutture interessate.

- [ ] Estendere i formati a testi più lunghi, nuove risorse, mappe, animazioni e
  tabelle di gameplay, affrontando limiti di ID, dimensioni, puntatori e interfacce.
  Distinguere sostituzione di contenuti esistenti e aggiunta di nuovi contenuti.
- [ ] Progettare separatamente script, patch di codice e regole di gioco:
  stabilire necessità di ricompilazione, revisione ABI, invalidazione delle cache
  e disabilitazione degli hook non compatibili. Un override grafico non abilita
  automaticamente modifiche alla logica.
- [ ] Definire strumenti e documentazione per autori, migrazioni di schema e
  verifica dei pacchetti; aggiungere esempi originali e regressioni pertinenti.

**Chiusura:** ogni nuova capacità ha un contratto, esempi e compatibilità
verificata; le modifiche non supportate sono dichiarate.

## Regole di avanzamento

Selezionare una sottofase in [TASK_HANDOFF.md](TASK_HANDOFF.md), registrando
risultati e criteri mancanti per famiglia. Non chiudere il genitore finché i
suoi criteri restano aperti e non riaprire figli conclusi senza nuova evidenza.
Seguire [MEMORY_WORKFLOW.md](MEMORY_WORKFLOW.md) per documentazione, record e
verifiche; mantenere ROM, risorse estratte e prove personali fuori da Git.

**Prossima azione:** ROMDATA-04-BG-CONSUMPTION, collegare ai descrittori preparati
le letture effettive CPU/DMA/API native e verificare provenienza e compatibilità;
abilitazione normale soltanto con tutti i gate soddisfatti.
Gli altri cataloghi, le migrazioni e le fasi mod restano nell'obiettivo completo.
