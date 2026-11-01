package com.fftarecomp.port

import android.content.Context
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import android.opengl.GLES20.*
import android.opengl.GLSurfaceView
import android.os.Process
import android.view.MotionEvent
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.locks.LockSupport
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.opengles.GL10

/** Simulation and audio run together off the UI thread; GL only uploads completed frames. */
class GameSurface(context: Context, val onError: (String) -> Unit) : GLSurfaceView(context) {
    companion object { val deliveredClicks=AtomicInteger(0) }
    val buttons = AtomicInteger(0)
    @Volatile var inputSource: AtomicInteger = buttons
    @Volatile var saveSelectorActive=false
    @Volatile var gamePaused = false
    @Volatile var appActive = true
    var videoOptions:VideoOptions
        get()=frames.videoOptions
        set(value){frames.videoOptions=value;requestRender()}
    @Volatile var viewRequest = ViewRequest(AspectMode.DEVICE,0)
    @Volatile var speed = 1f
    @Volatile var audioMode=FastAudio.NORMAL
    @Volatile var soundEnabled = true
    @Volatile var volume = 0.8f
    @Volatile var experimentalClicks=false
    @Volatile private var naming=false
    val nameEntryActive get()=naming
    @Volatile var bubbleSize=1f
    @Volatile var bubbleLength=1f
    fun cancelSave(){taps.offer(ScreenTap(.5f,.5f,true))}
    private val taps=java.util.concurrent.ConcurrentLinkedQueue<ScreenTap>()
    private var touchingGame=false
    private var downX=0f
    private var downY=0f
    // Compose sends touches on its buttons to those buttons, never this underlying View.
    // Only complete single-pointer gestures that began on the game can become clicks.
    override fun onTouchEvent(event:MotionEvent):Boolean {
        if((!experimentalClicks && !naming && !NativeBridge.gbaSaveScreen()) || gamePaused || !appActive){touchingGame=false;return false}
        when(event.actionMasked){
            MotionEvent.ACTION_DOWN->{touchingGame=true;downX=event.x;downY=event.y}
            MotionEvent.ACTION_POINTER_DOWN,MotionEvent.ACTION_CANCEL->touchingGame=false
            MotionEvent.ACTION_UP->{
                if(touchingGame && kotlin.math.abs(event.x-downX)<48*resources.displayMetrics.density &&
                    kotlin.math.abs(event.y-downY)<48*resources.displayMetrics.density){
                    val view=frames.displayedView
                    val point=TouchSelection.map(event.x,event.y,width,height,view)
                    if((naming || point!=null) && taps.size<8)taps.offer(ScreenTap(event.x/width,event.y/height,
                        event.buttonState and MotionEvent.BUTTON_SECONDARY!=0))
                }
                touchingGame=false
            }
        }
        return true
    }
    @Volatile var performance = "Starting..."
    @Volatile private var running = false
    private var worker: Thread? = null
    private val frames = FrameRenderer(onError)

    init {
        setEGLContextClientVersion(2)
        preserveEGLContextOnPause = true
        setRenderer(frames)
        renderMode = RENDERMODE_WHEN_DIRTY
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        running = true
        worker = Thread(::runGame, "FFTA-game").apply { start() }
    }

    override fun onDetachedFromWindow() {
        running = false
        buttons.set(0)
        worker?.interrupt()
        worker?.join(2000)
        worker = null
        super.onDetachedFromWindow()
    }

    private fun runGame() {
        Process.setThreadPriority(Process.THREAD_PRIORITY_DISPLAY)
        val benchmark=GameBenchmark.create(context)
        NativeBridge.gbaNormalAudioReader(true)
        val normalOutput=NormalAudioOutput { message -> post { onError(message) } }
        // Apply only to simulation, after its independent audio thread exists.
        if(GameBenchmark.preferFastCpu(context))NativeBridge.gbaPreferPerformanceCpu()
        val sampleBuffer = ShortArray(4096)
        // Small bounded queue absorbs AudioTrack partial writes without accumulating latency.
        val queued = ShortArray(16384)
        var queuedCount = 0
        var back: ByteBuffer? = null
        var uiBack=UiBuffers(4)
        val track = try {
            val minimum = AudioTrack.getMinBufferSize(48000, AudioFormat.CHANNEL_OUT_STEREO,
                AudioFormat.ENCODING_PCM_16BIT).coerceAtLeast(4096)
            AudioTrack.Builder()
                .setAudioAttributes(AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_GAME)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC).build())
                .setAudioFormat(AudioFormat.Builder().setSampleRate(48000)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_STEREO)
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT).build())
                .setBufferSizeInBytes(minimum * 2).build()
        } catch (error: Exception) { post { onError("Audio initialization: ${error.message}") }; null }
        var audible = false
        var appliedVolume=Float.NaN
        var lastAudioKey=-1
        var deadline = System.nanoTime()
        var count = 0
        var sampleStart = deadline
        var simulationNanos = 0L
        var wasPaused = false
        try {
            while (running) {
                if (gamePaused || !appActive) {
                    taps.clear()
                    normalOutput.active=false
                    if (!wasPaused) {
                        NativeBridge.gbaSave()
                        buttons.set(0)
                        track?.pause(); track?.flush()
                        queuedCount = 0
                        audible = false
                        wasPaused = true
                    }
                    LockSupport.parkNanos(10_000_000)
                    deadline = System.nanoTime()
                    sampleStart = deadline; count = 0; simulationNanos = 0
                    continue
                }
                wasPaused = false
                saveSelectorActive=NativeBridge.gbaSaveScreen()
                val request=viewRequest
                val frontendCanvas=NativeBridge.gbaMenuBackground()
                val config = benchmark?.view ?: if(frontendCanvas) ViewConfig.menu(frames.screenWidth,frames.screenHeight)
                    else ViewConfig.create(request.aspect,request.zoom,frames.screenWidth,frames.screenHeight)
                val bytes = config.width * config.height * 4
                if (back == null || back.capacity() != bytes) {
                    back = ByteBuffer.allocateDirect(bytes).order(ByteOrder.nativeOrder())
                }
                uiBack.resize(bytes)
                val audioCheck=benchmark!=null && benchmark.frame>=benchmark.frames && benchmark.audioFrames>0
                val multiplier = if(audioCheck)4f else if(benchmark!=null)1f else speed.coerceIn(1f,4f)
                val mode=if(audioCheck)FastAudio.NORMAL else audioMode
                val sound = soundEnabled && (multiplier==1f || mode!=FastAudio.SILENT) && track != null
                val independentNormal=sound && multiplier>1f && mode==FastAudio.NORMAL
                normalOutput.volume=volume
                normalOutput.active=independentNormal
                val audioKey=if(!sound)0 else if(multiplier==1f)1 else 10+mode.ordinal*10+(multiplier*2).toInt()
                if(audioKey!=lastAudioKey) {
                    track?.pause();track?.flush();queuedCount = 0;audible=false;lastAudioKey=audioKey
                }
                if(audioCheck)benchmark?.beginAudio()
                val start = System.nanoTime()
                if(!experimentalClicks && !naming && !NativeBridge.gbaSaveScreen())taps.clear()
                while(true){val tap=taps.poll()?:break;NativeBridge.gbaTap(tap.x,tap.y,tap.back);deliveredClicks.incrementAndGet()}
                val base=if(frontendCanvas)config else ViewConfig.create(request.aspect,0,frames.screenWidth,frames.screenHeight)
                val samples = NativeBridge.gbaFrame(benchmark?.keys() ?: (inputSource.get().inv() and 0x3ff),
                    config.extraX, config.extraY, multiplier, sound, if(mode==FastAudio.NORMAL)2 else 1, back, sampleBuffer,
                    experimentalClicks,base.width*160f/base.height,bubbleSize,bubbleLength,
                    frames.screenWidth,frames.screenHeight,config.fillScreen,uiBack.pixels,uiBack.params,uiBack.background)
                naming=NativeBridge.gbaNameEntry()
                val frameNanos=System.nanoTime()-start
                simulationNanos += frameNanos
                if (samples < 0) throw IllegalStateException(NativeBridge.gbaStatus())
                val recycled=frames.publish(back, config,uiBack)
                back=recycled.first
                uiBack=recycled.second?:UiBuffers(bytes)
                requestRender()
                val publicationEnd=if(benchmark!=null)System.nanoTime() else 0L
                if (sound && !independentNormal) {
                    if (!audible) { track?.play(); audible = true }
                    val desiredVolume=volume.coerceIn(0f,1f)
                    if(desiredVolume!=appliedVolume){track?.setVolume(desiredVolume);appliedVolume=desiredVolume}
                    if (queuedCount + samples > queued.size) queuedCount = 0
                    sampleBuffer.copyInto(queued, queuedCount, 0, samples)
                    queuedCount += samples
                    val written = track?.write(queued, 0, queuedCount, AudioTrack.WRITE_NON_BLOCKING) ?: 0
                    if (written > 0) {
                        queued.copyInto(queued, 0, written, queuedCount)
                        queuedCount -= written
                    }
                } else if (audible) {
                    track?.pause(); track?.flush(); queuedCount = 0; audible = false
                }
                val publishEnd=if(benchmark!=null)System.nanoTime() else 0L
                if(audioCheck)benchmark?.recordAudio(normalOutput)
                else benchmark?.record(frameNanos,track?.underrunCount ?: 0)
                if(benchmark!=null && benchmark.frame>=benchmark.frames && (benchmark.audioFrames==0 || benchmark.audioDone)){benchmark.recordLoop(publicationEnd-start-frameNanos,publishEnd-publicationEnd,System.nanoTime()-publishEnd,0,0,System.nanoTime()-start);gamePaused=true;continue}
                val measureEnd=if(benchmark!=null)System.nanoTime() else 0L
                var saveTime=0L
                var diagnosticsTime=0L
                count++
                if (count >= 120) {
                    val elapsed = System.nanoTime() - sampleStart
                    performance = "Game loop: %.1f FPS\nSimulation: %.2f ms/frame\nSpeed: %.1fx\nAudio underruns: %d\n".format(
                        count * 1e9 / elapsed, simulationNanos / 1e6 / count / multiplier,
                        multiplier, track?.underrunCount ?: 0)
                    val saveStart=if(benchmark!=null)System.nanoTime() else 0L
                    NativeBridge.gbaSave()
                    val saveEnd=if(benchmark!=null)System.nanoTime() else 0L
                    saveTime=saveEnd-saveStart
                    if(BuildConfig.DEBUG && benchmark!=null) {
                        java.io.File(context.cacheDir,"performance.txt").writeText(NativeBridge.gbaDiagnostics()+performance)
                    }
                    if(benchmark!=null)diagnosticsTime=System.nanoTime()-saveEnd
                    count = 0; simulationNanos = 0; sampleStart = System.nanoTime()
                }
                benchmark?.recordLoop(publicationEnd-start-frameNanos,publishEnd-publicationEnd,measureEnd-publishEnd,saveTime,diagnosticsTime,System.nanoTime()-start)
                // Actual GBA cadence: 280896 cycles / 16777216 Hz = 59.7275 FPS.
                if(benchmark!=null && !benchmark.paced)continue
                deadline += 16_742_706L
                val now = System.nanoTime()
                if (now - deadline > 4 * 16_742_706L) deadline = now
                while (running && System.nanoTime() < deadline) {
                    LockSupport.parkNanos((deadline-System.nanoTime()).coerceAtMost(2_000_000L))
                }
            }
        } catch (error: Throwable) {
            gamePaused = true
            post { onError(error.stackTraceToString()) }
        } finally {
            normalOutput.close()
            NativeBridge.gbaNormalAudioReader(false)
            try { NativeBridge.gbaSave() } catch (_: Throwable) {}
            try { track?.pause(); track?.flush(); track?.release() } catch (_: Exception) {}
        }
    }
}

private class UiBuffers(bytes:Int){
    val pixels:ByteBuffer=ByteBuffer.allocateDirect(480*160*4*11).order(ByteOrder.nativeOrder())
    val params=FloatArray(99)
    var background:ByteBuffer=ByteBuffer.allocateDirect(bytes).order(ByteOrder.nativeOrder())
        private set
    fun resize(bytes:Int){if(background.capacity()!=bytes)background=ByteBuffer.allocateDirect(bytes).order(ByteOrder.nativeOrder())}
}

private class FrameRenderer(private val onError: (String) -> Unit) : GLSurfaceView.Renderer {
    @Volatile var videoOptions=VideoOptions()
    private val videoPass=VideoPass()
    @Volatile var displayedView=ViewConfig(0,0)
        private set
    private data class PublishedFrame(val pixels:ByteBuffer,val view:ViewConfig,val ui:UiBuffers)
    private val mailbox=FrameMailbox<PublishedFrame>()
    private var uploadedView:ViewConfig?=null
    private val uploadedUiParams=FloatArray(99)
    @Volatile var screenWidth = 2280
        private set
    @Volatile var screenHeight = 1080
        private set
    private var textureWidth = 0
    private var textureHeight = 0
    private var texture = 0
    private var program = 0
    private var position = 0
    private var coordinates = 0
    private var uiProgram=0
    private var uiPosition=0
    private var uiCoordinates=0
    private val uiTextures=IntArray(12)
    private var bgWidth=0
    private var bgHeight=0
    private val vertices = floats(floatArrayOf(-1f,-1f, 1f,-1f, -1f,1f, 1f,1f))
    private val uv = floats(floatArrayOf(0f,1f, 1f,1f, 0f,0f, 1f,0f))

    fun publish(buffer:ByteBuffer,view:ViewConfig,ui:UiBuffers):Pair<ByteBuffer?,UiBuffers?> {
        val old=mailbox.publish(PublishedFrame(buffer,view,ui))
        return old?.pixels to old?.ui
    }

    override fun onSurfaceCreated(gl: GL10?, egl: EGLConfig?) {
        try {
            videoPass.reset()
            val vertex = shader(GL_VERTEX_SHADER,
                "attribute vec2 position; attribute vec2 uv; varying vec2 tex; void main(){ gl_Position=vec4(position,0.0,1.0);tex=uv; }")
            val fragment = shader(GL_FRAGMENT_SHADER,
                "precision mediump float; varying vec2 tex; uniform sampler2D image; void main(){ gl_FragColor=texture2D(image,tex).bgra; }")
            program = glCreateProgram()
            glAttachShader(program,vertex); glAttachShader(program,fragment); glLinkProgram(program)
            val linked = IntArray(1); glGetProgramiv(program,GL_LINK_STATUS,linked,0)
            check(linked[0] != 0) { glGetProgramInfoLog(program) }
            glDeleteShader(vertex); glDeleteShader(fragment)
            position = glGetAttribLocation(program,"position")
            coordinates = glGetAttribLocation(program,"uv")
            val textures = IntArray(1); glGenTextures(1,textures,0); texture = textures[0]
            glBindTexture(GL_TEXTURE_2D,texture)
            glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_MIN_FILTER,GL_NEAREST)
            glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_MAG_FILTER,GL_NEAREST)
            glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_WRAP_S,GL_CLAMP_TO_EDGE)
            glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_WRAP_T,GL_CLAMP_TO_EDGE)
            textureWidth = 0; textureHeight = 0
        uploadedView=null
            uiProgram=glCreateProgram()
            glAttachShader(uiProgram,shader(GL_VERTEX_SHADER,"""
                attribute vec2 position; attribute vec2 uv; varying vec2 tex;
                uniform vec4 rect; uniform vec2 screen;
                void main(){vec2 p=vec2(position.x*.5+.5,.5-position.y*.5)*rect.zw+rect.xy;
                  gl_Position=vec4(p.x/screen.x*2.-1.,1.-p.y/screen.y*2.,0.,1.);tex=uv;}
            """.trimIndent()))
            glAttachShader(uiProgram,shader(GL_FRAGMENT_SHADER,"""
                precision highp float;
                varying vec2 tex; uniform sampler2D panel; uniform sampler2D world;
                uniform vec2 screen; uniform vec4 worldRect; uniform vec4 clip; uniform float rawPreview;
                void main(){vec2 p=vec2(gl_FragCoord.x,screen.y-gl_FragCoord.y);
                  if(p.x<clip.x||p.y<clip.y||p.x>=clip.x+clip.z||p.y>=clip.y+clip.w)discard;
                  if(rawPreview>1.5){gl_FragColor=texture2D(panel,tex).bgra;return;}
                  vec4 data=floor(texture2D(panel,tex)*255.+.5);
                  if(data.b==0.&&data.a==16.)discard;
                  float code=data.r+data.g*256.;
                  vec3 fg=vec3(mod(code,32.),mod(floor(code/32.),32.),mod(floor(code/1024.),32.));
                  vec3 bg=floor(texture2D(world,(p-worldRect.xy)/worldRect.zw).bgra.rgb*31.+.5);
                  vec3 lane=vec3(1.,2.,1.);
                  vec3 color=floor(min(floor((fg*lane*data.b+bg*lane*data.a+8.)/16.),vec3(31.,63.,31.))/lane);
                  gl_FragColor=vec4(color/31.,1.);
                }
            """.trimIndent()))
            glLinkProgram(uiProgram)
            glGetProgramiv(uiProgram,GL_LINK_STATUS,linked,0)
            check(linked[0]!=0){glGetProgramInfoLog(uiProgram)}
            uiPosition=glGetAttribLocation(uiProgram,"position");uiCoordinates=glGetAttribLocation(uiProgram,"uv")
            glGenTextures(12,uiTextures,0)
            for(i in 0..11){
                glBindTexture(GL_TEXTURE_2D,uiTextures[i])
                glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_MIN_FILTER,GL_NEAREST)
                glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_MAG_FILTER,GL_NEAREST)
                glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_WRAP_S,GL_CLAMP_TO_EDGE)
                glTexParameteri(GL_TEXTURE_2D,GL_TEXTURE_WRAP_T,GL_CLAMP_TO_EDGE)
                if(i<11)glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA,480,160,0,GL_RGBA,GL_UNSIGNED_BYTE,null)
            }
            bgWidth=0;bgHeight=0
        } catch (error: Throwable) { onError("OpenGL: ${error.message}") }
    }

    override fun onSurfaceChanged(gl: GL10?, width: Int, height: Int) {
        screenWidth = width; screenHeight = height
    }

    override fun onDrawFrame(gl: GL10?) {
        videoPass.begin(screenWidth,screenHeight,videoOptions)
        glViewport(0,0,screenWidth,screenHeight)
        glClearColor(0.025f,0.035f,0.03f,1f); glClear(GL_COLOR_BUFFER_BIT)
        if (program == 0) return
        val pending=mailbox.acquire()
        if(pending!=null)try {
            val buffer=pending.pixels
            val config=pending.view
            buffer.position(0)
            glActiveTexture(GL_TEXTURE0); glBindTexture(GL_TEXTURE_2D,texture)
            if (textureWidth != config.width || textureHeight != config.height) {
                glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA,config.width,config.height,0,GL_RGBA,GL_UNSIGNED_BYTE,buffer)
                textureWidth = config.width; textureHeight = config.height
            } else {
                glTexSubImage2D(GL_TEXTURE_2D,0,0,0,config.width,config.height,GL_RGBA,GL_UNSIGNED_BYTE,buffer)
            }
            pending.ui.let{ui->
                ui.params.copyInto(uploadedUiParams)
                val uiParams=uploadedUiParams
                for(i in 0..10)if(uiParams[i*9+2]>0f){
                    ui.pixels.position(i*480*160*4)
                    glBindTexture(GL_TEXTURE_2D,uiTextures[i])
                    glTexSubImage2D(GL_TEXTURE_2D,0,0,0,480,160,GL_RGBA,GL_UNSIGNED_BYTE,ui.pixels)
                }
                if(uiParams[17]>0f){
                    ui.background.position(0)
                    glBindTexture(GL_TEXTURE_2D,uiTextures[11])
                    if(bgWidth!=config.width||bgHeight!=config.height){
                        glTexImage2D(GL_TEXTURE_2D,0,GL_RGBA,config.width,config.height,0,GL_RGBA,GL_UNSIGNED_BYTE,ui.background)
                        bgWidth=config.width;bgHeight=config.height
                    }else glTexSubImage2D(GL_TEXTURE_2D,0,0,0,config.width,config.height,GL_RGBA,GL_UNSIGNED_BYTE,ui.background)
                }
            }
            uploadedView=config
        } finally {
            mailbox.release(pending)
        }
        val view=uploadedView ?: return
        val uiParams=uploadedUiParams
        val scale = if(view.fillScreen)maxOf(screenWidth.toFloat()/view.width, screenHeight.toFloat()/view.height)
            else minOf(screenWidth.toFloat()/view.width, screenHeight.toFloat()/view.height)
        val width = (view.width*scale).toInt(); val height=(view.height*scale).toInt()
        glViewport((screenWidth-width)/2,(screenHeight-height)/2,width,height)
        glActiveTexture(GL_TEXTURE0);glBindTexture(GL_TEXTURE_2D,texture)
        glUseProgram(program); glUniform1i(glGetUniformLocation(program,"image"),0)
        glEnableVertexAttribArray(position); glEnableVertexAttribArray(coordinates)
        vertices.position(0); uv.position(0)
        glVertexAttribPointer(position,2,GL_FLOAT,false,0,vertices)
        glVertexAttribPointer(coordinates,2,GL_FLOAT,false,0,uv)
        glDrawArrays(GL_TRIANGLE_STRIP,0,4)
        if(uiProgram!=0){
            glViewport(0,0,screenWidth,screenHeight)
            glUseProgram(uiProgram)
            glUniform2f(glGetUniformLocation(uiProgram,"screen"),screenWidth.toFloat(),screenHeight.toFloat())
            glUniform4f(glGetUniformLocation(uiProgram,"worldRect"),((screenWidth-width)/2).toFloat(),
                ((screenHeight-height)/2).toFloat(),width.toFloat(),height.toFloat())
            glUniform1i(glGetUniformLocation(uiProgram,"world"),0)
            glUniform1i(glGetUniformLocation(uiProgram,"panel"),1)
            glEnableVertexAttribArray(uiPosition);glEnableVertexAttribArray(uiCoordinates)
            vertices.position(0);uv.position(0)
            glVertexAttribPointer(uiPosition,2,GL_FLOAT,false,0,vertices)
            glVertexAttribPointer(uiCoordinates,2,GL_FLOAT,false,0,uv)
            for(i in 0..10)if(uiParams[i*9+2]>0f){
                glActiveTexture(GL_TEXTURE0);glBindTexture(GL_TEXTURE_2D,if(uiParams[i*9+8]==1f)uiTextures[11]else texture)
                glActiveTexture(GL_TEXTURE1);glBindTexture(GL_TEXTURE_2D,uiTextures[i])
                glUniform1f(glGetUniformLocation(uiProgram,"rawPreview"),uiParams[i*9+8])
                glUniform4fv(glGetUniformLocation(uiProgram,"rect"),1,uiParams,i*9)
                glUniform4fv(glGetUniformLocation(uiProgram,"clip"),1,uiParams,i*9+4)
                glDrawArrays(GL_TRIANGLE_STRIP,0,4)
            }
            glActiveTexture(GL_TEXTURE0)
        }
        videoPass.finish(screenWidth,screenHeight,view.width,view.height)
        displayedView=view
    }
    private fun shader(type: Int, source: String): Int {
        val shader = glCreateShader(type); glShaderSource(shader,source); glCompileShader(shader)
        val status=IntArray(1);glGetShaderiv(shader,GL_COMPILE_STATUS,status,0)
        check(status[0] != 0) { glGetShaderInfoLog(shader) }
        return shader
    }
    private fun floats(values: FloatArray): FloatBuffer = ByteBuffer.allocateDirect(values.size*4)
        .order(ByteOrder.nativeOrder()).asFloatBuffer().apply { put(values);position(0) }
}
