package com.fftarecomp.port

import android.content.Context
import android.content.pm.ActivityInfo
import android.os.Bundle
import android.hardware.input.InputManager
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.compose.foundation.Image
import androidx.compose.ui.res.painterResource
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.util.concurrent.atomic.AtomicInteger

private val Paper = Color(0xFFF6EBD0)
private val Ink = Color(0xFF302519)
private val Gold = Color(0xFFB68B35)
private val Forest = Color(0xFF14231C)

/** Separate sources prevent releasing a touch button from releasing a held controller button. */
class GameInput {
    val buttons = AtomicInteger(0)
    private val sources = mutableMapOf<String, Int>()
    @Synchronized fun set(source: String, mask: Int) {
        if (mask == 0) sources.remove(source) else sources[source] = mask
        buttons.set(sources.values.fold(0) { value, next -> value or next })
    }
    @Synchronized fun clear() { sources.clear(); buttons.set(0) }
}

@Stable
class GameSettings(context: Context) {
    private val prefs = context.getSharedPreferences("video_audio_input", Context.MODE_PRIVATE)
    val touch=TouchLayout(context)
    var aspect by mutableStateOf(AspectMode.entries.getOrElse(prefs.getInt("aspect",AspectMode.DEVICE.ordinal)) { AspectMode.DEVICE })
    var zoom by mutableIntStateOf(prefs.getInt("zoom",0).coerceIn(0,ViewConfig.maxZoom(aspect)))
    var sound by mutableStateOf(prefs.getBoolean("sound",true))
    var volume by mutableFloatStateOf(prefs.getFloat("volume",0.8f).coerceIn(0f,1f))
    var controls by mutableStateOf(prefs.getBoolean("controls",true))
    var opacity by mutableFloatStateOf(prefs.getFloat("opacity",0.65f).coerceIn(0.2f,1f))
    var controlSize by mutableFloatStateOf(prefs.getFloat("size",1f).coerceIn(0.75f,1.25f))
    var bubbleSize by mutableFloatStateOf(prefs.getFloat("bubble_size",1f).coerceIn(.5f,2f))
    var bubbleLength by mutableFloatStateOf(prefs.getFloat("bubble_length",1f).coerceIn(1f,2f))
    var video by mutableStateOf(VideoOptions(
        screen=prefs.getInt("video_screen",0).coerceIn(0,4),
        brightness=prefs.getFloat("video_brightness",1f).coerceIn(.15f,1.5f),
        darken=prefs.getFloat("video_darken",-1f).coerceIn(-1f,1f),
        response=prefs.getInt("video_response",0).coerceIn(0,3),
        grid=prefs.getInt("video_grid",0).coerceIn(0,3),
        scaling=prefs.getInt("video_scaling",1).coerceIn(0,3),
        vsync=prefs.getBoolean("video_vsync",true),fullscreen=prefs.getBoolean("video_fullscreen",true)))
    fun videoChange(value:VideoOptions){video=value;save()}
    var multiplier by mutableFloatStateOf(prefs.getFloat("speed_rate",
        prefs.getInt("speed",2).coerceIn(1,4).toFloat()).let{saved->FAST_SPEEDS.minBy{kotlin.math.abs(it-saved)}})
    var audioMode by mutableStateOf(FastAudio.entries.getOrElse(prefs.getInt("ff_audio",0)){FastAudio.NORMAL})
    var touchVisible by mutableStateOf(true)
    var controllerConnected by mutableStateOf(false)
    var fastForward by mutableStateOf(false)
    var holdFast by mutableStateOf(false)
    val pad=PadBindings().apply {
        PadAction.entries.forEach{action->
            runCatching{PadControl.valueOf(prefs.getString("pad_"+action.name,action.default.name)!!)}
                .getOrNull()?.let{bindings[action]=it}
        }
        leftStickMovement=prefs.getBoolean("pad_left_stick",true)
    }
    var bindingRevision by mutableIntStateOf(0)
    var captureAction by mutableStateOf<PadAction?>(null)
    var pauseRequest by mutableIntStateOf(0)
    fun bind(action:PadAction,button:PadControl){
        pad.assign(action,button);captureAction=null;bindingRevision++;savePad()
    }
    fun savePad(){
        prefs.edit().apply {
            PadAction.entries.forEach{putString("pad_"+it.name,pad.bindings.getValue(it).name)}
            putBoolean("pad_left_stick",pad.leftStickMovement)
        }.apply()
    }
    fun captureKey(event:KeyEvent):Boolean{
        val action=captureAction?:return false
        val button=PadControl.fromKey(event.keyCode)?:return false
        if(event.action==KeyEvent.ACTION_DOWN && event.repeatCount==0)bind(action,button)
        return true
    }
    fun captureMotion(event:MotionEvent):Boolean{
        val action=captureAction?:return false
        PadControl.axes{event.getAxisValue(it)}.firstOrNull()?.let{bind(action,it)}
        return true
    }
    @Volatile var inputActive = false
    fun zoomBy(delta: Int) { zoom=(zoom+delta).coerceIn(0,ViewConfig.maxZoom(aspect));save() }
    fun save() { prefs.edit().putInt("aspect",aspect.ordinal).putInt("zoom",zoom)
        .putBoolean("sound",sound).putFloat("volume",volume).putBoolean("controls",controls)
        .putFloat("opacity",opacity).putFloat("size",controlSize).putFloat("speed_rate",multiplier).putInt("ff_audio",audioMode.ordinal)
        .putInt("video_screen",video.screen).putFloat("video_brightness",video.brightness)
        .putFloat("video_darken",video.darken).putInt("video_response",video.response)
        .putInt("video_grid",video.grid).putInt("video_scaling",video.scaling)
        .putBoolean("video_vsync",video.vsync).putBoolean("video_fullscreen",video.fullscreen)
        .putFloat("bubble_size",bubbleSize).putFloat("bubble_length",bubbleLength).apply() }
}

class MainActivity : AppCompatActivity(), InputManager.InputDeviceListener {
    private val input = GameInput()
    private lateinit var settings: GameSettings
    private var inGame = false
    private lateinit var inputManager: InputManager
    private fun isController(device:InputDevice?):Boolean = device!=null && ControllerInput.recognizes(0,device.sources)
    private fun refreshControllers() {
        val connected=InputDevice.getDeviceIds().any{isController(InputDevice.getDevice(it))}
        if(connected!=settings.controllerConnected) {
            input.clear();settings.pad.clear();settings.holdFast=false
            settings.controllerConnected=connected
            settings.touchVisible=!connected
        }
    }
    override fun onInputDeviceAdded(deviceId:Int)=refreshControllers()
    override fun onInputDeviceRemoved(deviceId:Int)=refreshControllers()
    override fun onInputDeviceChanged(deviceId:Int)=refreshControllers()
    private fun useController() {
        if(settings.touchVisible){input.clear();settings.pad.clear();settings.holdFast=false
            if(BuildConfig.DEBUG)android.util.Log.d("FFTAInput","controller: touch overlay hidden")}
        settings.touchVisible=false
        
    }
    override fun dispatchTouchEvent(event:MotionEvent):Boolean {
        if(inGame && event.actionMasked==MotionEvent.ACTION_DOWN &&
            event.getToolType(0)==MotionEvent.TOOL_TYPE_FINGER) settings.touchVisible=true
        return super.dispatchTouchEvent(event)
    }
    override fun onDestroy() {
        inputManager.unregisterInputDeviceListener(this)
        super.onDestroy()
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        settings = GameSettings(this)
        inputManager=getSystemService(Context.INPUT_SERVICE) as InputManager
        inputManager.registerInputDeviceListener(this,null)
        refreshControllers()
        setContent {
            MaterialTheme(colorScheme=lightColorScheme(primary=Ink,onPrimary=Paper,
                secondary=Gold,background=Forest,surface=Paper,onSurface=Ink)) {
                MainScreen(settings,input) { inGame=it }
            }
        }
    }
    override fun onPause() { input.clear();settings.pad.clear();settings.holdFast=false;super.onPause() }
    private fun routePad(source:String,held:Set<PadControl>) {
        for((action,down) in settings.pad.update(source,held)){
            if(action.mask!=0)input.set("pad:"+action.name,if(down)action.mask else 0)
            else when(action){
                PadAction.FAST->settings.holdFast=down
                PadAction.TOGGLE_FAST->if(down)settings.fastForward=!settings.fastForward
                PadAction.ZOOM_IN->if(down)settings.zoomBy(-1)
                PadAction.ZOOM_OUT->if(down)settings.zoomBy(1)
                PadAction.PAUSE->if(down){settings.pauseRequest++;input.clear();settings.holdFast=false}
                else->Unit
            }
        }
    }
    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        val controller=ControllerInput.recognizes(event.source,event.device?.sources?:0)
        if(inGame && controller && settings.captureKey(event))return true
        if (!inGame || !settings.inputActive) return super.dispatchKeyEvent(event)
        if(controller){
            if(event.action==KeyEvent.ACTION_DOWN)useController()
            val button=PadControl.fromKey(event.keyCode)
            if(button!=null){
                routePad("key:"+event.deviceId+":"+event.keyCode,if(event.action==KeyEvent.ACTION_DOWN)setOf(button)else emptySet())
                return true
            }
        }
        val down = event.action == KeyEvent.ACTION_DOWN
        val bit = when(event.keyCode) {
            KeyEvent.KEYCODE_BUTTON_A,KeyEvent.KEYCODE_Z -> 0
            KeyEvent.KEYCODE_BUTTON_B,KeyEvent.KEYCODE_X -> 1
            KeyEvent.KEYCODE_BUTTON_SELECT,KeyEvent.KEYCODE_TAB -> 2
            KeyEvent.KEYCODE_BUTTON_START,KeyEvent.KEYCODE_ENTER -> 3
            KeyEvent.KEYCODE_DPAD_RIGHT -> 4
            KeyEvent.KEYCODE_DPAD_LEFT -> 5
            KeyEvent.KEYCODE_DPAD_UP -> 6
            KeyEvent.KEYCODE_DPAD_DOWN -> 7
            KeyEvent.KEYCODE_BUTTON_R2 -> 8
            KeyEvent.KEYCODE_BUTTON_L2 -> 9
            else -> -1
        }
        if (bit>=0) { input.set("key:${event.keyCode}",if(down) 1 shl bit else 0);return true }
        if (event.keyCode in listOf(KeyEvent.KEYCODE_BUTTON_L1,KeyEvent.KEYCODE_BUTTON_R1,
                KeyEvent.KEYCODE_MINUS,KeyEvent.KEYCODE_EQUALS,KeyEvent.KEYCODE_BUTTON_THUMBL,KeyEvent.KEYCODE_BUTTON_THUMBR)) {
            if(event.keyCode==KeyEvent.KEYCODE_BUTTON_THUMBL) settings.holdFast=down
            else if(down && event.repeatCount==0) when(event.keyCode) {
                KeyEvent.KEYCODE_BUTTON_L1,KeyEvent.KEYCODE_MINUS -> settings.zoomBy(1)
                KeyEvent.KEYCODE_BUTTON_R1,KeyEvent.KEYCODE_EQUALS -> settings.zoomBy(-1)
                KeyEvent.KEYCODE_BUTTON_THUMBR -> settings.fastForward=!settings.fastForward
            }
            return true
        }
        return super.dispatchKeyEvent(event)
    }
    override fun onGenericMotionEvent(event: MotionEvent): Boolean {
        if(inGame && ControllerInput.recognizes(event.source,event.device?.sources?:0) && event.action==MotionEvent.ACTION_MOVE){
            if(settings.captureMotion(event))return true
            if(settings.inputActive){
                val held=PadControl.axes{event.getAxisValue(it)}
                if(held.isNotEmpty() || listOf(MotionEvent.AXIS_X,MotionEvent.AXIS_Y,MotionEvent.AXIS_Z,MotionEvent.AXIS_RZ,MotionEvent.AXIS_LTRIGGER,MotionEvent.AXIS_RTRIGGER).any{kotlin.math.abs(event.getAxisValue(it))>0.25f})useController()
                routePad("axes:"+event.deviceId,held)
                return true
            }
        }
        return super.onGenericMotionEvent(event)
    }
}

@Composable
private fun MainScreen(settings: GameSettings, input: GameInput, setInGame: (Boolean)->Unit) {
    val context=LocalContext.current
    val scope=rememberCoroutineScope()
    var playing by remember { mutableStateOf(false) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(NativeBridge.loadError) }
    var status by remember { mutableStateOf("Select your unmodified FFTA European (EUR) ROM to begin.") }
    val rom=remember { File(context.filesDir,"ffta.gba") }
    val save=remember { File(context.filesDir,if(GameBenchmark.isolatedSaves(context))"benchmark.sav" else "ffta.sav") }
    val startup=remember{StartupState(context)}
    val initialRoute=remember {
        val start=System.nanoTime()
        val ready=startup.ready
        val present=rom.isFile
        val route=StartupPolicy.route(ready,present,NativeBridge.loadError!=null,startup.recorded)
        if(BuildConfig.DEBUG)android.util.Log.d("FFTAStartup","route=$route ready=$ready rom=$present check_ms=${(System.nanoTime()-start)/1e6}")
        route
    }
    var available by remember { mutableStateOf(rom.isFile) }
    var mustSelectRom by remember { mutableStateOf(false) }
    var switchingRom by remember { mutableStateOf(false) }
    var loading by remember {mutableStateOf(initialRoute==StartupRoute.LOADING)}
    // Loading and gameplay share one orientation owner. A screen-specific
    // onDispose must not restore portrait between these two compositions.
    val useLandscape=loading || playing
    DisposableEffect(useLandscape) {
        val activity=context as AppCompatActivity
        val previous=activity.requestedOrientation
        if(useLandscape)activity.requestedOrientation=ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
        onDispose {if(useLandscape)activity.requestedOrientation=previous}
    }

    LaunchedEffect(Unit) {
        if(loading) {
            busy=true
            try {
                withContext(Dispatchers.Default){
                    if(!NativeBridge.romVerified)NativeBridge.load(rom,save)
                    if(!NativeBridge.compiled)NativeBridge.prepare()
                    startup.setReady(true)
                }
                playing=true
            }catch(failure:Throwable){
                withContext(Dispatchers.IO){runCatching{startup.setReady(false)}}
                error=failure.message
            }
            finally{busy=false;loading=false}
        } else if(!available || NativeBridge.loadError!=null) {
            withContext(Dispatchers.IO){runCatching{startup.setReady(false)}}
        }
    }
    val picker=rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        if(uri!=null) {
            busy=true;error=null
            scope.launch {
                try {
                    withContext(Dispatchers.IO) {
                        val temp=File(context.filesDir,"import.gba")
                        try {
                            context.contentResolver.openInputStream(uri)?.use { source ->
                                temp.outputStream().use { target ->
                                    val block=ByteArray(65536);var total=0L
                                    while(true) { val n=source.read(block);if(n<0)break
                                        total+=n;require(total<=32L*1024*1024){"File is larger than a GBA ROM"};target.write(block,0,n) }
                                }
                            } ?: throw IllegalStateException("Cannot read the selected document")
                            if(NativeBridge.romVerified)NativeBridge.gbaSave()
                            NativeBridge.load(temp,save)
                            temp.copyTo(rom,overwrite=true)
                            mustSelectRom=false
                            startup.setReady(false)
                        } finally { temp.delete() }
                    }
                    available=true;status="ROM verified. Ready to compile on this device."
                } catch(failure: Throwable) { error=failure.message }
                finally { busy=false }
            }
        }
    }
    LaunchedEffect(busy) {
        while(busy) {
            if(NativeBridge.loadError==null) {
                status=NativeBridge.gbaStatus()
                withContext(Dispatchers.IO) { File(context.filesDir,"last-stage.txt").writeText(status) }
            }
            delay(250)
        }
    }
    LaunchedEffect(playing,switchingRom) { setInGame(playing&&!switchingRom) }
    if(loading) {
        LoadingScreen()
    } else if(playing&&!switchingRom) {
        GameScreen(settings,input,onExit={playing=false;switchingRom=false},onSwitchRom={
            if(NativeBridge.gbaSave()!=1) {
                false
            } else {
                startup.setReady(false)
                mustSelectRom=true
                status="Select an unmodified FFTA European (EUR) ROM to continue."
                switchingRom=true
                true
            }
        })
    } else {
        Box(Modifier.fillMaxSize().background(Forest).safeDrawingPadding(),contentAlignment=Alignment.Center) {
            Column(Modifier.widthIn(max=540.dp).fillMaxWidth().padding(20.dp)
                .background(Paper,RoundedCornerShape(20.dp)).border(1.dp,Gold,RoundedCornerShape(20.dp))
                .padding(24.dp).verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(16.dp)) {
                Text("FFTARecomp",fontFamily=FontFamily.Serif,fontWeight=FontWeight.Bold,fontSize=32.sp,color=Ink)
                Text("ANDROID PREVIEW",color=Gold,letterSpacing=3.sp,fontSize=11.sp)
                HorizontalDivider(color=Gold)
                Text("Your adventure, a wider view.",fontFamily=FontFamily.Serif,fontSize=20.sp)
                Text("Select your own unmodified FFTA European (EUR) ROM. The game is compiled locally; your ROM stays on this device.")
                Button(onClick={picker.launch(arrayOf("*/*"))},enabled=!busy&&NativeBridge.loadError==null,
                    modifier=Modifier.fillMaxWidth()) { Text(if(available)"Change ROM" else "Select ROM") }
                Button(enabled=available&&!mustSelectRom&&!busy&&NativeBridge.loadError==null,onClick={
                    busy=true;loading=true;error=null
                    scope.launch {
                        try {
                            withContext(Dispatchers.Default) {
                                if(!NativeBridge.romVerified)NativeBridge.load(rom,save)
                                if(!NativeBridge.compiled)NativeBridge.prepare()
                                startup.setReady(true)
                            }
                            playing=true
                            switchingRom=false
                        } catch(failure: Throwable) {
                            withContext(Dispatchers.IO){runCatching{startup.setReady(false)}}
                            error=failure.message
                        }
                        finally{busy=false;loading=false}
                    }
                },modifier=Modifier.fillMaxWidth()) {Text(if(NativeBridge.compiled)"Continue game" else "Play")}
                if(busy)LinearProgressIndicator(Modifier.fillMaxWidth())
                Text(status,fontSize=13.sp)
                error?.let {Text(it,color=Color(0xFF9B2929),fontSize=13.sp)}
                Text("A project by Jorge Rios",fontSize=12.sp,color=Ink.copy(alpha=0.6f))
            }
        }
    }
}


@Composable
private fun LoadingScreen() {
    Box(Modifier.fillMaxSize().background(Forest).safeDrawingPadding(),contentAlignment=Alignment.Center) {
        Column(Modifier.widthIn(max=360.dp).fillMaxWidth().padding(24.dp)
            .background(Paper,RoundedCornerShape(20.dp))
            .border(1.dp,Gold,RoundedCornerShape(20.dp)).padding(28.dp),
            horizontalAlignment=Alignment.CenterHorizontally,verticalArrangement=Arrangement.spacedBy(18.dp)) {
            Image(painterResource(R.drawable.ffta_logo),contentDescription=null,modifier=Modifier.size(112.dp))
            Text("FFTARecomp",fontFamily=FontFamily.Serif,fontWeight=FontWeight.Bold,fontSize=28.sp,color=Ink)
            HorizontalDivider(color=Gold.copy(alpha=0.65f))
            Text("LOADING",color=Gold,letterSpacing=3.sp,fontSize=11.sp)
            LinearProgressIndicator(modifier=Modifier.fillMaxWidth().height(3.dp),color=Gold,trackColor=Ink.copy(alpha=0.1f))
            Text("Your adventure awaits.",fontFamily=FontFamily.Serif,fontSize=16.sp,color=Ink.copy(alpha=0.7f))
        }
    }
}

@OptIn(ExperimentalLayoutApi::class)
@Composable
private fun GameScreen(settings:GameSettings,input:GameInput,onExit:()->Unit,onSwitchRom:()->Boolean) {
    val context=LocalContext.current
    val activity=context as AppCompatActivity
    val lifecycle=activity.lifecycle
    var surface by remember {mutableStateOf<GameSurface?>(null)}
    var paused by remember {mutableStateOf(false)}
    var editingTouch by remember {mutableStateOf(false)}
    var originalTouch by remember {mutableStateOf<Map<TouchControl,TouchPlacement>>(emptyMap())}
    var originalVibration by remember {mutableStateOf(false)}
    var failure by remember {mutableStateOf<String?>(null)}
    var switchRomError by remember {mutableStateOf<String?>(null)}
    var page by remember {mutableStateOf("main")}
    // Snapshot reads must happen during composition, not only inside SideEffect.
    val request=ViewRequest(settings.aspect,settings.zoom)
    val speed=if(settings.fastForward!=settings.holdFast)settings.multiplier else 1f
    val audioMode=settings.audioMode
    val sound=settings.sound
    val volume=settings.volume
    val touchClicks=settings.touch.experimentalClicks
    val video=settings.video
    val bubbleSize=settings.bubbleSize
    val bubbleLength=settings.bubbleLength
    LaunchedEffect(settings.pauseRequest){
        if(settings.pauseRequest>0){paused=true;page="main";input.clear();settings.pad.clear();settings.holdFast=false}
    }
    BackHandler {
        if(editingTouch){
            settings.touch.layout.restore(originalTouch);settings.touch.vibration=originalVibration;editingTouch=false
        }else paused=!paused
        input.clear();settings.pad.clear();settings.holdFast=false
    }
    DisposableEffect(Unit) {
        val oldCutout=if(android.os.Build.VERSION.SDK_INT>=28)activity.window.attributes.layoutInDisplayCutoutMode else 0
        if(android.os.Build.VERSION.SDK_INT>=28)activity.window.attributes=activity.window.attributes.apply{
            layoutInDisplayCutoutMode=android.view.WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
        activity.window.addFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        WindowCompat.setDecorFitsSystemWindows(activity.window,false)
        val bars=WindowInsetsControllerCompat(activity.window,activity.window.decorView)
        bars.hide(WindowInsetsCompat.Type.systemBars())
        bars.systemBarsBehavior=WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        onDispose {
            input.clear();settings.pad.clear();settings.holdFast=false;settings.inputActive=false;settings.save()
            activity.window.clearFlags(android.view.WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
            WindowCompat.setDecorFitsSystemWindows(activity.window,true)
            bars.show(WindowInsetsCompat.Type.systemBars())
            if(android.os.Build.VERSION.SDK_INT>=28)activity.window.attributes=activity.window.attributes.apply{layoutInDisplayCutoutMode=oldCutout}
        }
    }
    DisposableEffect(surface,lifecycle) {
        val view=surface
        val observer=LifecycleEventObserver {_,event->
            if(event==Lifecycle.Event.ON_PAUSE){view?.appActive=false;input.clear();view?.onPause()}
            if(event==Lifecycle.Event.ON_RESUME){view?.onResume();view?.appActive=true}
        }
        view?.appActive=lifecycle.currentState.isAtLeast(Lifecycle.State.RESUMED)
        lifecycle.addObserver(observer)
        onDispose{lifecycle.removeObserver(observer)}
    }
    // Dialog dismissal can restore navigation bars after the composition's
    // immediate hide (notably on custom ROMs). Reapply once after dismissal;
    // no polling, and windowed mode keeps its system bars.
    LaunchedEffect(paused,editingTouch,video.fullscreen) {
        if(!paused && !editingTouch && video.fullscreen) {
            delay(250)
            val bars=WindowInsetsControllerCompat(activity.window,activity.window.decorView)
            bars.systemBarsBehavior=WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
            bars.hide(WindowInsetsCompat.Type.systemBars())
        }
    }
    SideEffect {
        settings.inputActive = !paused && !editingTouch && failure==null
        if(!settings.inputActive){settings.pad.clear();settings.holdFast=false}
        surface?.gamePaused=paused||editingTouch||failure!=null
        surface?.videoOptions=video
        val bars=WindowInsetsControllerCompat(activity.window,activity.window.decorView)
        if(video.fullscreen)bars.hide(WindowInsetsCompat.Type.systemBars())else bars.show(WindowInsetsCompat.Type.systemBars())
        surface?.viewRequest=request
        surface?.speed=speed
        surface?.audioMode=audioMode
        surface?.soundEnabled=sound
        surface?.volume=volume
        surface?.experimentalClicks=touchClicks
        surface?.bubbleSize=bubbleSize
        surface?.bubbleLength=bubbleLength
    }
    Box(Modifier.fillMaxSize().background(Color.Black)) {
        AndroidView(factory={ctx->GameSurface(ctx){error->
            activity.runOnUiThread{failure=error;paused=true}
        }.also{surface=it;it.inputSource=input.buttons}},modifier=Modifier.fillMaxSize())
        if((settings.touchVisible&&!paused)||editingTouch)TouchOverlay(settings,input,editingTouch,
            onPause={paused=true;page="main";input.clear();settings.pad.clear();settings.holdFast=false},
            onSave={settings.touch.save();editingTouch=false},
            onCancel={settings.touch.layout.restore(originalTouch);settings.touch.vibration=originalVibration;editingTouch=false})
        if(paused&&!editingTouch) {
            Dialog(onDismissRequest={if(failure==null)paused=false},properties=DialogProperties(usePlatformDefaultWidth=false)) {
                Column(Modifier.widthIn(max=490.dp).fillMaxWidth(0.9f)
                    .heightIn(max=(LocalConfiguration.current.screenHeightDp-28).dp)
                    .background(Paper,RoundedCornerShape(18.dp)).border(1.dp,Gold,RoundedCornerShape(18.dp))
                    .padding(20.dp).verticalScroll(rememberScrollState()),verticalArrangement=Arrangement.spacedBy(8.dp)) {
                    Text(when(page){"main"->"FFTARecomp";"rom"->"Switch ROM";else->page.replaceFirstChar{it.uppercase()}},fontFamily=FontFamily.Serif,fontSize=26.sp,fontWeight=FontWeight.Bold,color=Ink)
                    Text("PAUSED",letterSpacing=3.sp,fontSize=10.sp,color=Gold)
                    HorizontalDivider(color=Gold)
                    failure?.let{Text(it,color=Color(0xFF9B2929),fontSize=12.sp)}
                    switchRomError?.let{Text("Could not save before switching: $it",color=Color(0xFF9B2929),fontSize=12.sp)}
                    when(page) {
                        "main"->{
                            MenuButton("Resume",failure==null){paused=false}
                            MenuButton("Video & zoom"){page="video"}
                            MenuButton("Fast forward"){page="fast forward"}
                            MenuButton("Text bubbles"){page="text bubbles"}
                            MenuButton("Audio"){page="audio"}
                            MenuButton("Input"){page="input"}
                            MenuButton("Switch ROM"){
                                switchRomError=null
                                if(!onSwitchRom()) switchRomError=NativeBridge.gbaStatus()
                            }
                            MenuButton("Exit Game"){onExit()}
                        }
                        "video"->{
                            Row(verticalAlignment=Alignment.CenterVertically){Switch(settings.video.fullscreen,{settings.videoChange(settings.video.copy(fullscreen=it))});Text("Fullscreen")}
                            Text("Window mode on Android shows the system bars.",fontSize=12.sp)
                            VideoChoices("Screen model",listOf("Raw","Unlit","Frontlit","Backlit","Classic"),settings.video.screen){settings.videoChange(settings.video.copy(screen=it))}
                            Text("Brightness: ${(settings.video.brightness*100).toInt()}%")
                            Slider(settings.video.brightness,{settings.videoChange(settings.video.copy(brightness=it))},valueRange=.15f..1.5f)
                            Row(verticalAlignment=Alignment.CenterVertically){Switch(settings.video.darken<0f,{settings.videoChange(settings.video.copy(darken=if(it)-1f else 0f))});Text("Automatic panel darkness")}
                            if(settings.video.darken>=0f)Slider(settings.video.darken,{settings.videoChange(settings.video.copy(darken=it))})
                            VideoChoices("LCD response",listOf("Off","Simple","Smart","Persistence"),settings.video.response){settings.videoChange(settings.video.copy(response=it))}
                            VideoChoices("Pixel grid",listOf("Off","Auto","Light","Strong"),settings.video.grid){settings.videoChange(settings.video.copy(grid=it))}
                            VideoChoices("Scaling",listOf("Off","Nearest","Sharp","EPX"),settings.video.scaling){settings.videoChange(settings.video.copy(scaling=it))}
                            Row(verticalAlignment=Alignment.CenterVertically){Switch(settings.video.vsync,{settings.videoChange(settings.video.copy(vsync=it))});Text("VSync")}
                            SettingsDropdown("Aspect ratio",AspectMode.entries.map{it.label},settings.aspect.ordinal){index->
                                val aspect=AspectMode.entries[index]
                                settings.aspect=aspect;settings.zoom=settings.zoom.coerceAtMost(ViewConfig.maxZoom(aspect));settings.save()
                            }
                            Text("Zoom out: ${settings.zoom} / ${ViewConfig.maxZoom(settings.aspect)}")
                            Row(horizontalArrangement=Arrangement.spacedBy(10.dp)) {
                                OutlinedButton(onClick={settings.zoomBy(1)}){Text("- Show more")}
                                OutlinedButton(onClick={settings.zoomBy(-1)}){Text("+ Show less")}
                            }
                            val view=ViewConfig.create(settings.aspect,settings.zoom)
                            Text("Render: ${view.width} x ${view.height}",fontSize=12.sp)
                        }
                        "fast forward"->{
                            Row(verticalAlignment=Alignment.CenterVertically){Switch(settings.fastForward,{settings.fastForward=it});Text("Always on")}
                            SettingsDropdown("Speed",FAST_SPEEDS.map{speedLabel(it)},FAST_SPEEDS.indexOf(settings.multiplier)){index->
                                settings.multiplier=FAST_SPEEDS[index];settings.save()
                            }
                            SettingsDropdown("Audio mode",FastAudio.entries.map{it.label},settings.audioMode.ordinal){index->
                                settings.audioMode=FastAudio.entries[index];settings.save()
                            }
                            Text(when(settings.audioMode){
                                FastAudio.NORMAL->"Audio plays at its normal speed while the game advances faster."
                                FastAudio.FAST->"Audio speeds up with the game."
                                FastAudio.SILENT->"No audio during fast forward."
                            },fontSize=13.sp)
                            Text("Controller: hold L3 to switch speed temporarily; press R3 to toggle.",fontSize=13.sp)
                        }
                        "audio"->{
                            Row(verticalAlignment=Alignment.CenterVertically){Switch(settings.sound,{settings.sound=it;settings.save()});Text("Game audio")}
                            Text("Volume: ${(settings.volume*100).toInt()}%")
                            Slider(settings.volume,{settings.volume=it},onValueChangeFinished={settings.save()})
                        }
                        "input"->{
                            MenuButton("Touchscreen"){page="touchscreen"}
                            MenuButton("Gamepad"){page="gamepad"}
                        }
                        "text bubbles"->{
                            Text("Size: ${(settings.bubbleSize*100).toInt()}%")
                            Slider(settings.bubbleSize,{settings.bubbleSize=it},valueRange=.5f..2f,onValueChangeFinished={settings.save()})
                            Text("Length: ${(settings.bubbleLength*100).toInt()}%")
                            Slider(settings.bubbleLength,{settings.bubbleLength=it},valueRange=1f..2f,onValueChangeFinished={settings.save()})
                            MenuButton("Default"){settings.bubbleSize=1f;settings.bubbleLength=1f;settings.save()}
                        }
                        "touchscreen"->TouchscreenSettings(settings){
                            originalTouch=settings.touch.layout.snapshot();originalVibration=settings.touch.vibration
                            input.clear();settings.pad.clear();settings.holdFast=false
                            editingTouch=true
                        }
                        "gamepad"->GamepadSettings(settings)

                    }
                    if(page!="main")MenuButton("Back"){settings.save();page=if(page=="touchscreen"||page=="gamepad")"input" else "main"}
                }
            }
        }
    }
}

@Composable
private fun MenuButton(label:String,enabled:Boolean=true,click:()->Unit) {
    OutlinedButton(onClick=click,enabled=enabled,modifier=Modifier.fillMaxWidth(),shape=RoundedCornerShape(8.dp)) {
        Text(label,Modifier.fillMaxWidth(),fontFamily=FontFamily.Serif,fontSize=17.sp)
    }
}


@Composable
private fun VideoChoices(title:String,options:List<String>,selected:Int,onSelect:(Int)->Unit){
    SettingsDropdown(title,options,selected,onSelect)
}

/** One compact value; popup owns outside taps and vertical finger scrolling. */
@Composable
private fun SettingsDropdown(title:String,options:List<String>,selected:Int,onSelect:(Int)->Unit){
    var expanded by remember { mutableStateOf(false) }
    val maxHeight=(LocalConfiguration.current.screenHeightDp*0.5f).dp.coerceAtMost(280.dp)
    val menuScroll=rememberScrollState((selected.coerceAtLeast(0)*48*LocalDensity.current.density).toInt())
    Text(title,color=Ink,fontFamily=FontFamily.Serif)
    BoxWithConstraints(Modifier.fillMaxWidth()) {
        val menuWidth=maxWidth
        OutlinedButton(onClick={expanded=!expanded},modifier=Modifier.fillMaxWidth(),
            shape=RoundedCornerShape(8.dp),colors=ButtonDefaults.outlinedButtonColors(contentColor=Ink)) {
            Text(options.getOrElse(selected){options.firstOrNull().orEmpty()},Modifier.weight(1f),fontSize=16.sp)
            Text(if(expanded)"\u25B4" else "\u25BE",color=Gold,fontSize=18.sp)
        }
        DropdownMenu(expanded=expanded,onDismissRequest={expanded=false},scrollState=menuScroll,
            modifier=Modifier.width(menuWidth).heightIn(max=maxHeight).background(Paper)) {
            options.forEachIndexed { index,label ->
                DropdownMenuItem(text={Text(label,color=Ink,fontFamily=FontFamily.Serif,
                    fontWeight=if(index==selected)FontWeight.Bold else FontWeight.Normal)},
                    trailingIcon={if(index==selected)Text("\u2713",color=Gold)},
                    modifier=Modifier.background(if(index==selected)Gold.copy(alpha=.18f) else Color.Transparent),
                    onClick={expanded=false;onSelect(index)})
            }
        }
    }
}
