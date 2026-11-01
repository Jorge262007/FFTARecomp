# Compilare Android con Docker

Servono **Docker con Buildx** e **Python 3.9+** sul computer host. Su Windows,
Docker Desktop deve usare container Linux. Non occorre installare Rust, Java,
SDK o NDK sull'host; non serve una ROM per compilare l'APK.

Dalla radice del repository:

```sh
python3 scripts/android/build_android_docker.py
```

In PowerShell lo stesso comando può usare `python` al posto di `python3`.
La scelta predefinita è il port mantenuto in **`AndroidPort/`**. Il risultato è:

```text
dist/docker-android/FFTARecomp-android-arm64-preview.apk
dist/docker-android/SHA256.txt
dist/docker-android/BUILD-INFO.txt
dist/docker-android/BUILD-SOURCE.json
dist/docker-android/tests/TEST-*.xml
```

L'APK è **ARM64, debug-signed**, con le librerie Rust e TinyCC. Il codice Rust
è compilato in release. La build esegue `:app:assembleDebug` e
`:app:testDebugUnitTest`; l'esportazione richiede entrambe le librerie ARM64,
i sorgenti/avvisi TinyCC previsti e i risultati dei test Kotlin. Il checksum
dell'APK esportato viene ricontrollato anche sull'host.

La cartella di output può essere scelta esplicitamente, anche con spazi:

```sh
python3 scripts/android/build_android_docker.py --output "dist/Android APK"
```

## Alberi separati

`AndroidPort/` e il workspace principale hanno sorgenti e versioni differenti.
Lo script non li sincronizza e non sostituisce gli APK già presenti in `APK/`
o `Installer/Android/`. Per compilare esplicitamente l'Android del workspace
principale, dalla radice:

```sh
python3 scripts/android/build_android_docker.py --project-dir . --output dist/docker-android-root
```

Il port resta autocontenuto. Anche dopo aver copiato soltanto `AndroidPort/`,
da quella cartella si può eseguire:

```sh
python3 scripts/build_android_docker.py
```

In questo caso l'output predefinito è `dist/docker-android/` dentro il port.
`--dry-run` controlla gli ingressi e mostra il comando senza chiamare Docker.

## Toolchain, cache e firma

La ricetta `AndroidPort/Dockerfile.android` usa immagini Rust 1.94.0 e
Eclipse Temurin Java 25 fissate per digest, SDK Android 35, build-tools 35.0.0,
NDK 28.2.13676358 e il wrapper Gradle 9.5.0 del progetto. Il target nativo è
`aarch64-linux-android`, API minima 24. I command-line tools Android e i
sorgenti TinyCC sono verificati tramite SHA256; il wrapper verifica il proprio
download Gradle. Versioni e hash dei sorgenti/APK sono nei file esportati.

La prima build scarica immagini e dipendenze. Le cache BuildKit conservano
Cargo, Gradle, compilazioni e la **chiave debug generata dentro Docker**.
Port e workspace principale usano cache distinte. La chiave consente di
aggiornare successive APK prodotte dallo stesso builder/cache; le chiavi di
precedenti build native non vengono importate. Eliminare la cache della firma
cambia la chiave delle build successive. Non vengono lette chiavi personali.

Le immagini di base sono fissate, ma repository dei pacchetti e dipendenze
scaricabili restano servizi esterni: non è una promessa di APK identiche byte
per byte fra macchine o cache diverse. Il codice usa il vero stato dei sorgenti
selezionati, comprese le modifiche locali, anziché un vecchio commit archiviato.

Il launcher prepara un contesto temporaneo con soli ingressi pubblici ammessi.
Esclude ROM, salvataggi, registrazioni, binari generati, `local.properties`,
cache, file nascosti e collegamenti simbolici. Non monta il checkout in scrittura
nel container. L'SDK e i risultati nativi intermedi restano dentro Docker;
solo gli artefatti finali passano nella cartella di output.

## Log e verifiche

Il launcher principale conserva stdout/stderr del builder in
`target/build-logs/android-*.log`, anche quando Docker fallisce, e restituisce
il codice d'errore. L'entry point autocontenuto usa `target/build-logs/` del port.
I percorsi e gli hash dei sorgenti selezionati rimangono nel log.
Se Docker fallisce, il contesto pubblico viene conservato accanto al log in
una cartella `android-…-context`, per poter esaminare gli ingressi esatti del
tentativo. I contesti temporanei delle build riuscite vengono rimossi.

Le verifiche del launcher coprono filtro del contesto, selezione dell'albero,
percorsi con spazi, errori e checksum. Gli adattatori dei builder esistenti
controllano separatamente le convenzioni NDK Windows/Linux senza modificare
l'esecuzione del gioco. La compilazione e i test Kotlin non costituiscono una
prova su dispositivo, del controller fisico, dell'audio o delle prestazioni.

Verifica del 2026-11-01: entrambe le build Docker complete sono riuscite;
**24 test Kotlin** passati per AndroidPort e **12** per il workspace principale,
senza errori né casi saltati. I **7 test** del launcher/adapter/export sono
passati, compresa la prova del port copiato fuori dal repository. Hash APK,
librerie ARM64, archivio sorgente TinyCC e risultati JUnit sono stati verificati.
Docker Desktop/Windows e l'esecuzione su dispositivo restano verifiche distinte.

Fonti della toolchain: [Android command-line tools](https://developer.android.com/studio#command-line-tools-only)
e [compatibilità Java/Gradle](https://docs.gradle.org/current/userguide/compatibility.html).
