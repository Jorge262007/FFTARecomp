# FFTA: estado de investigacion del fallo real

2026-10-19: demo cinematica v9 optimizada. MEDIDO: mascara diurna fresca
1136x320 20.647 -> 8.065 ms; reutilizada por categorias exactas 0.661 ms.
Extraccion de UI sintetica 3.332 -> 2.192 ms; hashes de pixels sin cambios.
GPU RTX 5050: paridad UI/CPU con scissor, cache y resize aprobada.
NO MEDIDO: FPS sostenidos y picos en menus reales durante una sesion completa.
Detalles y comandos de microprueba en ffta_atmosphere_demo.md.

2026-10-18: Pub USA separado a esquina derecha; tienda USA con lienzo centrado;
aviso de banco vacio USA identificado. Ventanas de servicios compartidas USA
reconocidas por propietario; Prison real sigue NO MEDIDO.
[ffta_usa_town_menus.md](ffta_usa_town_menus.md).

2026-10-17: perfil de mouse para Press Start y menu principal USA implementado
y probado por replay. [ffta_usa_frontend_mouse.md](ffta_usa_frontend_mouse.md).

2026-10-17: usuario confirma efectos de magia ya corregidos; no tocarlos.
Franja Lightning damage medida en WIN0 [16,8,224,24], extraida completa
sin el corte X=160. [ffta_spell_help_banner.md](ffta_spell_help_banner.md).

2026-10-17: menu de acciones de Lester dividido por X=160; su marco real
empieza en X=152. Extraccion segun cabecera/pie de BG2, pendiente de feedback.
[ffta_action_frame_width.md](ffta_action_frame_width.md).
Siguiente pendiente: efectos de magias Thunder/Pyro repetidos a ambos lados.

2026-10-17: punta de oreja de Clara en Giza capturada en vivo y corregida.
[ffta_portrait_ears.md](ffta_portrait_ears.md): dos piezas Y=48/56 excluidas
por el antiguo filtro Y>=64; ahora comparten la capa del retrato.

2026-10-17: Comprar/Vender centrados; categorias compactas de Vender y
Monster Bank medidos por replay con clicks de coordenadas. Detalles y limites:
[ffta_shop_bank_mouse.md](ffta_shop_bank_mouse.md).

Prioridad: reproducir ffta-batalla-3.input antes de afirmar una solucion.
La prueba previa de 44 objetos no reproduce el fallo comunicado por el usuario.
No se aplica otro cambio de logica del juego sin aprobacion.
Sin commits y sin cambios al instalador en esta investigacion.

MEDIDO: AGENTS.md leido y git status --short ejecutado; el repo ya contiene
cambios preparados, cambios sin preparar y archivos sin seguimiento.
MEDIDO: compilacion release inicial correcta, 0.42 s; compilacion de los
diagnosticos correcta, 26.86 s; tests de recomp: 6 aprobados.

MEDIDO: ffta-batalla-3.input termina con un evento en frame 12440.
Contiene eventos de soltar y pulsar en el mismo frame, por ejemplo 11730.
InputScript ordenaba tuplas (frame, mascara), alterando su orden de registro.
Se corrigio el lector para ordenar de forma estable solo por frame.
Esto afecta al replay del host, no a la logica ni a los bytes del juego.

MEDIDO: runc ejecuta la ruta nativa con el mismo Machine, MemMap, run_dma
y PPU que play, sin ventana, audio de escritorio, vsync ni espera por frame.
Se anadio --sav para precargar una copia en memoria; runc no escribe el .sav.
La entrada se reproduce desde frame 0, no desde el inicio del tramo analizado.

MEDIDO: creado E:/AI/ARC/ffta-respaldo.sav sin modificar el original.
La copia, el original y ffta-batalla-3.input.sav tienen SHA256:
71189F7FB6AED638640078FBA3A35FDA6C39C8962E74DCC75935AAC948DA9063.
El replay usa otra copia privada: target/ffta-replay3/start.sav.
SUPOSICION: arranque HLE como en las rutas BIOS buscadas localmente;
se pregunto al usuario si la grabacion utilizo BIOS real.

Comando exacto para el primer replay completo:

```powershell
$env:FFTA_WIDESCREEN='32:9'
$env:FFTA_PATCH_BG='1'
$env:FFTA_PATCH_SPR='1'
$env:FFTA_BG_EDGE='transparent'
$env:FFTA_WIDE_DEBUG=''
$env:FFTA_OAM_LOG='1'
$env:FFTA_OAM_LOG_FROM='10500'
$env:FFTA_OAM_CAPTURE_PREFIX='target/ffta-replay3/run1'
$env:FFTA_BENCH_LIBRARY='target/ffta-movement/native-normal.dll'
.\target\release\recomp.exe runc 'Final Fantasy Tactics Advance (USA).gba' --frames 12540 --input ffta-batalla-3.input --sav target/ffta-replay3/start.sav --out target/ffta-replay3/run1-final.ppm *> target/ffta-replay3/run1.log
```

El DLL indicado es una copia local ya compilada de la ROM del usuario;
no es un archivo que se deba distribuir. Desde frame 10500 el log registra
por frame camara BG, llamadas/argumentos a los tres productores conocidos,
descriptores recibidos, lista host y entradas OAM. Cada 30 frames guarda
hashes de ambos framebuffers, OAM y OBJ VRAM; cada 120 puede guardar una imagen.
MEDIDO: el replay llego a la batalla; se generaron imagenes del tramo final.
Dos ejecuciones completas desde frame 0 dieron 69 muestras identicas de
framebuffer central, framebuffer ancho, OAM y OBJ VRAM, frames 10500..12540.
Salida literal del script: determinism_samples 69 mismatches [].
Hash FNV final de ambas: abc92001e317eae0.
Hash SHA256 central final de ambas:
3ffb91edfe2e1bc047624f0b43c0c30b39770af3ba031d364a260f6b48f66bc2.
NO MEDIDO: confirmacion del usuario de que el arranque grabado era HLE;
la grabacion no guarda metadatos de BIOS y no se encontro BIOS instalada.

MEDIDO: hay nueve instancias graficas identificadas por el puntero
de prioridades pasado al productor menos 0x1e, no por palette ni r5.
En el tramo final desaparecen llamadas de algunas instancias antes de
030065A8; no hay mismatches en la lista host ni el OAM transferido.
Ejemplo: 0201af18 deja de llegar despues del frame 12042, junto al borde
izquierdo original. Falta terminar la medicion de los saltos y de sus tiles.

MEDIDO por lectura: 030062EC, Thumb, origen ROM 08A38EA0..08A38FC8,
calcula visibilidad de una lista con limites 0..239 en X y 0..159 en Y.
08394028 contiene su puntero Thumb 030062ED. El llamador 0808F804 la invoca
en 0808F832 por el thunk 08142254 y solo marca visible si retorna 1.
08021DA8 devuelve NULL si flags & 0x40 == 0; el llamador 0808F1F4 sale
en 0808F216 si obtiene NULL, antes de emitir objetos.
MEDIDO: igualdad de los 296 bytes de RAM con el origen ROM, hash:
1e988998a21067596046e2b303da1ad9b6924c18146f396ce0bf5f6e038433c7.
MEDIDO: en frame de productor 12041, X=-7, la funcion retorna 1 y se
invoca 030065A8 para 0201af18. En frame 12042, X=-10, sus seis componentes
son rechazados a la izquierda; retorna 0 y ya no se invoca el productor.
La traza lee instrucciones reales del interprete en la ruta nativa:

The literal game disassembly was removed from the public source distribution.
Only independently described behavior, hook addresses and verification hashes are retained.

En 0300638A, los valores r0 de los seis extremos derechos son
-2, -10, -2, -10, -2, -10. Todos estan por debajo de 0 pero dentro del
margen de 32:9, cuyo limite izquierdo es -164.
El llamador 0808F804 no marca el resultado de visibilidad cuando retorna 0.
MEDIDO: se pierde antes del productor y no por fallo de publicacion del host.

MEDIDO: los 320 bytes de OBJ VRAM referidos por esa instancia en su ultima
lista recibida permanecen identicos en frames de imagen 12041..12044:
8dac051f7fa7a16c7abd58bbaef877779ae968e1b88dbb2defc903151d6d84eb.
170 bytes no nulos, cero referencias fuera de OBJ VRAM. Por tanto, una
liberacion de estos tiles no explica esta desaparicion concreta.
NO MEDIDO: todos los assets de unidades nunca visibles ni todos los mapas.
El filtrado 03006170 es un ajuste de prioridad por intersecciones, no una
comparacion directa con 240; no se propone modificarlo por similitud.

MEDIDO: gamedb.sqlite no devolvio funciones de esta ROM en 0808E000..08090000;
estas direcciones se obtuvieron de llamadas reales y lectura de instrucciones.
Los desensamblados y logs completos quedan en target/ffta-replay3, ignorado
por git. No se incorporan bytes de ROM ni DLL generados a la distribucion.

MEDIDO por lectura: existe otro productor Thumb en 030068A4, procedente
de 08A39458, con comparaciones contra 239/159; no esta en el enganche actual.
MEDIDO: la observacion de 030068A4 no registro entradas en frames
10500..12539. La tabla del DLL tiene other=false para 030068A5: su entrada
habria pasado por el interprete observado. No participa en este tramo;
NO MEDIDO: su uso en otras escenas o animaciones.
MEDIDO, frames 11334..12539: 080014C8 desde LR 08001D85, 1377 llamadas;
080016D4 desde LR 08001DF3, 1206; ambas con limite 48 y dentro del enganche.
030065A8 desde LR 0808F313, 5645 llamadas, y LR 0808F6CD, 1204.
Los demas llamadores documentados de 14C8/16D4 no se observaron en ese tramo;
las dos funciones se enganchan por PC, independientemente del llamador.

MEDIDO: capturas exactas proof-12042.png y proof-12044.png muestran el
personaje desapareciendo en el margen izquierdo mientras el fondo sigue.
El periodo del productor se etiqueta con bus.frames antes de ejecutarse;
la captura se etiqueta al completarse el frame. La publicacion y el PPU
introducen el desfase visible entre el descarte de 12042 y la imagen 12044.
NO MEDIDO: correspondencia exclusiva de cada componente con una unidad
logica del juego; los identificadores son punteros de instancias graficas.

MEDIDO: tests finales normales: 80 + 7 + 3 aprobados, 0 fallos;
3 pruebas ignoradas por defecto. Prueba nueva del lector de entrada aprobada.
Los diagnosticos de instrucciones y tiles conservaron el mismo hash final
en los replays adicionales. No se modifico la ROM, el .sav original ni
la condicion del juego. La aplicacion de un parche de visibilidad esta pendiente.

Estado anterior, sustituido por la medicion de ocho unidades descrita abajo:
Propuesta minima de enganche por PC del predicado 030062EC,
activo bajo FFTA_PATCH_SPR y solo en widescreen. Conservar la ejecucion
original y sus ciclos; al retornar, usar el resultado calculado para el
rectangulo X=-extra..239+extra, manteniendo Y=0..159. Guardar argumentos
de entrada en el host para ese calculo y comprobar SHA256 de ROM y el
contenido completo del predicado en ROM/RAM antes de habilitarlo.
Extra=0, hash incorrecto o bytes diferentes: resultado original intacto.
Esto cambia el resultado que ve el juego y requiere aprobacion antes de
implementarlo. El productor y la lista i32 existentes seguirian evitando
el descarte posterior y el limite de X de 9 bits para los objetos ampliados.
Pruebas propuestas: los dos replays completos A/B, presencia de la instancia
12041..12046, tiles, OAM original, equivalencia 600 frames con extra=0 y
32:9/1536; revisar tambien el productor 030068A4 si hay llamadas reales.

NO MEDIDO: no se encontro un checkpoint completo utilizable para restaurar
CPU, RAM y perifericos. Las instantaneas IWRAM del recompilador son imagenes
de codigo y no sustituyen el estado acumulado. No se implemento un checkpoint.

Actualizacion: metrica por unidad del tutorial
MEDIDO: enganche X implementado bajo FFTA_PATCH_VIS y FFTA_PATCH_SPR,
comprobando SHA256 y funcion ROM/RAM. Resultado original con extra=0.
MEDIDO: 1201 frames finales: actual falla en 995; propuesto falla en 493.
NO RESUELTO: no hay ocho unidades publicadas en todos los frames.
MEDIDO: seis cuerpos finales siguen en RAM pero reciben rechazo Y antes
del productor. Hay tambien un cuerpo adicional Leslaie, informado separado.
SUPOSICION: referencia ocho corresponde a IDs 0..7, excluyendo ese cuerpo.
MEDIDO: extra=0 conserva las 1201 metricas y 41 muestras de hashes.
Detalles, limites y listas completas en docs/ffta_character_metric.md.
No se ha aplicado la propuesta adicional de quitar el rechazo Y.

Siguiente investigacion: casillas durante zoom-out
MEDIDO POR EL USUARIO: personajes validados jugando en 32:9, ya no
desaparecen. El parche de personajes se conserva sin modificaciones.
Los fallos anteriores de ocho publicados no anulan esa validacion visual;
la definicion del octavo personaje y la diferencia con visibilidad Y
siguen documentadas en ffta_character_metric.md.
MEDIDO: ffta-batalla-4.input y su sidecar no estan en la raiz del repo.
Busqueda rg --files bajo E:/AI/ARC, filtrada por batalla-4, sin resultados.
NO MEDIDO: replay 4, determinismo, lista de rango en RAM, frames del fallo,
condicion del escritor BG0/BG1 y capturas antes/despues. Falta la grabacion.
MEDIDO por lectura: InputScript v1 contiene frame y mascara KEYINPUT;
el grabador escribe solo cambios de keys. No graba zoom_level ni extra_y.
El zoom se procesa aparte en play.rs por Minus/Equal. El replay headless
configure_headless_widescreen solo fija extra X; no reproduce zoom-out.
MEDIDO por lectura: MAX_ZOOM=2, MAX_EXTRA_Y=320; play limita los pasos por
MAX_ZOOM y por ancho/alto maximos. No equivale a poder alcanzar extra_y=320
desde 32:9 con la configuracion actual. El maximo real de la captura falta.
Logging anadido bajo FFTA_OAM_LOG: FFTA_VIEW al cambiar dimensiones,
con frame, nivel de zoom, extra X/Y y dimensiones de vista/render.
No cambia el estado emulado ni el zoom, no toca sprites ni instalador.
No se aplica un parche BG nuevo sin medir antes la causa.

Actualizacion zoom y casillas, prueba manual pendiente
MEDIDO: runner runc admite FFTA_ZOOM_LEVEL=n o calendario frame:nivel.
32:9 niveles 0..6: extra X 164,199,238,282,331,387,448;
extra Y 0,10,21,34,47,63,80. Variable ausente conserva el runner anterior.
MEDIDO: replays completos de batalla-3 coinciden en 1201 hashes finales.
Se construyo movement-freeze.input conservando todos los eventos y anadiendo
12540 3ff. No se requiere otra grabacion del usuario.
MEDIDO: matriz calculada en 02005CB0, 64x64; BG0 copia una ventana 32x32.
En la muestra final, 108 tiles RAM frente a 102 VRAM; 1024/1024 palabras
VRAM coinciden con el recorte RAM de origen (33,35).
MEDIDO: escritor Thumb 0801AF08, llamador 0801B80C. Productor de casillas
0801D130: 20 llamadas style=1, caller 0801D803, coordenadas logicas X/Y
y proyeccion en tiles, cuatro cuadrantes por casilla. La tabla de flags
02008190 contiene marcas parciales de vecinos; contar bytes no cuenta casillas.
MEDIDO: PPU solo escribia 160 filas. En zoom 6 las 160 filas adicionales
quedaban negras; capturas before-11347.png y frozen-13101.ppm.
IMPLEMENTADO PARA PRUEBA: FFTA_PATCH_MOVEMENT=1, requiere PATCH_BG y ROM
exacta, SHA256 de 0801AF08..0801B5D0 y coincidencia de los 1024 valores
de cada plano VRAM con su recorte RAM. Si no coincide usa la ruta anterior.
El host lee los planos RAM completos y termina las filas extra en batalla.
No cambia escrituras originales del juego, sus ciclos, el enganche de
personajes ni la politica de borde del terreno. Por defecto esta desactivado.
MEDIDO: extra=0, 600 frames, MOVEMENT=0/1: framebuffer y OAM identicos.
NO MEDIDO TODAVIA: exito visual del parche nuevo, metrica por casilla antes
y despues, y regresion de los 1201 frames con el parche nuevo activo.
Prueba manual preparada en target/ffta-zoom/manual/FFTA-movement-test.gba;
su save privado procede de ffta-respaldo.sav. No se toco ningun save original.
Actualizacion: el usuario confirma que las casillas ya no desaparecen.
MOVEMENT esta activo por defecto, con las mismas verificaciones de ROM y RAM.
MEDIDO: el escenario conserva planos RAM 64x64 en 0200B1A0 y 020091A0;
VRAM contiene recortes 64x32. El parche STAGE lee esos planos completos solo
tras verificar el recorte entero (2048 palabras por capa). No modifica el juego.
MEDIDO: comparacion STAGE=0/1: 1861 frames de personajes sin cambios,
incluidos los 1201 finales de la grabacion; 63 muestras de framebuffer central
y OAM sin cambios. Frame final: 8 personajes, sin grupos duplicados.
MEDIDO: extra=0, comparacion por frame durante 600 frames: framebuffer y OAM
identicos. Capturas stage-off-13101.png y stage-on-13101.png muestran mas tejado
y arquitectura lateral. La confirmacion en juego real sigue NO MEDIDO.
MEDIDO: las filas adicionales de zoom ahora se dibujan, no quedan vacias.
Input contiene Keyboard y Gamepad, remapeo y layout Xbox por defecto.
NO MEDIDO: mando real; no se prueba por instruccion del usuario.
Instalador Linux: codigo, script y workflow preparados; ejecucion NO MEDIDO.
La carpeta assets tiene referencias de compilacion y no se elimina.

Correccion del cielo durante zoom:
MEDIDO: replay frame 13100, palette0 cambia por linea: 7D21,7DA4,7E67,
7F2A,7FCC en lineas 0,40,80,120,159. El gradiente es raster de paleta,
no una capa affine o bitmap como se afirmo antes sin medir.
Las filas host adicionales leian la paleta de VBlank (azul inicial).
Ahora conservan los colores de los extremos antes de VBlank y prolongan
esos colores fuera de las 160 lineas. No se copian filas del escenario.
Actualizacion: prolongar un color fijo seguia dejando una banda lisa.
El host ahora extrapola por canal RGB555 el gradiente observado entre
las lineas 0 y 159; los pixeles originales de 240x160 no cambian.
MEDIDO: en frame 13101, filas 0/79/80/239/240/319 cambian RGB
de 0,0,255 a 8,74,255 a 99,247,255 a 140,255,255 sin salto en
los bordes de la pantalla original. Captura sky-extended-13101.png.
MEDIDO: 600 frames extra=0 framebuffer y OAM iguales; 1201 frames de
personajes del tutorial sin cambios respecto al parche validado.
Retiradas heuristicas OBJ VRAM/paleta/OAM no vacias: no resolvian las
siluetas segun el usuario y anadian exploraciones de memoria por linea.
Se intento una comprobacion de la paleta propia de cada pieza OBJ en
widescreen: evita dibujar una pieza cuando su banco de 16 colores esta
completamente vacio. No se ha confirmado si elimina las siluetas reales.
NO MEDIDO: causa de las siluetas transitorias; no aparecen en las muestras
10020,10200,10400,10600 del replay existente. No se dan por arregladas.

Zoom de escenario durante menus y cinematicas (2026-10-12):
MEDIDO EN CODIGO: la ruta de planos completos solo aceptaba BG2/BG3 y
exigia ambos fondos activos con scroll identico. El detector anterior tambien
admitia terreno en BG0/BG1, pero esas capas no leian el plano RAM completo.
CORRECCION: identificar cada plano por BGCNT 4C03/4E02 y su estado RAM
ya verificado, sin depender de la activacion ni del scroll de su pareja.
Se conserva la verificacion de ROM, FFTA_PATCH_STAGE y FFTA_PATCH_BG.
No se modifican datos del juego ni el parche de personajes.
MEDIDO: tests gba-core: 85 passed, 3 ignored. Prueba sintetica de pareja
oculta, cambio BG0/BG2, extension vertical y 600 frames con extra=0 iguales.
NO MEDIDO: replay del fallo real, hashes de framebuffer/OAM de 600 frames
reales y metrica de 8 personajes. No hay grabaciones .input en el repo
ni en la raiz E:/AI/ARC. Requiere validacion jugando; no se afirma resuelto.

Panel de stats y zoom (2026-10-12):
MEDIDO EN CODIGO: las filas extra excluian capas UI, pero dibujaban OBJ.
Cambio: incluir todas las capas texto activas tambien en las filas extra;
las UI no repiten verticalmente fuera de su mapa de tiles. Ruta extra=0 intacta.
MEDIDO: 86 tests passed, 3 ignored; prueba de panel en y=180 y no repeticion.
NO MEDIDO: reproduccion visual del panel del enemigo de la captura del usuario.

Panel al pasar el cursor sobre unidades (2026-10-12):
MEDIDO: replay derivado zoom-no-range.input recuperado del repo anterior.
La grabacion ffta-batalla-3.input de ese repo contiene solo su cabecera.
Dos ejecuciones desde frame 0 hasta 13000: hash f158877f466426fb igual.
Se usa exclusivamente target/ffta-stats/repro.sav, copia del respaldo.
MEDIDO: el panel de Ritz no es un BG: productor 080014C8, lista 083916FA,
cuatro piezas semitransparentes, tiles 1F0/200/208, paleta OBJ 11.
Todas las piezas y tiles existen. BLDCNT=3F43, BLDALPHA=090E: el fondo
se mezcla con terreno y parece parcialmente ausente; no es descarte de piezas.
Cambio de presentacion: FFTA_PATCH_STATS=1 por defecto vuelve opaco solo
ese fondo identificado en la lista host de la ROM exacta y funciones verificadas.
No cambia atributos OAM ni logica del juego; extra=0 conserva la ruta original.
MEDIDO: 87 tests normales y 3 con ROM local pasan. Frame 13000 cambia
2558 pixels, todos dentro de x600..687/y184..231; exterior identico y OAM igual.
MEDIDO: framebuffer y OAM iguales en 600 frames con extra=0, STATS=0/1.
Capturas hover-13000.png y after-13000.png en target/ffta-stats.
NO MEDIDO: panel rojo de enemigos; el replay construido no llego a mostrarlo.
La metrica de este recorrido derivado tiene frames con menos de 8: no se
considera aprobada la prueba de 8 constantes ni se toca el parche de personajes.
MEDIDO: comparacion STATS=0/1 en 1201 frames (11800..13000): metrica
identica en todos. 297 frames dan menos de 8 en ambas versiones de este
replay derivado con la biblioteca nativa recuperada; no es una regresion nueva.
La lista completa queda en target/ffta-stats/character-failures.json.
Windows Installer/payload y TAR Linux compilados y actualizados, sin commits.

Correccion del intento anterior de stats (2026-10-12):
El usuario confirma que quitar semitransparencia NO arreglo el recorte.
Se RESTAURA siempre el modo semi original; se descarta el intento opaco.
MEDIDO en VRAM: las cuatro piezas del fondo contienen una curva con
pixels transparentes: cobertura de 67 pixels en fila 104 y 88 en fila 127.
Esto es distinto de las capas BG del menu de acciones.
Cambio host: completar solo los texels transparentes de esas piezas
identificadas, usando el indice de color vivo del tile rectangular 208.
Se preservan paleta, letras, atributos OAM, prioridad y alpha original.
FFTA_PATCH_STATS=0 conserva forma original; 1 completa el fondo en widescreen.
No es una recuperacion de sprites o tiles ausentes: es completar la geometria
del fondo del panel, limitado a la plantilla verificada y sin tocar la ROM.
MEDIDO: 88 tests passed, 3 ignored. Test de huecos, color vivo, semitransparencia
y extra=0. Nueva comparacion real de 600 frames: framebuffer/OAM iguales.
NO MEDIDO: panel rojo; la validacion visual del usuario sigue pendiente.
MEDIDO: replay completo hasta 13000 con el cambio nuevo. Fondo sin huecos
y alpha preservado: cap-after-13000.png. Se completan 217 pixels, todos
dentro del panel; fuera del panel identico y atributos OAM identicos.
MEDIDO: metrica de personajes identica en los 1201 frames de STATS=0/1.
Los 297 frames con menos de 8 del recorrido derivado permanecen iguales.
El comando PowerShell devolvio 1 pese a llegar al frame 13000 y generar
los volcados; el log registra NativeCommandError para mensajes de stderr.
Los verificadores posteriores terminan con codigo 0; no se oculta ese retorno.
Windows y Linux compilados y actualizados, sin tocar saves originales.

Objetivo correcto del panel de stats (aclaracion del usuario):
El semicirculo NO estaba mal. El fallo es que el panel no llega al borde.
Se REVIERTE completar sus texels transparentes. Forma y alpha originales.
Nuevo STATS=1: solo las piezas rectangulares del tile 208 se prolongan
hasta el borde del framebuffer ancho. H-flip elige izquierda o derecha.
Se repite su columna exterior viva, con su textura, paleta y semitransparencia.
Capas, posiciones de texto, curva, OAM y logica de personajes intactas.
MEDIDO: 88 tests passed, 3 ignored. Test de ambas orientaciones, curva
sin rellenar, alpha y no extension con STATS=0 o extra=0.
MEDIDO: comparacion real de 600 frames extra=0: framebuffer/OAM iguales.
MEDIDO: reconstruccion del volcado privado en edge-preview.png muestra
la banda transparente hasta el borde derecho, con la curva conservada.
NO MEDIDO: replay completo nuevo de 1201 frames, panel rojo y validacion
jugando del usuario. El preview reconstruye el volcado, no es una captura nueva.
Windows y Linux compilados; Installer/payload y TAR actualizados, sin commits.

Menu de pausa nativo adaptado del HTML del usuario (2026-10-13):
Estilo pergamino, gradientes crema, bordes laton, esquinas ornamentales,
seleccion cuero/dorado y dibujos propios para iconos. Sin motor HTML,
red, dependencias nuevas ni fuentes o iconos web empaquetados.
Se conserva la fuente pixel nativa: Cinzel/Inter y los efectos Web Audio
no se trasladan literalmente. Valores reales de av.cfg/input.cfg conservados.
Input abre pestanas Keyboard/Gamepad y conserva el dispositivo elegido.
Listas desplazables por teclado/mando/rueda; boton Back siempre visible.
Selectores video desplegables con aceptar/cancelar; FF segmentado clickable.
Exit pide confirmacion con No seleccionado por defecto.
MEDIDO: 42 tests passed: input-config 12, menu 19, recomp 11.
Pruebas de dropdown, targets de remapeo con scroll, tabs, FF y salida segura.
MEDIDO: previews nativos generados/revisados para main, gamepad, FF,
salida y selector. Windows/Linux release compilados y payload/TAR actualizados.
NO MEDIDO: interaccion jugando con ventana/mando fisico y efectos del GPU
sobre el estilo; no se inicio una partida ni se tocaron saves originales.
Referencia HTML en target/pause-menu/reference.html; previews en esa carpeta.

## 2026-10-13 - Menu moderno e iconos de mando completos

Menu renderizado en un canvas independiente de la GBA, sin alterar el framebuffer
del juego. Cinzel/Inter con antialiasing y avisos OFL en Licenses/fonts.
Iconos vectoriales para A/B/X/Y, LB/RB, LT/RT, View/Menu/Guide, L3/R3,
las cuatro direcciones del D-pad y las direcciones de ambos sticks.
Ratones usan coordenadas del canvas; GPU y fallback CPU presentan el mismo menu.
Asignaciones y parches del juego conservados. Preview: target/pause-menu/gamepad-modern.png.
Prueba fisica del mando: NO MEDIDO.

## 2026-10-16 - TinyCC en memoria para Windows y Linux
MEDIDO: mismo TinyCC que Android, fuente 43c7708b85681a2fd4451c8a541af4494a8919b2.
MEDIDO: EUR build completo 3.611 s; USA 4.706 s; USA GCC anterior 116.634 s.
MEDIDO: cache EUR 0.079 s; framebuffer/OAM iguales a GCC en 600 frames de ambas ROMs.
MEDIDO: USA extra=0 identico con parches off/on en 600 frames.
MEDIDO: 141 tests existentes y 2 pruebas TinyCC adicionales pasaron.
MEDIDO: replay 12540 frames igual a GCC; metrica antigua cuenta 4 en ambos, no valida 8.
MEDIDO: Windows y Linux compilados; no se empaqueta GCC. TinyCC incluye su fuente exacto y licencia.
NO MEDIDO: ejecucion Linux, no hay WSL instalado en este host.
Detalles y limites en docs/desktop_tinycc.md. Sin cambios en sprites, fondos o gamepad.

## 2026-10-16 - Dialogos anclados al borde y tamano independiente del zoom
MEDIDO: snapshot EUR separa BG2 texto, BG3 fondo y cinco piezas OAM 8-bpp de retrato.
MEDIDO: previews 32:9 x1/x2 muestran texto, fondo y retrato juntos al borde, sin extension.
MEDIDO: 99 tests core pasaron; 600 frames extra=0 con layout off/on dan framebuffer/OAM identicos.
Windows compilado en recomp-dialogue-layout.exe y copiado al recomp.exe habitual y payload.
NO MEDIDO: validacion jugando; no se afirma validacion visual USA ni otros formatos de dialogo.
Punto futuro de Settings: MemMap.ffta_dialogue.style (scale, offset_x, offset_y), sin menu nuevo.
A/B: FFTA_DIALOGUE_LAYOUT=0/1, por defecto 1. Detalles en docs/dialogue_layout.md.


2026-10-16: Text Bubbles submenu under Video: persistent size and backing length.
Exact desktop zoom/crop now drives uniform dialogue scale. Host-list objects cannot
be treated as portrait pieces. Parked sentinel filter also covers horizontal-only wide.
MEDIDO: 157 tests passed; EUR extra=0 framebuffer/OAM identical for 600 frames.
NO MEDIDO: visual elimination of startup flashes and appearance on the user display.


2026-10-16 portrait follow-up: corrected host-list portrait exclusion introduced
in prior dialogue update. Host portraits now share text/backing placement and scale.
MEDIDO: both edges x 3 sizes x 7 zooms, host and OAM rendering identical with
expected portrait bounds; 158 tests passed, 4 ignored. No game logic or saves changed.


2026-10-16 independent dialogue UI layer (desktop, first migrated profile).
Native BG2/BG3 and portrait capture; world pass excludes them. GPU draws UI once
in physical window coordinates, independent of world zoom. Software and pause
paths included. FFTA_UI_LAYER=0 restores the preceding layout for comparison.
MEDIDO: 189 suite tests pass; separate RTX 5050 offscreen GPU/CPU comparison passes.
MEDIDO: EUR and USA extra=0 framebuffer/OAM equal for 600 frames each.
MEDIDO: real saved snapshot UI data and 72,855 opaque output pixels unchanged
at x1, x1.122462 and x2. Captures target/ui-layer-after-*.png.
NO MEDIDO: live user transitions and 8-character replay metric in this change.
Other menu profiles remain on existing paths; Android and installer code unchanged.


2026-10-16 dialogue continuation marker: measured USA replay frame 2141,
OBJ attrs 803B/0088/2122, tile 0x122, 8x16, palette 2, priority 0. Its graphics
match the existing EUR snapshot. Marker now shares dialogue layer, retaining
bounce and disabled state; tested with raw OAM and host replacements.
Text Bubbles moved to root directly after Fast Forward; eight rows fit.
MEDIDO: reconstructed before/after captures at x1/x2 confirm attachment.


2026-10-16 inward Size pivot: verified lower dialogue clipping in open window.
Native backing/portrait bounds choose upper/lower screen-edge anchor; Size grows
uniformly toward the middle, leaving space for the continuation marker without
following its bounce. Measured same live scene reconstructed at 110/200 percent.
100-percent image unchanged; EUR/USA extra=0 framebuffer/OAM equal for 600 frames.
192 tests pass, 5 ignored. Windows build: target/release/recomp-bubble-inward.exe.
User's existing game process was not closed; its in-use executable may remain old.

2026-10-16 independent action and hover-stat panels:
MEDIDO: actual EUR open-actions snapshot BG2=5C08/BG3=4A09; native left
information/portrait and right action list extracted into two separate layers.
Stats recognized from complete four-piece palette-11 OAM backing; glyph palette
12 retained. Native alpha blends against raw top BG, not world OBJ or an already
blended BG. Reused buffers include serial and parallel zoom rows.
Panels anchor to their screen corners and Size grows uniformly inward; world
zoom does not affect native UI size. Length remains dialogue-only.
MEDIDO: actual EUR action and USA stats snapshots reconstruct original hardware
framebuffer with changed_pixels=0, including Size=200 percent output captures:
target/panels-actions-after-200.png and target/panels-stats-after-200.png.
MEDIDO: 196 tests pass, 5 ignored; separate RTX 5050 GPU/CPU test passes.
MEDIDO: USA and EUR extra=0 framebuffer AND OAM equal for 600 frames each.
NO MEDIDO: new live user transitions and 1201-frame eight-character metric.
No sprite producer, installer logic, Android source, ROM or original save changed.
Windows executable: target/release/recomp-panels-corners.exe.
FFTA_UI_LAYER=0 retains old world UI for A/B comparison.

2026-10-16 panel Length: actions, portrait/info and hover stats now use the
persisted Text Bubbles Length for horizontal scale about their existing corner.
Size remains uniform scale; Length leaves vertical dimensions unchanged.
MEDIDO: 47 screen/recomp tests pass, 1 ignored; actual saved USA stats and EUR
actions reconstruct native UI with zero changed pixels at Length 150 percent.
MEDIDO: USA/EUR extra=0 framebuffer and OAM identical over 600 frames each.
Windows built: target/release/recomp-panels-length.exe; payload updated.
Captures: target/panels-actions-length150.png, target/panels-stats-length150.png.
NO MEDIDO: live user check. No sprite/game logic or installer source changed.

2026-10-16 Length correction: removed horizontal texture scaling for compact
panels. Capture native backing independently from glyphs/portraits/cursor.
Stats backing uses palette-11 pieces, excluding palette-12 glyph pieces.
Length translates foreground by an integer native offset toward screen center;
only the backing is extended using an interior column, keeping the outer cap.
Size still scales both axes equally; GPU/software/pause use the same packed UI.
MEDIDO: complete test suite passes, including rigid glyph translation, fixed
outer cap and exact Length=100 percent pixels. Actual saved EUR action/USA
stats native reconstruction: changed_pixels=0. Length=150 percent captures:
target/panels-actions-length-content150.png and target/panels-stats-length-content150.png.
MEDIDO: USA/EUR extra=0 framebuffer AND OAM identical for 600 frames each.
NO MEDIDO: live user transitions and new eight-character replay metric.
Windows: target/release/recomp-panels-length-content.exe; payload updated.
No guest logic, sprite producers, installer source or original saves changed.

2026-10-16 panel border completion:
MEDIDO: read-only live EUR snapshot PID 9292 at Length=200 percent shows action
header/footer and info strip cut short. BG2 action frame tiles 005..00A and
0A8..0B9 are decoration, not ordinary action labels. Include these in backing.
Background-only capture now ignores foreground world OBJ: portrait silhouettes
must not punch holes into the blue plane used for Length completion.
Use a fixed flat-side stretch seam, not one following each row's curve.
Separate native stats capture excludes legacy EUR host gap padding; original
cap geometry is retained in both orientations. Guest sprite producers unchanged.
MEDIDO: 201 tests passed, 5 ignored; tests cover action border retention,
background under portrait, mirrored rounded caps, and legacy host padding.
MEDIDO: saved native UI reconstruction zero changed pixels, including Length
100/200 and zoom x1/x2, Size100/150 percent. Before/after captures:
target/panels-current-window.png (actual original window),
target/panels-borders-actions-before.png / panels-borders-actions-after.png,
target/panels-borders-stats-before.png / panels-borders-stats-after.png,
target/panels-borders-actions-zoom2-size150.png (snapshot reconstructions).
MEDIDO: USA/EUR extra=0 framebuffer and OAM identical over 600 frames each.
NO MEDIDO: actual red enemy hover scene after fix; cap orientation regression
uses generated mirrored fixtures. No new 1201-frame character metric run.
Windows build target/release/recomp-panels-borders.exe; payload updated.
No installer source, original save, ROM or Android source modified. No commits.

2026-10-16 action-menu jitter:
Cause: full UI pixel bounds included animated OBJ cursor. Its varying left edge
changed Length displacement for the entire panel. Use static backing bounds for
Length distance and action panel anchoring. Cursor animation stays in foreground.
MEDIDO: saved real EUR menu snapshot with only cursor X changed to 171/173/175/177:
all UI rects identical; packed menu excluding cursor animation has identical SHA256
084a981ed32e54d7c077a13699e921167230f687534350701c698d120ddea7a7.
Captures target/menu-jitter-171.png through menu-jitter-177.png.
MEDIDO: 202 tests pass, 5 ignored; new regression includes cursor blink and Length
100/125/150/200 percent. USA/EUR extra=0 framebuffer AND OAM equal for 600 frames.
NO MEDIDO: live user validation after fix; live read-only sample had no matching
active menu cursor, so saved snapshot variants are a controlled reproduction.
Windows: target/release/recomp-menu-stable.exe; payload updated.
No installer source, guest producer, ROM or original save changes; no commits.

2026-10-16 target attack forecast in native screen-space UI:
MEDIDO: captured live EUR window PID6968 and six graphics/RAM regions read-only.
BG attacker signature 5C08/4A09 coexists with target OBJ backing 1F0/200/208.
Bug: begin_dialogue_frame detected Stats then unconditionally replaced it with
Actions. Target OAM remained in zoomed world. Preserve detected Stats beside BG
attacker info. Forecast glyphs share bank11: backing-only capture must identify
backing tiles, not the entire bank (glyph 228/230/238 must stay foreground).
MEDIDO: real EUR snapshot UI reconstruction changed_pixels=0 at zoom x1/x2;
UI layout rect identical, native target UI byte hash identical:
80F72591E91B9BC939864EFCEC4B82A1FEBFF6647AB755651B835EC5E3148841.
Captures target/forecast-live-window.png (before, actual window),
target/forecast-after-x1.png / forecast-after-x2.png (reconstructions),
target/forecast-after-size125-length200.png (Size/Length verification).
MEDIDO: 203 tests pass, 5 ignored, including both regional profiles, two zooms,
combined attacker BG and target OBJ, bank11 glyph exclusion, original OAM intact.
MEDIDO: USA/EUR extra=0 framebuffer and OAM equal for 600 frames each.
NO MEDIDO: actual USA target screenshot and live user verification after fix;
no new 1201-frame eight-character metric. No guest producer changes.
Windows target/release/recomp-attack-forecast.exe; payload updated.
No installer source, Android source, ROM or original save writes; no commits.

2026-10-16 experimental mouse, EUR language selector:
Read-only investigation plus opt-in semantic target layer (FFTA_MOUSE=1).
Five flag/text OAM groups and quill signature gate interaction; exact EUR SHA256
and short function bytes must match. Pointer0201C3D4 -> structure0200F3D4;
selection+2=0200F3D6, Thumb write0813FFCA. No guest memory/function modifications.
Hover navigates through ordinary Down pulses; click confirms once with A.
Targets follow existing viewport/zoom inverse transform. Per-frame virtual keys
recorded for replay. Manual buttons, pause/focus/scene loss cancel automation.
MEDIDO: all five target requests reached; one confirmation edge;206 tests pass,
5 ignored.700-frame interpreter and two native replay PPM files byte-identical,
hash d17fc597c1fb9325. USA/EUR extra=0 framebuffer/OAM equal for600 frames.
NO MEDIDO: physical mouse in user window, touch, other game scenes. Disabled by
default; no original save writes, ROM distribution, installer source or commits.
See docs/ffta_mouse_language.md. Windows target/release/recomp-mouse-language.exe.

2026-10-16 mouse direct selection:
User requested skipping intermediate languages. Measured direct selector-byte
commit updates the original quill/text palettes in two frames with no intervening
languages. Controller now commits just the validated selection byte between
emulated frames, then original game redraws; A confirmation remains one edge.
Exact EUR hash, short original bytes and complete current scene/pointer bounds
required. Recording adds FRAME mouse:INDEX; frames/runc/verify use guarded replay
and fail on scene/ROM mismatch. Historical key-only inputs remain compatible.
MEDIDO: all25 source/target pairs tested; real controller reaches4,2,0,3,1 with
no directional keys, no intermediate quill or palette state; one confirmation.
MEDIDO:208 suite tests pass,5 ignored. Interpreter/native full PPM byte equality
at all five destinations304/308/312/316/320;370-frame final also identical to
repeated native replay, hash d17fc597c1fb9325. Private target/mouse-direct-* files.
MEDIDO: USA/EUR extra=0 framebuffer AND OAM equal over600 frames without requests.
NO MEDIDO: user's physical mouse after direct-selection change. Experimental
FFTA_MOUSE=1 still required; only EUR language selector supported.
Windows target/release/recomp-mouse-direct.exe; payload updated. No original save,
ROM, installer source, Android or sprite producer changes; no commits.

2026-10-16 reusable invisible mouse grid:
Implemented interaction_grid.rs:8x8 logical cells across viewport, exact target
bounds within shared cells, EventId(context,option), layers/modal blockers,
cell tracking and hover entry/exit/click. Unchanged targets reuse the index.
EUR language adapter now routes through the grid; direct state commit unchanged.
MEDIDO:214 tests pass,5 ignored. Exact reference match at153600 test positions;
all38400 native language pixel centers match the previous target rectangles.
Actual EUR run:600 cells,1 build, five direct selections, one confirmation.
Five rendered images and input events byte-identical to direct-rectangle build.
Native/interpreter replay equal,370-frame hash d17fc597c1fb9325.
USA/EUR extra=0 framebuffer AND OAM identical over600 frames each.
NO MEDIDO: user's physical mouse with this build, new menu event adapters, any
speed gain over the five-option prototype. Main menu and battles not started.
Docs ffta_interaction_grid.md. Windows target/release/recomp-mouse-grid.exe.
No original save, ROM, installer source or Android modifications; no commits.

2026-10-16 EUR main menu mouse:
Reused invisible grid for Press Start, New Game, Saved Game, Color Mode,
Language, name keyboard/categories/Delete/OK and name Yes/No. Direct verified
selection bytes and original confirm/back keys; queued clicks cannot cross
menu contexts. Scene/ROM checks fail closed. FFTA_MOUSE=1 remains experimental.
MEDIDO: actual full-boot click scripts traverse the front-end, select TV,
Resume/Load and start New Game; letters/categories/name confirmation exercised.
Native1490-frame click replay equals interpreter pixels. USA/EUR extra=0
framebuffer AND OAM equal previous600-frame references.
NO MEDIDO: populated save-file slots/resume (all available saves blank), physical
mouse acceptance, USA mouse front-end and battle mouse controls.
Windows target/release/recomp-mouse-main.exe; docs/ffta_mouse_frontend.md.
No original save/ROM, installer source, Android or sprite-producer changes.
No commits.

2026-10-17 experimental battle hover:
Added ffta_battle_mouse.rs: reads logical stage dimensions, bounds, height/flags,
range styles and camera projection. Mouse movement queues a logical destination;
original cursor handler commits it. Left click confirms via original A after
commit; right click B. Exact USA/EUR ROM profiles and short code checks required.
FFTA_MOUSE=1 opt-in; action menus and unsupported cursor modes stay excluded.
MEDIDO:196 existing tutorial cells,20 range-style1; original projection matches
196/196 and range query256/256. Real USA full replay: three direct targets stay
exact in interpreter and Windows native32:9 zoom6; no intermediate arrow walk.
Single-frame wake pulse fixes an observed extra-step failure in the first build.
MEDIDO:221 tests pass,5 ignored; USA/EUR600-frame extra=0 framebuffer/OAM match.
Character metric11340..12540:1201 frames, exactly8 present, no duplicates.
EUR isolated original-function test passes on copied RAM; original scripted
mode4 was explicitly overridden only in that test, not in runtime behavior.
NO MEDIDO: full EUR manual hover, all stages/slopes, battle action menu mouse.
Windows target/release/recomp-mouse-battle.exe. docs/ffta_mouse_battle.md.
No original save/ROM, installer source, Android or sprite-producer edits.
No commits.

2026-10-17 battle input follow-up, awaiting user gameplay test:
Live vertical lists use their instance count/cursor rather than option names.
Original navigation commit USA080287A0 / EUR080299DC selects the requested row.
MEDIDO: USA replay jumps0->3->0, opens Action and Snowball; first one-frame wake
was missed by the menu polling cadence and replaced with wake-until-commit plus
suppression of repeated steps. Exact ROM and short function-byte checks gate it.
Follow-up adds plain left=A/right=B outside semantic targets, including dialogue
and forecasts, and fixes the field gate when unused BG layers are disabled.
Wait-facing arrows adapt mouse direction and queue original direction then A.
Name typing supports ASCII letters/digits, Backspace and Enter; original glyph
selection and keys insert the name. EUR3/USA2-page profiles are checked.
MEDIDO:10 battle-input tests and9 frontend/name tests pass; earlier filtered
FFTA suite37 pass,4 ignored. NO MEDIDO: user gameplay acceptance of follow-up.
Windows target/release/recomp.exe. No installer/save/ROM edits or commits.

2026-10-17 desktop mouse follow-up (experimental, off by default):
Input now has Keyboard / Gamepad / Mouse and Keyboard. The last uses its own
persisted ten-button profile (WASD, Q/E), preserving the Keyboard bindings.
The runtime and launcher select the active profile; assets/cursor.png is original.
MEDIDO: Do it/Cancel is a separate two-button handler, USA08029350/EUR0802A58C.
The adapter reads its live button records and drives original input, including
deferred A after the requested row. A controlled call selects Cancel successfully.
MEDIDO: world-map cursor logical stores USA080375CE/080376A2 and EUR08038B3A/
08038C0E, reached through the original input routine. Native replay matches the
interpreter framebuffer; requested native screen points remain exact after release.
Town window lists and nested shop/pub text lists use live row counts/geometry.
Two cursor styles are handled. Polling lists release each direction after an
observed step because the pub list consumes edges instead of repeating held input.
MEDIDO: extra=0 framebuffer and OAM identical in600 frames against the previous
executable; tutorial regression8 present/8 drawn in all1201 final frames, no duplicates.
NO MEDIDO: every mission/inventory paging state and manual clicks at every scale.
Pending: reported end-turn facing issue and repeated thunder/effects in32:9.
Android port is deferred; mouse remains off by default there. No installer,
original ROM/save or sprite/PPU renderer edits. No commits. docs/ffta_mouse_world.md.
2026-10-19 cinematic v16 / opt-in EUR debug victory:
MEDIDO: separate UI now blends over the lit stage; removed Interior rectangle
lighting bypass. Original non-cinematic RGB555 composition remains unchanged.
48 screen tests pass, 1 ignored; explicit RTX5050 UI GPU test passes both paths
at two zooms. Interior/fog=0 previews with/without UI bounds are byte-identical.
MEDIDO: FFTA_DEBUG=1 enables F8, exact EUR SHA256 plus short result/controller
code checks. Original result state106 runs, then scene state8 performs cleanup.
Native full-boot tutorial replay: requested at23000, result finished23394,
frame24800 shows Cyril world map. No cartridge save loaded or written.
Core suite in series:164 passed,5 ignored. Parallel PPU tests collide through
global render dimensions (11 failures); series rerun passes unchanged tests.
600-frame extra=0 regression versus v15: framebuffer FNV d163983309983bb3;
OAM SHA256 fc0ddc7814251e837ed37568baf9f2a757aaf1d6da9ffb647ac24cec14a92311;
both identical. NO MEDIDO: every mission's script/rewards and manual UI approval.
Windows target/release/FFTARecomp-Debug.exe. F8 is a developer skip, not a combat
simulation. No original save, installer, Android, ROM or commits modified.

2026-10-20 F8 developer menu (partial, EUR only):
F8 now opens a paused native menu, never wins the battle directly.
FFTA_DEBUG=1 and the exact EUR hash are required. Tools start OFF; enabling
them allows the existing verified victory action. Turning them OFF prevents
new requests; it does not undo a completed victory. No debug setting persists.
Each debug launch reads the resolved save into a new ffta-debug-saves/session
copy; autosave and teardown write only that copy, never the source save.
Mission IDs 1..405 can be browsed with arrows or a dropdown. These are internal
records, not 405 verified playable stages. Loading remains LOCKED: the probe
changes to the battle callback but leaves only sky, without terrain or units.
Unlock characters remains LOCKED: original recruitment calls produced five
special recruits on copied initialized RAM, but other offers rejected creation;
raw unit construction also produces NPC variants. No all-character claim.
Private probe outputs are ignored under target; no ROM assets/code added.
MEDIDO: menu suite26 passed; debug victory guards4 passed. Windows release
build succeeded (29.80s); target/release/FFTARecomp-Debug.exe13973504bytes.
1280x720 developer menu preview generated and visually inspected. NO MEDIDO:
manual F8/gamepad interaction, full mission loading or live character unlock.
No new600-frame/eight-unit regression run for this menu-only integration;
the PPU and existing sprite patches were not edited in this step.

2026-10-20 developer menu follow-up:
Developer tools now defaults ON in new processes. Mission dropdown has wheel
scrolling, a visible scrollbar and click-to-seek on its track. Mission loading
and all-character unlock remain explicitly LOCKED, not bypassed by this flag.
Shader effects submenu applies session-only live GPU controls: master switch,
profile, fog0..3, sun shafts, bloom, vignette and detail accents. Every change
or Print settings to terminal emits FFTA_SHADER_CONFIG with mission/frame,
all switches and fixed sun data. No presenter reports applied=false.
Manual workbook docs/ffta_stage_shaders.xlsx created with blank configuration
records0..405 and Guide. Not a validated level catalogue or automatic loader.
See docs/ffta_shader_debug.md. No installer, original save, ROM or commits.

2026-10-20 pub lighting v17:
Inspected live pub screenshot. Interior detector now accepts broad sconces,
their split warm cores and pale/gray window glass. Stable window fill, warm
material-preserving local spill and small flame-only flicker; no moving sun.
Reference preview detects7 visible sources (3 windows,3 sconces,1 lantern).
This is a sampled displayed image, not a native scene dump. Hidden sources and
live new-build gameplay/performance remain NO MEDIDO. UI/sprite source exclusions
retained.51 screen tests pass,1 ignored; RTX5050/Vulkan preview passes.
Workbook Pub row=TESTING. Details/limits: docs/ffta_pub_lights.md.

2026-10-20 authored GPU Pub lights v18:
New explicit source data in assets/lighting/pub.csv replaces the color detector
for the recognized Pub rig. Per-fragment distance falloff, color/intensity,
Lambert response and approximate image-derived relief. No true3D occlusion.
F8 Shader effects: Point lights, Mark torch/window and Undo last marked light.
Marking freezes emulation, hides extracted UI and consumes the placement click.
Native canvas coordinates retain their positions through zoom/aspect changes.
Config files reload live; full light properties log to the terminal. Seven
front-wall/lantern defaults; hidden fixtures are not invented.
GPU same-image tests: empty/all468144 changed pixels; moved torch436916;
torch time231233; window time byte-identical.29 menu and55 screen tests pass,
1 hardware test ignored. Exact Pub/shop identity test passes. Live placement,
all hidden sources and gameplay frame times NO MEDIDO. See ffta_point_lights.md.


## 2026-10-20 - F7 manual light editor v19
- Individual selected-source intensity/range sliders; global ambient per profile.
- No predefined emitters; exact old Pub presets migrate out, marked/customized retained.
- Marking no longer restricted to Pub. Manual profile slots 000..999, separate CSVs.
- Battle projected-origin compensation reuses mouse-grid RAM coordinates; town canvas unchanged.
- NO MEDIDO: live placement/camera across every map. Slot identity is manual, not a ROM stage ID.
- No installer, ROM assets, original save or commits changed.

MEDIDO v19: menu 31 passed; screen 57 passed, 1 ignored. Windows release build finished. GPU proof empty/all 359913/922880 changed pixels; moved torch 410089; flicker 192975; window stable. Editor preview target/light-editor-v19.png inspected. Binary target/release/FFTARecomp-LightEditor-v19.exe SHA256 A282B68626403DF8B9AAA2FAA6D1E85B600DF921F4E92A2714C9EAA63BE0E6ED. NO MEDIDO: live placement after this build and universal camera anchoring.

## 2026-10-20 - Light editor v20
- Range controls falloff across its footprint, not just the outer cutoff.
- Per-source Warmth slider, cool blue through golden yellow; nighttime window preset is steady/golden.
- F7 Stage name is editable (Enter saves / Esc cancels), persisted per manual profile.
- Named lighting XLSX regenerated beside profile CSVs, separate from mission register; verified map IDs blank.
- MEDIDO: 32 menu + 59 screen tests pass, 1 ignored. GPU range20=21893 vs range160=639880 pixels; warmth differs, windows steady.
- MEDIDO: independent XLSX ZIP CRC/XML validation, two names retained. Windows release compiled.
- Binary: target/release/FFTARecomp-LightEditor-v20.exe, SHA256 6C289F29D5DA833162EBDE4B2F4AC62FA64789B33C79E10A6272EABFA1F6D1DA.
- NO MEDIDO: live user placement, all-stage camera anchoring and runtime identification of manual names.
- No installer/original save/ROM content/commits changed; no added dependencies.

## 2026-10-20 - F6 and global stage warmth v21
- F6 toggles shader enabled state live, once per keyboard press, no pause/restart; F7/F8 stay synchronized.
- Ambient warmth (global): -100% Cool / Neutral / +100% Warm; independent of source colors and brightness.
- Whole-stage white balance after lighting; separate extracted UI unchanged, neutral exact bypass.
- Temperature persists per manual profile and is appended to lighting XLSX; old profiles default neutral.
- MEDIDO: 33 menu + 60 screen tests passed, 1 ignored; actual GPU neutral identical and shader OFF bypass verified.
- MEDIDO: warm/cool red delta 13668579, blue delta -22027665 without emitters. XLSX CRC/XML and two profiles verified.
- Windows: target/release/FFTARecomp-LightEditor-v21.exe, SHA256 DFFADDCA713E91E11E1E890FEEB065445B5244CA7CD83D65FB5044488722CC90.
- NO MEDIDO: interactive F6 key in live new build. No original saves, installer, ROM assets, new dependencies or commits.

## 2026-10-20 - Dialogue transitions / Monster Bank v22
- General dialogue attachment uses pre-scroll backing and stable metadata through one-plane animation phases, never retained UI pixels.
- Monster Bank/site speech separated from centered menu/cursor, with custom edge/scale/length layout.
- Cinematic UI compositing covers all shader profiles; avoids raw unshaded background reconstruction under translucent panels.
- MEDIDO: before capture target/monster-bank-v21-before.png; 165 core + 33 menu + 60 screen tests pass; 5+1 skipped by default.
- MEDIDO: explicitly run RTX5050 GPU alpha test over a colored background at two zooms.
- NO MEDIDO: live entry/exit timing in new build; user to review. Existing profiles, original saves, installer and commits untouched.
- Windows: target/release/FFTARecomp-Dialogue-v22.exe. See docs/ffta_dialogue_transitions.md.

## 2026-10-20 - Fully manual shader / two-click sun v23
- Removed live scene-color classification, location/interior hints and light-list-forces-Interior behavior.
- F7/F8 Neutral/Day/Night/Fog/Interior chosen and saved explicitly per named manual slot; no automatic emitters.
- Point lights compose over chosen base. Existing source data, names and temperatures retained; older slots default Neutral.
- Solar direction: click source side then travel side; game frozen, no guest input; only normalized direction saved/logged/exported.
- MEDIDO: 34 menu + 63 screen tests pass, 1 ignored. Actual GPU neutral source-identical, no inferred emitters, opposite sun arrows affect Day even with lights.
- MEDIDO: XLSX CRC/XML preserves both names/temperatures/presets and normalized solar vector. Verified stage IDs remain blank.
- NO MEDIDO: live new-game scene or interactive two-click tool in new build; user to review.
- Windows target/release/FFTARecomp-ManualShader-v23.exe. No installer, original saves, ROM content, new dependencies or commits.
# Shader v24: cobertura completa del mundo

Se elimina la exclusion de azul/cian y las mascaras automaticas del camino manual.
Ambiental, sol y luces marcadas iluminan la imagen completa del juego; UI separada.
MEDIDO: 34 tests menu, 63 tests screen; prueba GPU de ocho colores con ambiental
0 y 0.5, radio de luces y direccion solar. NO MEDIDO: revision jugando con v24.
Perfiles del usuario y saves originales sin modificar. Sin cambios de instalador.

# Sombras por contornos v25

Editor F7: contorno rojo relleno para objeto, azul para huecos; mouse arrastrado
o puntos, click derecho/Enter cierra, Backspace deshace, Esc cancela. Dos clicks
definen direccion/alcance. Opacidad individual, redibujar objeto y borrar huecos.
Juego pausado al dibujar; mascaras 2D propias, sin inferir geometria de ROM.
Sol disponible en Neutral; eliminado relieve automatico basado en contraste.
MEDIDO: 101 tests menu/screen; GPU verifica proyeccion/hueco/exterior/opacidad,
sol en Neutral y regresiones de luces/cobertura completa. NO MEDIDO: uso del
editor con mouse en una batalla real. Sin cambios de instalador ni saves.

# Editor F5 y bloqueo de navegacion v26

MEDIDO: filas especiales de sol/sombras consumian Arriba/Abajo/Cancelar antes
de la navegacion comun. Corregido; 36 tests menu pasan, incluido recorrido de
todas las filas, rueda y Cancelar. F5 abre luces/sombras; F7 ajustes generales.
NO MEDIDO: navegacion en ventana real. Sin cambios de shader, perfiles ni saves.

# Conjuntos de sombras con Ctrl v27

Cada Ctrl cierra/aplica una zona roja y permite dibujar otra del mismo conjunto.
Enter termina y pide dos clicks para la direccion compartida. Hasta 32 zonas por
conjunto, intensidad comun; azul recorta huecos. Anadir zona permite retomar uno.
MEDIDO: 103 tests y GPU de dos zonas, direccion/intensidad comunes, hueco intacto
y espacio entre zonas sin sombra. NO MEDIDO: interaccion Ctrl en batalla real.
Perfiles antiguos de un contorno siguen siendo compatibles. Sin cambios de ROM.

# Longitud de sombras y nombres v28

Range estaba desactivado para sombras. Ahora Shadow length cambia la longitud
del vector compartido conservando direccion/opacidad. Proyeccion desde la base
de cada area evita colapsos al apuntar abajo. F5: Name permite nombrar sombra,
antorcha o ventana sin modificar IDs; etiquetas persistidas en perfil y Excel.
MEDIDO: tests de persistencia, IDs, longitud y proyeccion. Prueba GPU de alcance,
dos areas y huecos. NO MEDIDO: manejo en ventana real. Sin tocar saves/instalador.

# Anclaje al terreno v29

Origen de luces/sombras sigue el scroll real del par BG del terreno, conserva
anclaje si el fondo se oculta y sigue midiendo con F6 apagado. Maneja wrap 9 bits.
Cambio de perfil reinicia referencia con la convencion de coordenadas anterior.
MEDIDO: 108 tests menu/screen; casos de camara, wrap y sombra/terreno sincronizados.
NO MEDIDO: movimiento en batalla real. Perfiles y saves originales sin modificar.

# Seguimiento visual y sombra pegada v30

Contornos manuales ensenan referencias RGB locales; busqueda conjunta limitada,
rechazo de patrones ambiguos y exclusion de OBJ/UI conocidos. Relearn en F5.
Si se pierde referencia se conserva correccion fiable y usa prediccion de camara.
Extension conserva la huella roja original y sus huecos; maximo 96 px nativos.
Shadow follows sun por conjunto; largo/opacidad independientes. Sin ROM en repo.
MEDIDO: 111 tests, GPU de extension/huecos, benchmark release sintetico de tracking
8 conjuntos 1536x800: 0.7727 ms/frame. NO MEDIDO: FPS/coste total en juego real.

# Tutorial y sombras simples v31

MEDIDO en 4 volcados locales: callback08025115/mision0/evento6/localizacion1.
Perfil guardado Tutorial (slot005 existente) se selecciona al entrar; efecto
apagado fuera de esa identidad y en pausa. Sombras manuales/tracking apagados
por defecto, conservando datos. Refuerzo local suave de sombras, max8%, sin largo.
MEDIDO: 113 tests y GPU de refuerzo suave, superficies claras intactas y OFF exacto.
NO MEDIDO: recorrido real del tutorial/camara. Sin cambios de saves/instalador.

# Perfil Tutorial Winter sunlight v32

Perfil propio separado, primer slot libre, sin sobrescribir Tutorial del usuario.
Sol fijo viaja de arbustos derechos hacia izquierda/fondo, vector pantalla(-2,-1).
Ambiental0.88, temperatura0.02, elevacion35grados, niebla1 y sombras simples suaves.
MEDIDO: tests de instalacion sin sobreescritura; GPU RTX5050 462080 pixels cambiados,
delta medio16.34/255. Preview revisada visualmente. NO MEDIDO: revision jugando.

# Activacion de perfiles fuera del tutorial v33

MEDIDO en instancia6424 v32: Shop seleccionado, evento162/localizacion60, BG
4C03/4E02 y DISPCNT3F40. El filtro anterior apagaba TODOS los perfiles fuera
del tutorial. Se separan mundo activo y seleccion automatica de Winter sunlight.
Resto de perfiles vuelven a funcionar sobre terreno; titulo/pausa quedan fuera.
Winter solo tutorial y al salir restaura perfil anterior si no hubo cambio manual.
115 tests pasan y Windows compilado. NO MEDIDO: prueba real con v33.



2026-10-20 v34: cinco sombras suaves estaticas para nieve del tutorial, en
coordenadas del plano y scroll real, sin alterar personajes ni mapas del juego.
MEDIDO GPU: cinco grupos, repeticion identica, pan 20x10 nativo correcto,
F6/bypass exacto; 4773 pixeles cambiados, descenso maximo 30/255.
Pub/Shop: seleccion automatica de perfiles guardados, conservando ediciones.
MEDIDO previo Shop: evento162/localizacion60; Pub usa lector de menu verificado.
Prision: perfil portable incluido; identificador exclusivo y activacion automatica
NO MEDIDO, pendiente de dejar la escena abierta. No se usa el grupo generico.
Capturas locales: target/snow-v34-gpu/before-after.png y snow-after.png.


2026-10-20 v35: correccion del informe v34 tras feedback del usuario.
Se selecciona el perfil guardado Tutorial por nombre, NO Winter sunlight.
En este equipo hay dos Tutorial (base y slot005): se conserva el seleccionado
si ya coincide; de lo contrario se usa el primer slot del nombre (base=0).
No se modifican archivos guardados. Pub y Shop mantienen sus nombres guardados;
Prision sigue sin identificador de escena exclusivo medido: NO asignado por error
a otros servicios. Su perfil existente se conserva.
Causa medida: activacion anterior exigia event6, y el inicio observado usa event1.
Ahora se compara huella del terreno16x16, estable en cuatro volcados con camaras
distintas: 6e763f2e390af953. Se conservan callback EUR, mision0 y localizacion1.
La mascara receptora de sombras usa nieve original, no el color oscurecido por
ambient. GPU con perfiles reales: ambient1.02 cambia3859 pixeles; ambient0.41
cambia5208 (mascaras proyectadas antiguas desactivadas, como en produccion). Hashes de los CSV originales identicos antes/despues.
NO MEDIDO: validacion jugando de toda la introduccion y transiciones reales.
Capturas: target/saved-snow-v35-gpu/base-snow-after.png y 005-snow-after.png.


## 2026-10-20: IDs EUR medidos por el usuario
Pub=62, Prision=63, Shop=65, Monster Bank=66, leidos por FFTA_STAGE.
Se asignan a los nombres guardados pub, Prision, Shop y Monster Bank.
Sustituye las restricciones anteriores que dejaban estos interiores sin asignar.
El mapa no hereda efectos; una vinculacion manual persistente de F5 tiene prioridad.
Esto arregla la seleccion automatica, NO termina la reconstruccion del plano
completo con iluminacion pintada antes de transformar la camara. El desfase de
sombras durante cinematica sigue pendiente; no se declara resuelto.

## 2026-10-20: fixed terrain-plane shadows and assisted contour
The PPU now emits a separate source-coordinate buffer for terrain pixels.
Coordinates use the same calculated RAM map origins and live HOFS/VOFS as
render_host_tile_row, on each scanline. UI, OBJ and backdrop emit no terrain
coordinate. Serial and parallel zoom preserve this sidecar independently of RGB.
The complete 512x512 terrain atlas is reconstructed from the two verified RAM
maps, original tiles and palette, without UI, objects or a background color.
It is only a local runtime image, never included as game artwork in the repo.

New red contours and blue holes are saved as absolute terrain-plane pixels,
identified by # shadow_stage=shadow_ID in their existing profile CSV.
The fixed shadow atlas is rasterized only when geometry/opacity/direction changes.
Camera movement and zoom sample its existing pixels using the PPU sidecar;
they do not move, match, re-extrude or repaint contours. Old contours remain
stored in their legacy coordinate space, disabled by default. Redraw an old red
contour before adding new plane-space holes or areas to that group.
Guessed procedural tutorial snow ellipses are disabled in the running game.
The user will author mound footprints; their silhouettes have not been measured.

F5: the existing red/blue shadow editor remains in use. Assistance is ON by
default. Tab toggles assistance while drawing; FFTA_SHADOW_ASSIST=0 starts manual.
Assistance only consults the reconstructed terrain atlas: locally smoothed stroke
tangents guide a nearby luminance/alpha edge search along each contour normal.
Each point moves at most 3 native pixels; weak/ambiguous edges keep the original
point. Assistance applies on Ctrl/Enter commit. It is not semantic object
segmentation and does not promise a correct silhouette on every texture.
New plane shadows do not require FFTA_MANUAL_SHADOWS=1. That flag only enables
legacy screen-space shadows, and should remain 0 for this test.

Tests: fixed atlas projection, bounded assisted edge fitting, flat-image fallback,
stage-space contour/hole persistence, source-coordinate invariance across camera
and map-crop changes, terrain-only reconstruction, serial/parallel zoom parity,
and extra=0 original pixel/OAM equality. Real cinematic alignment remains subject
to user gameplay feedback. Existing point-light positions have NOT been migrated
to this new plane-paint representation in this patch.


## 2026-10-21: dedicated F4 shadow editor
F4 opens/closes SHADOWS, independently of F5 lights and F7 shader settings.
The shadow page only selects shadow groups; New shadow group always clears
the previous selection. Ctrl commits another red area into the current group;
Enter commits the current contour and returns directly to F4. Drawing a direction
is now optional through its own menu action. Left clicks on contour/name actions
now generate Confirm instead of Right; this was why the creation button appeared
to do nothing. Old polygon entries have been removed from the lights page.

32 independent light sources and 32 independent shadow groups are supported per
profile. Shadow records do not consume GPU point-light slots or the light limit.
Existing CSV syntax is preserved. Group names, opacity, extension, holes and
redraw actions remain individual. Legacy optical surface relearn is excluded from
the new fixed-plane shadow menu.

Assistance now combines nearby RGB color gradients and luminance gradients along
the drawn contour normal. This also follows borders with similar luminance but
different colors. The maximum movement remains 3 native pixels; weak edges retain
the original point, and Tab still switches to manual drawing. This is a local
edge aid, not a semantic segmenter; user gameplay feedback remains necessary.

2026-10-21: activacion solar enviada explicitamente en uniform independiente del array de luces; sombras nuevas y areas del mismo grupo probadas con acumulacion de opacidad entre grupos. F4: Add painted area amplia el grupo; New shadow group permite oscurecer encima sin modificar el anterior. Regresion visual de luz pendiente de feedback del usuario; no declarada resuelta solo por tests.
2026-10-21: correccion de compatibilidad de luces guardadas: track_engine_camera ya no sobrescribe point_rig.origin; las sombras nuevas usan su atlas y las luces antiguas conservan CameraAnchor. F6 alterna solo el master de sesion sin publicar el perfil CSV. FFTA_LIGHT_STATE registra cambios de master/gameplay/scene/sun/profile, numero de emisores y cobertura. Perfil pub-stage-001 observado con siete emisores y sun_enabled=0, respetado. Resultado visual de luces y rayos pendiente de prueba del usuario.
2026-10-21: usuario pide reiniciar luces: retirados 9 emisores puntuales de CSV locales, con copia before-light-reset-20261021-010944; sombras y ajustes conservados. Descriptor de stage ya no depende del callback de script durante cinematicas: exige EUR, modo y par exacto de BG del terreno e ID valido. F7 Shader on characters persistente, excluye pixeles OBJ del shader usando mascara PPU y coordenadas de terreno; no altera las sombras pintadas. Tests callback y switch pasados; resultado de rayos en gameplay pendiente.
2026-10-21: Pub antorchas si estaban guardadas: ocho emisores en pub-stage-001.csv. Log de marcado origin=[-248,-48], reentrada origin=[-8,184], acumulacion CameraAnchor equivocada. Se reemplaza transformacion de emisores por negativo del origen absoluto RAM de tiles mas HOFS/VOFS, igual al muestreo del plano PPU; sin dependencia de historia de camara. Test con marked_0=(546,145) verifica (298,97), reentrada, cambio de crop y zoom; 40 menu y 91 screen pasados. CSV no modificado por este parche. Validacion visual de reentrada pendiente del usuario.
2026-10-21 validacion VISUAL real Pub: save EUR copiado, juego PID15940, entrada, salida al mapa y reentrada completadas por teclado. Fallo reproducido: fuentes CSV sin desplazamiento de origen del plano RAM7f68/6a=(-24,136). Migracion unica de ocho luces por (-24,+136), backup pub-stage-001-before-plane-migration.csv, metadata light_space=terrain-v1; no borradas/recolocadas. Capturas target/pub-visual-migrated-on.png y pub-visual-second-entry.png alineadas. A/B emisores intensidad cero/restaurada con mismo ambiente: target/pub-emitters-zero.png y pub-emitters-restored.png. ROI(630,170,715,230) ganancia luma17.851/255, ROI(725,225,800,285)13.309, ROI ventana(500,180,550,225)10.142. Diferencia absoluta luma entre dos entradas ROI(630,170,800,285)=3.041, con animacion/flicker activo; no se afirma igualdad exacta de frames. CSV restaurado tras prueba. Save original SHA256 igual a copia inicial; juego usa save debug aislado. Windows PubMeasured compilado34.93s, tests40menu91screen. Estos resultados visuales son con el binario PubReentry y CSV migrado; PubMeasured agrega metadata para los nuevos emisores.
2026-10-21: rayos solares separados de iluminacion local. Pasada atmosferica sobre cielo y terreno, antes de UI separada; no depende de cobertura del stage. GPU real mascara terreno=0: solar0 cambia0/819200, solar1 cambia273312/819200. Stats/prevision detectan las cuatro piezas de backing con coordenadas signed durante entrada/salida parcial; layout usa bounds completos, no pixels visibles recortados. Panel de acciones/atacante ya no exige scroll BG2=0; captura texto/backing en espacio UI nativo independiente del scroll de entrada. Tests nuevos cubren ambos formatos y offsets X/Y. Sin cambios de buffers guest; animacion visual en gameplay pendiente de feedback.
Correccion adicional medida por test de transicion: begin_dialogue_frame exigia edge[0]+edge[1]>0 para acciones. Ahora acepta backing presente sin tocar borde, o Stats activo durante la prevision. Evita salida temprana durante animacion. Tests completos repetidos antes de entrega.

2026-10-21 Menus de servicios EUR: cartel MENU en canvas izquierdo y resto del UI en canvas derecho para stages 62 Pub, 63 Prison, 65 Shop y 66 Monster Bank. Captura BG2/BG3 en coordenadas nativas durante entrada/salida; no depende del cursor habilitado para reconocer la geometria del root. Inventario y LIST no heredan un root antiguo en RAM; mouse traducido por su canvas. Sin cambios de ROM/RAM ni del shader. MEDIDO: pruebas core 173 aprobadas, 5 ignoradas; recomp 28 aprobadas, 1 ignorada, ejecutadas en serie por estado global del PPU. La regresion recorre los cuatro IDs y scroll 0/8/64/248/0, conserva la captura y desactiva el nuevo layout con extra=0. Windows release compilado. NO MEDIDO: validacion visual jugando de las cuatro animaciones, pendiente del usuario.

2026-10-21 UI scene EUR: modelo comun de paneles ffta_ui (roles, geometria nativa y anclajes), consumido por render/mouse/inspeccion. Bounds completos antes del recorte animado para dialogos; menus Actions/Stats usan geometria del propietario. Escala uniforme con ajuste al viewport incluyendo Length, sin estirar texto/retratos ni descartar foreground fuera del canvas. BG2/BG3 compartidos por scanline, pasadas OBJ de backing vacias evitadas, composicion ajena al panel descartada. PackedLayer reutiliza bytes/bounds por igualdad exacta; invalida al cambiar contenido, estilo o presencia. GPU evita copia/upload de world raw con mezcla cinematica y paneles opacos; conserva mezcla GBA con shader OFF. FFTA_UI_TIMING opcional separa CPU emu+PPU, preparacion y presentacion con espera (no timestamp GPU). FFTA_UI_REFS opcional exporta referencias BG/tiles/OAM y hashes para futuros mods; NO decodifica texto ni identifica aun punteros ROM de strings.
MEDIDO: release core 181 passed/7 ignored, recomp 29 passed/1 ignored, screen 91 passed/1 ignored; GPU explicita RTX5050 1 passed. EXTRA0 frames=600 framebuffer_and_oam_equal=true framebuffer=a04a7922125b2159 oam=971d444336ad5341. JSON de referencias validado con parser independiente. Benchmark sintetico panel estatico Length150%: sin cache 0.3886ms, cache 0.1223ms (solo empaquetado; no FPS). Los tiempos sinteticos del render completo variaron y no prueban mejora global. NO MEDIDO: validacion visual de todos los paneles/idiomas y FPS de gameplay real; pendientes. Docs: docs/ffta_ui_scene.md. Windows target/release/recomp.exe compilado. Sin cambios de instalador, dependencias nuevas, commits ni saves originales.

2026-10-21 Revision visual del usuario, pendientes antes de mods: PUB y Prision dejan una franja azul separada del root MENU. Usuario observa en Prision y tienda que MENU entra centrado y salta al anclaje izquierdo; animacion aun no medida por el agente. Captura actual target/missions-ui-offset-missing-arrow.png (852x480): lista de misiones desplazada a la derecha; debe quedar centrada en la pantalla completa. Clan Funds aparece elevado; debe permanecer abajo a la derecha como elemento independiente. Usuario identifica ausencia de la flecha azul de mas contenido y hueco en el borde inferior derecho de LIST; captura conserva ese hueco. No se aplicaron cambios: usuario continua revision.

2026-10-21 Revision visual adicional, pendientes: Card Keeper conserva MENU en el centro; aplicar la politica izquierda de los otros servicios. Clan Funds abajo a la derecha con margen fijo. Monster Bank: MENU entra centrado y salta a la izquierda. Engage with y Victoria deben centrarse en la pantalla completa. Al entrar/salir de Action aparece brevemente una barra azul en el lado derecho; usuario lo observa durante milisegundos, aun sin medicion temporal del agente. Algunos ataques, ejemplo Hurl, dividen el cartel entre dos posiciones. Captura target/action-portrait-bottom-overflow.png (852x480): batalla con banda jugable entre Y120 y Y360; retrato y datos inferiores de Milton llegan al borde Y360 y quedan recortados por la banda negra. Incluir retrato completo y texto en los limites logicos del panel y en su ajuste vertical; no basta el backing azul. No se aplicaron cambios; usuario sigue revisando.

2026-10-21 Abilities, pendiente registrado durante revision del usuario: target/abilities-arrows-missing-square.png (852x480). Flechas de scroll separadas del panel: una abajo en el escenario, otra cerca del encabezado MP. Usuario identifica un cuadrado faltante en el panel y ausencia de soporte del mouse. Captura confirma la flecha inferior flotante y conserva el defecto completo; la interaccion con mouse es MEDIDO por el usuario, no probada por el agente. No se aplicaron cambios.

2026-10-21 Tanda de bugs EUR implementada, pendiente de revision visual: MENU por borde izquierdo propio, listas centradas por propietario, Clan Funds independiente abajo derecha con margen, modales centrados, retratos incluidos en bounds, stats normalizados durante desplazamiento y mezcla, cierre parcial sin reciclar pixeles, flechas UI asociadas y Abilities paginado con pitch medido. Capsulas de ataques: rotacion unica del mapa256 y top-center sin Length; corregido descarte nativo de columnas240..255. Efectos text/affine finitos fuera de viewport y descriptores ffta_effects exportables (no ID de hechizo; OBJ-only pendiente). MEDIDO: core189/recomp32/screen91 y GPU RTX5050 1 aprobadas. EXTRA0 frames600 FB=a04a7922125b2159 OAM=971d444336ad5341 identicos. Windows release compilado. NO MEDIDO: todos los casos reales, especialmente Clan Funds/flechas/transiciones; checklist docs/ffta_ui_review.md. No nuevas capturas de ataques. Sin commits, instalador ni saves originales.

2026-10-21 Tienda: captura target/shop-buy-sell-funds-stats-bug.png confirma atributos numericos fuera del panel y contenido desplazado abajo a la derecha. Parche: el inventario tiene geometria completa 240x160, incluye sprites 4bpp de cifras en su canvas y se reconoce por propietario aunque todavia no tenga lista seleccionable. Clan Funds se captura desde sus OBJ en canvas propio; ya no recorta un rectangulo del contenido compuesto. Detector excluye retratos 8bpp y separa grupos desconectados. Desactivada repeticion lateral de backing en servicios, tambien con split_speech. Lector visual de root tolera desplazamiento vertical de entrada con flags de root. MEDIDO: core191/recomp32/screen91 aprobadas, mas regresion de composicion 1 aprobada (fondo permanece bajo contador, atributos en capa principal, OAM intacta). Windows target/release/FFTARecomp-ShopFix.exe compilado sin cerrar instancia anterior. NO MEDIDO: resultado visual real de Buy/Sell y animaciones con este ejecutable. Los restantes casos de la tanda siguen sujetos a revision.

2026-10-21 Prioridad del usuario: aislar causa de entrada/contenido/salida del PUB antes de extenderla. MEDIDO sin screenshots: objeto visual EWRAM con mapas 06005800/06005000 y estilo0180 existe y dibuja antes de crear callbackcursor08018fc5; root retorna con cursor ausente durante ~6 frames. Al cerrar flags de cursor0F->0D mientras ancho animado11->0. Rectangulo propietario canonico empiezaY72, noY76 deducido del cursor. Nuevo ffta_ui_window reconoce etapas por ancho/alto actual y objetivo, conserva [0,72,96,160] y anclaje izquierdo/inferior, sin retener pixeles ni escribir RAM. Scope EUR stage62 rootPUB. Mouse asocia origen de lista por pertenencia al rectangulo, no offset16 fijo. Replay de dos secuencias reales: 302 muestras visibles (10 entrada,282 abierto,10 salida),12 sin cursor; bounds/anchor estables y EWRAM intacta. Core196 y host32 pasan. Compilado target/release/FFTARecomp-PubFix.exe. NO MEDIDO: aceptacion visual del usuario; otros menus y cambios de batalla siguen pendientes de revision. Detalle docs/ffta_ui_pub_lifecycle.md. Instancia original permanece abierta en PUB, sin reemplazar su exe ni cerrar partida.

2026-10-21 Usuario confirma root MENU del PUB perfecto. Extendido lector de ventana original a dialogos y contenido del PUB: estilos0030/00C0 con recursos/tipos comprobados, tres propietarios independientes por frame antes del cursor y despues de inhabilitar controles. Dialogo ahora usa capa3 propia con margen fijo para retrato, separado del rectangulo exacto que posee su BG; listas usan geometria completa y centro estable. Root confirmado conserva anclaje y cabecera. MEDIDO: dos ciclos por memoria (Missions/Rumors),96 estados de cambios de geometria,112 instancias visibles con entrada/abierto/salida, bounds y anchors estables, RAM intacta; core197/host32/screen91 pasan. Windows target/release/FFTARecomp-PubWindows.exe compilado. NO MEDIDO: aceptacion visual del usuario del nuevo dialogo/submenus. Otros stages requieren perfilar su ventana antes de extender el scope; no se declara fix global aun. Sin screenshots, commits, instalador ni escrituras en saves originales.

2026-10-21 PUB residual mouse: native absolute-index query18CAC was being overridden during previous-row restoration, causing stale highlight palettes and inconsistent stepping. Pub content now bypasses it and injects destination at original selection/repaint helper6196C; root remains on its existing path. Added reusable right-side pixel-art scrollbar, dark-blue track/cyan proportional thumb, only when original total>visible; drag/track clicks consumed, wheel supported. Native pagination retained to populate circular glyph buffer correctly. Isolated real-snapshot probe: five direct jumps each have exactly one selected row; original scrollfirst1/2 and post-scroll direct selection also pass. Core197/host32/screen93 pass; Windows target/release/FFTARecomp-PubScroll.exe compiled. Pending user visual acceptance. Scope Pub verified list producer. No screenshots, original save writes, installer changes or commits.

## Pendiente para la próxima sesión: editor de nombre descompuesto (2026-10-22)

El usuario pidió capturar y recordar, sin aplicar ningún parche hoy.
Captura persistente: `docs/bugs/2026-10-22-name-entry-widescreen.png`.
Ejecutable abierto: `target/release/FFTARecomp-DialogueMouseFix.exe`; cliente 1704x480 (32:9), ROM EUR, nueva partida, escena de nieve al introducir el nombre del protagonista.

Observado: el teclado de letras está pegado al borde izquierdo y su panel se corta en el lado derecho; la pestaña Aa queda separada arriba, cerca del centro; A:Enter/B:Del y START:OK aparecen separados del teclado; una franja vertical con letras queda flotando a la derecha; el retrato y la caja del nombre aparecen por separado en el centro inferior. La interfaz original de introducción de nombre se está clasificando/transformando como varios paneles de diálogo o servicio independientes. Esta es una hipótesis para revisar, no una causa confirmada. Prioridad: identificar el propietario del editor completo y mantener teclado, pestañas, instrucciones, confirmación, retrato y campo de nombre en una misma geometría coherente; evitar que las reglas de burbujas separen sus piezas. Revisar soporte de mouse/teclado después de recomponer la presentación.

No cambiar los parches del Pub/Rumors/Clan Funds que el usuario confirmó ni la ocultación de ambos fotogramas de la pluma del mapa. El último parche de diálogo corrige los estados EUR 6/7 y prioriza el avance del texto sobre menús de fondo; no hay confirmación del usuario todavía sobre este último. Al retomar, revisar esta captura antes de editar y compilar Windows al entregar un fix.

## Store: ciclo de ventanas, selección directa y scroll (2026-10-22)

Se amplió el protocolo de ventanas animadas ya verificado del Pub al stage65 (Store), manteniendo la identificación por recursos/estilo y dimensiones medidas. El root MENU usa la misma geometría antes de existir el cursor y durante su salida. El inventario [1,0,26,11] conserva un canvas completo 240x160 para categorías, artículos, atributos inferiores e indicadores; no se recorta al alto de cuatro artículos. Speech conserva su propietario independiente. Pub queda con su política existente.

La barra reutiliza la geometría de ui_scrollbar, en la derecha de la lista nativa [8,0,216,88], visible solo con desbordamiento. El modelo real publica total/first y capacidad de4 filas, tanto Buy como Sell. Arrastrar y rueda solicitan una página absoluta; el callback original EUR08071030 presta su marco para llamar al redibujado original08070624 y restaurar todos los registros/CPSR antes de reanudar. No se emiten teclas ni sonidos de cambio de fila. La selección de artículos usa el mismo redibujado con el índice objetivo, sin visitar filas intermedias. El scroll presenta filas sin resaltado; B oculta la barra inmediatamente. Categorías siguen los IDs reales disponibles de Buy/Sell, sin inventar inventario en categorías vacías. Mouse continúa sujeto al switch Keyboard and Mouse y a píxeles/límites del panel visible.

Validación: snapshots privados de entrada/salida root y Buy/Sell; rutina original probada con listas de7 y22 artículos y saltos a primera/última página; hover3,0,2 sin direcciones simuladas y con publicación comprobada del índice final. Imágenes extraídas conservan categorías, precios y atributos. 204 pruebas core + prueba nueva de geometría de inventario durante fases +3 guardas host aprobadas. Pendiente aceptación visual del usuario. No se modificaron saves, Android ni Linux. El editor de nombres widescreen sigue pendiente por separado.

## Store: seguimiento de mouse corregido (2026-10-22)

Feedback del usuario: categorías deben cambiar solo al hacer clic; root Buy/Sell/Trade Items/Leave no salta directamente; primeras filas parpadean al pasar por otros artículos.

Se consume el hover de categorías sin cambiar su selección ni cancelar cambios pendientes; solo confirm=true llama al selector original. Las pruebas existentes ahora verifican explícitamente hover sin efecto y clic con cambio. Store root admite3/4 filas y se excluye del identificador de Pub cuando stage65. Su selección publica inmediatamente cursor+17, desplazamiento y solo paletas B/C del shadow tilemap, conservando texto e iconos, sin input direccional.

El parpadeo venía del uso de70624 completo incluso al cambiar solo la selección: primero limpia/reconstruye los tiles de la página. Para hover dentro de la misma página, tras actualizar los metadatos originales se continúa desde706DC, conservando las filas y ejecutando los actualizadores originales de cursor, paletas, detalles y paneles inferiores. El scroll real sigue usando la reconstrucción completa. La ruta rápida se limita a la llamada prestada y se valida por firma de ROM; restaura los registros originales al regresar al callback.

Prueba con grabaciones reales: root salta2,0,3,1 con índice inmediato y sin teclas; Buy/Sell pasan hover3,0,2 y límites de scroll; la traza del hover verifica que no ejecuta borrado ni construcción de glyphs (7067A/70692/706C8). Queda revisión visual del usuario para confirmar el parpadeo.

## Store: números de atributos sin animación de salida (2026-10-22)

El usuario solicitó ocultar instantáneamente los números blancos con contorno grueso al cerrar Buy/Sell, conservando el resto de la animación. Referencia privada: target/store-number-exit-reference.png. Atlas verificado de los atributos: pares de OBJ8x8, modo normal/paleta0, tiles02C0..02D3. Precios, iconos, cursores y Clan Funds quedan excluidos.

El estado de la ventana de inventario stage65 gobierna shop_numbers_exiting: Exiting/Hidden ocultan esos sprites en captura y pasada wide del host; Entering/Open los muestran. Se mantiene el estado mientras se retira el propietario y se limpia al abandonar el Store, evitando que aparezcan los últimos fragmentos al terminar el cierre. No se modifica OAM ni lógica de compra/venta.

Prueba desde RAM grabada: cerrar la ventana cambia430 píxeles, todos dentro de16 piezas del atlas de dígitos; el resto de la capa es idéntico y OAM queda intacto. Filtro unitario y cargo check aprobados. Pendiente comprobación visual del usuario en movimiento.

## Store: scroll sin páginas parciales (2026-10-22)

Feedback: el parpadeo de las filas1/2 desapareció al hacer hover, pero seguía al tocar/arrastrar la barra. La ruta de scroll todavía necesitaba reconstruir glyphs y podía atravesar scanlines/fotogramas con el buffer borrado. Ahora tocar la posición actual no solicita redibujado. Para cambios reales, la capa UI conserva la última página completa mientras el callback original reconstruye la siguiente; cualquier frame afectado por esa transacción se descarta visualmente y solo se publica una página capturada completa después de finalizar. El mundo y el tiempo emulado siguen avanzando. El caché se limpia al salir del inventario para no restaurar los números ocultos durante la salida.

Prueba con la ROM y RAM grabadas de Buy/Sell: se fuerza un render completo precisamente en08070696, inmediatamente después del borrado original. Los píxeles presentados coinciden con la página anterior; al terminar se publica la página destino correcta. Se mantienen los saltos de hover, categorías solo con clic y la desaparición de números al salir. Test unitario cubre buffers vacíos/parciales/completos aún en transacción y limpieza al cerrar. Aceptación visual del usuario pendiente.

## Prison: animaciones y preguntas con mouse (2026-10-22)

Stage63 añadido al protocolo de propietarios de ventanas ya usado en Pub/Store. Snapshot read-only del proceso abierto: root owner243F4, rect[0,9,8,9], estilo0180/0C20/00010005; speech23C7C estilo0030/0B20/00040015. El root tiene bounds canónicos[0,72,72,144] durante entrada/contenido/salida, y la burbuja conserva su propia capa. Root Release/Pardon/Leave acepta selección directa como Store. No se altera el save ni se fuerza ningún preso.

La bienvenida observada está en estado8: espera una opción del menú y no debe convertirse en A indiscriminada. Las preguntas de dos respuestas usan el estado9. Nuevo descriptor DialogueChoice valida las rutinas11B66/13214 y obtiene X/Y/espaciado de los mismos campos que usa el cursor original. Hover selecciona la respuesta directamente; clic confirma solo dentro del área de respuestas, usando el registro local en11B6C (sin KEYINPUT o RAM compartida de teclas). Tiene prioridad sobre el menú de fondo y no confirma al hacer clic fuera de las respuestas. Los estados de texto6/7 conservan su avance específico.

Pruebas: RAM real de bienvenida en copia privada, cinco posiciones de animación con geometría idéntica; root2,0,1 sin input direccional; estado9 preparado en esa copia y ambas respuestas recorren la rama de confirmación original preservando shared keys. 208 pruebas core +3 guardas host pasaron. Falta aceptación visual del usuario. Submenús con presos aplazados explícitamente por el usuario; no afirmar que fueron probados. El usuario conoce que L en mapa muestra leyes actuales; no hay ley universal, y generar preso se probará después.

## Save overwrite NOTICE y Card Keeper (2026-10-22)

El Yes/No de Write over old save data utiliza la misma pregunta state9, pero en scene17 y planos frontend nativos: no existe node Dialogue separado. El host ahora convierte el puntero a coordenadas nativas cuando no hay ese canvas; mantiene la ruta de capas separadas para diálogos normales. Ambos botones se validaron con RAM real de ese aviso y la rutina original, deteniendo antes del script de confirmación para no escribir ningún save. Clic fuera no confirma un menú inferior. El hash de ffta.sav permanece intacto.

Card Keeper medido como stage67, root23C54 [0,9,8,9], speech24268 [0,1,30,8]. Hereda propietarios estables en entrada/salida y separación de voz/root/Clan Funds. Trade reutiliza owner como Content [2,2,15,17]; la capa incluye también el panel derecho de Rate/Rank, mientras la barra se limita al rectángulo de la lista. La lista real tiene16 elementos/7 visibles y callback18FC5, con actualizador distinto de Pub. Selección directa en54380 publica índice/offset de cursor y deja al original restaurar todas las paletas y actualizar modelo/detalles. Scroll redirige dentro del marco original a54694 con first solicitado; omite únicamente su sonido de navegación. La última página completa permanece durante la reconstrucción para evitar parpadeos. Los hooks validan stage, propietario y firmas; no hay texto dependiente del idioma.

Prueba con ROM/RAM reales: root2,0,1 directo; hover5,1,6,0 sin visitar intermedios; páginas5,9,1,0 por actualizador original. Validación visual final del usuario pendiente, especialmente conversación Gossip y confirmaciones de intercambio no ejecutadas para no modificar su partida. No se alteraron Android/Linux/instaladores.

## Monster Bank: propietario estable y selección directa (2026-10-22)

Se incorpora stage66 al lector común de ventanas. Constructor original EUR5AEE4, rama state2, medido en copia privada de RAM de servicios: ventana en manager+CF4, rect[0,9,6,7], estilo0180/0C20/type00010005, cursor dentro de padre en+58. Entrada y salida ahora mantienen los bounds canónicos [0,72,56,128] y canvas izquierdo aun sin cursor activo. Speech/Content con firmas compartidas heredan sus capas independientes, sin reconocer textos traducidos.

La publicación directa normaliza solo el root verificado (handler5AEE5/updater5E7B5/stage66/count2/origin0,92) hacia su cursor/lista hijo; actualiza índice, desplazamiento y paletas B/C conservando glyphs. Se evita recorrer opciones mediante teclas. Se mantiene el clic dentro de la caja y el avance local de diálogo/Yes-No ya compartido.

Prueba bank_constructor_probe ejecuta el constructor/activador original en RAM copiada, completando sintéticamente solo la espera previa de bienvenida. Selección1,0,1 inmediata con keys3FF, cinco posiciones de entrada/contenido/salida estables. No es una reproducción completa de visita real ni prueba de lista poblada: queda aceptación visual del usuario; solicitada ventana real si está disponible. No se modifica el save. Pendiente siguiente tanda: errores visuales de batalla.

## Monster Bank poblado: corrección tras feedback (2026-10-22)

El usuario confirmó que sí tiene monstruos guardados y reportó escenario cortado, ausencia de scroll y previews rotas. Se capturó en lectura el proceso4396: stage66, parent23C4C handler5B625/state24, Content23C58 rect[2,2,14,17], preview24B24 [18,8,6,8], cursor23CA4, total20/visible7. Captura privada target/bank-populated-before.png. El parche anterior solo había medido el constructor del root; esa prueba fue insuficiente para la lista poblada.

Content66 ahora conserva lista+panel de preview en canvas nativo completo; la scrollbar se limita al marco real de la lista. Las piezas4bpp/priority0 del preview (tiles2C0..33F, incluidos sus fragmentos afines) se capturan juntas y se excluyen de la pasada del mundo. Se elimina el fallback genérico de Clan Funds en66: confundía fragmentos del monstruo con un contador y los movía a una esquina. El root conserva el parche de transiciones. Se desactiva la máscara nativa WIN del servicio solamente en la composición wide del mundo; captura de UI y framebuffer nativo mantienen sus reglas originales.

La lista real66 participa en el descriptor proporcional compartido. Selección directa publica índice/offset y paletas B/C en el buffer de su propia lista; el updater original actualiza modelo/preview. Scroll reutiliza el bloque original5C0BC..5C1E4 dentro del marco existente de5B624; el first vive en manager+1BB5 y se publica originalmente en list+1A. Página completa retenida durante su reconstrucción, y solo se omite su llamada de sonido de navegación. El intérprete mantiene la transacción aunque cambie first antes de acabar.

Prueba bank_populated_probe usa ROM y RAM reales capturadas: total20/visible7, targets0,6,2,5 directos/modelo confirmado; páginas7,13,3,0. La prueba aislada drena la cola de DMA como lo haría VBlank para no reencolar la misma estructura sin IRQ (la primera prueba se bloqueaba por esto, no por la lógica de scroll del juego). Se fuerza captura tras el borrado y se conserva la página previa completa. Imagen privada bank-preview-fixed.png muestra Bomb completo en su panel, sin capa Funds. Se leen asimismo bases/orígenes reales de StageState del proceso para reconstruir el mundo. La máscara de ventana está corregida, pero la imagen bank-world-actual-fixed.png todavía presenta límite superior coincidente con viewport nativo: NO afirmar cobertura completa del stage; falta comprobar en juego y estudiar el productor si persiste. UI/lista poblada medidas; cobertura final del escenario pendiente.

Regresión209 core y35 host aprobadas en serie. Windows compilado como target/release/FFTARecomp-MonsterBankPopulatedFix.exe. ffta.sav mantiene SHA256 5E38568C9EC73109C9BB85B6DA202837954F18EF43C58C2D24B6BB4C81AEC4B8. No se modificó el proceso abierto ni el save original.

## Batalla: clic libre, menú directo y geometría estable (2026-10-22)

Solicitud del usuario: seleccionar personaje con clic=A; sin caja/lista seleccionable, clic debe confirmar; Begin Battle perdió mouse; revisar entrada/salida del panel nombre/HP/MP/JP/LV y menú, con selección directa1?3. Referencia estática del proceso2988 guardada target/battle-current.png; RAM leída sin modificar el proceso/save. En escena abierta Undine con Move/Action/Wait/Status, MenuScene cursor2DDD0 cuenta4 seleccionado3, origen184,84. GRID no activo con menú abierto, como corresponde.

La guarda host ya no considera action_layout por sí solo un menú: el panel de información sin selector no bloquea A de fondo. Frontend, listas reconocidas, servicios y Notice mantienen su protección contra clics fuera. Las listas de acción verificadas callback18FC5/flags7 publican directamente índice/offset y paletas B/C de su buffer. El lector de batalla acepta el modelo dueño antes de OAM, porque la publicación del sprite queda un frame atrás y causaba perder el selector tras cambiar de fila.

Propietario de ventana de acciones medido2DD84: mapas0600E000/06005000, estilo0140/0C20/type00010005, rect[22,8,8,11]. El lector usa geometría canónica aun con extents de entrada/salida parciales y sin cursor; no depende de adivinar borde desde tiles parciales. Panel ActionInfo usa canvas fijo[0,48,160,160] para que el contorno visible variable del retrato y del backing no cambie el origen ni escala. Stats con destino canónico ya no expande sus bounds según píxeles deslizantes de la animación. La posición local del contenido original sigue animándose: el parche estabiliza el layout host; no afirmar que todas las transiciones del juego están resueltas sin feedback.

Notice se detecta por las mismas firmas de marco y ventanas originales antes de depender de action_layout: cubre el aviso previo a batalla aunque no esté activo el menú de acciones. No hubo captura real del Begin Battle en este turno (solicitada al usuario); queda validar manualmente ese caso y las animaciones en movimiento.

Prueba battle_current_probe con RAM real: targets0,3,1,2 inmediatos sin teclas direccionales y sin actualizar OAM artificialmente; cinco extents1/4/8/8?0/1?0 conservan bounds constantes del menú y ActionInfo. Regresión209 core +35 host pasadas antes del último ajuste de lectura por propietario; suite de mouse repetida después. Windows target/release/FFTARecomp-BattleUIFix.exe. Pendiente prueba visual del usuario; no se modifica partida original, ni Android/Linux/instaladores.

Seguimiento: el último reconocimiento sin OAM se restringe a action_layout. La prueba compartida de lista con flecha ancha detectó inicialmente que la lectura nueva aceptaba un contexto distinto; tras el límite30/30 pruebas mouse pasan. Se repitió battle_current_probe: todos los saltos y extents pasan. Windows se recompila después de esta restricción.

## Pendiente visual registrado: Notice duplicado en despliegue (2026-10-22)

Usuario pidió solo capturar y registrar para continuar revisando. Captura target/battle-notice-repeated.png, proceso14612 FFTARecomp-BattleUIFix, 1704x480. Texto «Select units to send into battle.» aparece dos veces: modal centrado y otra copia arriba a la derecha con flecha inferior. La copia superior conserva apariencia/posición del plano nativo y la central corresponde al modal extraído; hipótesis de doble captura/asignación de capas, aún no verificada. No se aplicó parche en este turno; usuario continúa revisando.
