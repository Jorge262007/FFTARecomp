# Lingua del menu desktop

Il menu usa **English** come lingua iniziale, indipendentemente dalla lingua
del sistema e della ROM. Da **Game > System > Language** si può scegliere
**English** oppure **Italiano**. In italiano il percorso è
**Gioco > Sistema > Lingua**.

Conferma apre l'elenco; le frecce selezionano una lingua e una seconda conferma
applica la scelta immediatamente. Annulla chiude l'elenco senza cambiare la
preferenza. Il mouse usa lo stesso elenco e lo stesso percorso di conferma.
La scelta viene salvata tramite il normale handler delle opzioni in `av.cfg`:

```ini
menu.language = en
```

`it` seleziona l'italiano. Una configurazione precedente senza questa chiave,
una chiave vuota o una lingua non supportata usa l'inglese. Le altre preferenze
restano conservate. In modalità portable la chiave si trova nello stesso
`Data/config/av.cfg` delle altre opzioni. La preferenza riguarda l'interfaccia
desktop: non cambia il testo del gioco o i dati della ROM. `AndroidPort/` è
un albero separato e non riceve questa implementazione.

Il catalogo copre cornice, navigazione, titoli, etichette e scelte delle opzioni,
istruzioni tastiera/controller, forme di salvataggio e conferma, dettagli e
controlli dei selettori/strumenti. Data del salvataggio standard e descrizione
dell'ultima operazione annullabile conservano i propri dati e si ridisegnano
nella lingua corrente. I messaggi dinamici delle normali operazioni di
salvataggio usano la lingua selezionata quando vengono prodotti.

Nomi e percorsi dei file, binding, token di configurazione, sintassi dei comandi,
valori inseriti, descrizioni importate, riferimenti tecnici e payload diagnostici
conservano il testo originale. I risultati di operazioni già eseguite non sono
ritradotti retroattivamente. Tradurre la presentazione di una scelta non cambia
il valore salvato: per esempio **INTERNO** corrisponde sempre a
`video.fps = internal`.

## File delle lingue e aggiunta di una lingua

Ogni lingua è **un singolo file JSON** nella cartella
[`crates/input-config/languages/`](../crates/input-config/languages/):

- [`en.json`](../crates/input-config/languages/en.json): catalogo inglese di base.
- [`it.json`](../crates/input-config/languages/it.json): traduzioni italiane.

Il file contiene `code` (token salvato nelle opzioni), `name` (nome nativo
mostrato nel selettore) e `messages` (chiavi e relativi testi). Il nome del file
corrisponde al codice: `fr.json`, `pt-br.json`, ecc.

Per esempio, per aggiungere il francese:

1. Creare il file copiando automaticamente tutte le voci inglesi:

   ```sh
   python3 scripts/menu_language.py create fr --name "Français"
   ```

2. Aprire `crates/input-config/languages/fr.json` e tradurre soltanto i valori
   di `messages`. Lasciare invariate le chiavi e i segnaposto come `{slot}`,
   `{value}` e `{0}`. Salvare il JSON in UTF-8. Alcune chiavi derivano dai vecchi
   testi italiani: sono identificatori stabili e non vanno tradotti.

3. Controllare i file e ricompilare con la normale procedura del progetto:

   ```sh
   python3 scripts/menu_language.py check
   cargo build --locked --release -p recomp -p ffta-play
   ```

**Non servono modifiche a Rust, elenchi di lingue o registrazioni manuali.**
La compilazione scopre automaticamente ogni `languages/*.json`, controlla i
file e incorpora il registro nel programma. La nuova lingua compare nelle
opzioni dopo la ricompilazione. Gli eseguibili distribuiti contengono già le
traduzioni: non richiedono Python o JSON esterni durante l'esecuzione.

Si possono anche aggiungere i JSON a mano, copiando `en.json` e cambiando
`code`, `name` e i testi. Il comando `create` è una comodità e rifiuta di
sovrascrivere file esistenti. Il controllo e la compilazione rifiutano JSON
malformati, chiavi duplicate/sconosciute, nomi duplicati, traduzioni vuote e
segnaposto diversi dall'inglese, indicando il file da correggere.

Una lingua può essere parziale: le chiavi assenti usano il testo di `en.json`.
Aggiungere nuove voci inglesi non obbliga a completare subito tutte le altre
lingue. Se manca anche la chiave inglese, il renderer mostra la chiave sorgente.
Le voci uguali all'inglese sono ammesse, per esempio Audio, Video o ROM.

`menu::i18n::format` sostituisce i segnaposto senza reinterpretare i valori
inseriti. I renderer risolvono le traduzioni prima della misura o dell'andata
a capo; la lingua appartiene a ciascuna istanza del menu.

L'esempio `menu --example preview` accetta qualsiasi codice incorporato come
ultimo argomento, dopo il percorso facoltativo dello sfondo (stringa vuota per
ometterlo). Le pagine `system` e `language` mostrano il selettore senza caricare
ROM o salvataggi.

## Verifica

I test mirati controllano default e compatibilità delle vecchie configurazioni,
roundtrip delle due lingue, annullamento/conferma, selezione con mouse, cambio
immediato, riapertura e conservazione dei token tecnici. Il rendering osservato
copre entrambe le lingue a 480×640, 854×480 e 1280×720, istruzioni controller,
timestamp e descrizione Undo conservata. La compilazione e il comando check verificano chiavi uniche e
parità dei segnaposto; i valori con parentesi graffe rimangono letterali.

Le prove Windows GNU/Wine, le anteprime senza ROM e il controllo di compilazione
di `recomp`/`ffta-play` sono registrati nel record del 2026-11-01. Non equivalgono
a una sessione Windows nativa o a una prova con controller fisico. La suite
menu contiene inoltre un'incongruenza preesistente e separata fra
`ContentSlot::ALL` e le righe `Page::ToolInputs`: il riferimento CheatCatalog
è enumerato ma non presente nella pagina.

Il registro generato è inoltre verificato con una terza lingua sintetica in una
cartella temporanea, compilando la vera implementazione di Language senza
modificare Rust né aggiungere lingue di prova alla distribuzione. Il comando
create/check ha test su file esistenti, Unicode, traduzioni parziali e rifiuti
per metadati, chiavi e segnaposto non validi. I cataloghi inglese e italiano
sono stati migrati conservando esattamente le 378 voci precedenti.
