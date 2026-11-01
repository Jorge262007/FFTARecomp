# FFTARecomp Android

Port Android autocontenido. Puedes mover o subir **esta carpeta completa**
sin copiar nada de FFTARecomp - Copy ni del proyecto de PC.
El README permanece en la raiz de esta carpeta para que GitHub lo muestre.

- APK para probar: APK/FFTARecomp-android-arm64-preview.apk.
- Interfaz Android / proyecto Android Studio: android/.
- Motor Rust y recompilador necesarios: crates/.
- Compilacion y prueba de camara: scripts/.
- Acreditaciones y licencias: Licenses/.
- No contiene ROM, partidas, entradas grabadas, SDK, NDK ni caches.

## Compilar desde Windows

Tambien se puede compilar con **Docker/Buildx y Python 3.9+**, sin instalar
SDK, NDK, Java ni Rust en el equipo. Desde esta carpeta:

    python scripts/build_android_docker.py

Exporta la APK ARM64, SHA256 y resultados de pruebas a `dist/docker-android/`.
La receta y el script estan dentro de esta carpeta y siguen funcionando al
copiarla por separado. En Windows, Docker Desktop debe usar contenedores Linux.
Los logs quedan en `target/build-logs/`; la primera ejecucion descarga las
herramientas. La firma debug se conserva en la cache Docker y no importa la
clave de instalaciones compiladas anteriormente fuera de Docker.

Necesitas Rust con aarch64-linux-android, Python 3.9 o posterior, un JDK,
Android SDK con NDK 28.2.13676358, y GCC o Clang en PATH para el host.

Desde esta carpeta en PowerShell:

    $env:JAVA_HOME = 'C:\Program Files\Eclipse Adoptium\jdk-25.0.4.101-hotspot'
    $env:GRADLE_USER_HOME = 'C:\Users\shais\.gradle'
    python scripts/build_android.py --sdk D:\Android\SDK

Ajusta esas rutas a tu PC. El resultado se deja dentro de APK/.
El script descarga y verifica TinyCC, compila ARM64, incluye sus avisos
y codigo fuente requerido, compila la APK y ejecuta las pruebas Kotlin.
La primera compilacion necesita las dependencias disponibles o conexion.

## Subir manualmente a GitHub

Puedes arrastrar esta carpeta tal cual. La APK se incluye para tu comodidad:
si no quieres subir binarios, quita solo el archivo .apk de APK/ y
APK/SHA256.txt. Compilar vuelve a generar la APK.
Los archivos ignorados por Git tambien se suben si los arrastras por la web:
antes de una subida posterior elimina los target/, build/, .gradle/
y local.properties que se hayan generado. Conserva gradle/wrapper/.

No se ha movido ni sustituido el proyecto de PC. Esta es una copia
autocontenida de las fuentes Android actuales, preparada para trasladarla.
Los cambios posteriores en el proyecto de PC no se sincronizan automaticamente.

## Estado de la prueba RG Rotate

Android 12, T618, 720x720. Barrido de camara en zoom 6: muestras reales
de pantalla aproximadamente 55-60 FPS, con caidas puntuales; no son 60 fijos.
Queda por mejorar la estabilidad del audio en este dispositivo: se registraron
underruns durante el benchmark. No presentar esta prueba como rendimiento perfecto.

Pasaron la equivalencia pixel a pixel, los hashes framebuffer/OAM del recorrido
y la comparacion de 600 frames sin area adicional. La prueba Android del mando
integrado paso con eventos inyectados usando su identidad real; esto no sustituye
una prueba manual de sus interruptores fisicos.

Para repetir el recorrido, usa scripts/ffta_android_camera_bench.py --help.
Requiere tu grabacion local y una copia de tu partida; no vienen incluidas.

## Optimizacion en curso

El trabajo nuevo continua solo dentro de esta carpeta. Estado y pruebas en
[docs/PERFORMANCE.md](docs/PERFORMANCE.md). No se anuncia rendimiento cerrado
ni audio sin cortes hasta medirlo en el dispositivo objetivo.

Fix de personajes con zoom vertical y audio Normal de fast-forward:
[docs/URGENT_FIXES.md](docs/URGENT_FIXES.md).

Actualizacion del motor PC, UI independiente del zoom y seleccion tactil
experimental (apagada por defecto, Input > Touchscreen):
[docs/EXPERIMENTAL_TOUCH.md](docs/EXPERIMENTAL_TOUCH.md).

Optimizacion de burbujas de dialogo, mediciones en RG Rotate y pruebas:
[docs/DIALOGUE_PERFORMANCE.md](docs/DIALOGUE_PERFORMANCE.md).

Optimizacion posterior del motor, seleccion de nucleos y mediciones completas
en zoom 2, 3 y 6: [docs/FRAME_OPTIMIZATION.md](docs/FRAME_OPTIMIZATION.md).

Optimizacion posterior de dialogos: reutilizacion de filas y recorte por las
ventanas del juego, con pruebas A/B: [docs/DIALOGUE_ROWS.md](docs/DIALOGUE_ROWS.md).

Revision de picos puntuales y entrega de frames a OpenGL:
[docs/FRAME_SPIKES.md](docs/FRAME_SPIKES.md).
