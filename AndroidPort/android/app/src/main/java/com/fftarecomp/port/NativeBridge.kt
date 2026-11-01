package com.fftarecomp.port

import android.os.Build
import android.os.Debug
import java.io.File
import java.nio.ByteBuffer
import java.security.MessageDigest
import kotlin.math.ceil
import kotlin.math.floor
import kotlin.math.ln
import kotlin.math.pow

enum class AspectMode(val label: String, val baseExtra: Int, val baseHeight: Int=160) {
    GBA("3:2 Original", 0), WIDE("16:10", 8), WIDESCREEN("16:9", 22),
    PHONE("20:9", 58), ULTRAWIDE("21:9", 66), SUPERWIDE("32:9", 164),
    SQUARE("1:1",8,256), NOTE10("19:9",49), DEVICE("Full screen",49)
}
val FAST_SPEEDS=listOf(1.5f,2f,3f,4f)
fun speedLabel(value:Float)=if(value==value.toInt().toFloat()) "${value.toInt()}x" else "${value}x"

enum class FastAudio(val label:String) { NORMAL("Normal"), SILENT("Silent"), FAST("Fast audio") }

data class ViewRequest(val aspect:AspectMode,val zoom:Int)

data class ViewConfig(val extraX: Int, val extraY: Int,val fillScreen:Boolean=false) {
    val width get() = 240 + 2 * extraX
    val height get() = 160 + 2 * extraY
    companion object {
        private fun base(aspect:AspectMode,screenWidth:Int,screenHeight:Int):Pair<Double,Double> {
            if(aspect!=AspectMode.DEVICE)return (240+2*aspect.baseExtra).toDouble() to aspect.baseHeight.toDouble()
            val ratio=screenWidth.coerceAtLeast(1).toDouble()/screenHeight.coerceAtLeast(1)
            return if(ratio>=1.5)maxOf(242.0,160*ratio).coerceAtMost(1536.0) to 160.0
                else 256.0 to (256/ratio).coerceAtMost(800.0)
        }
        fun maxZoom(aspect: AspectMode,screenWidth:Int=2280,screenHeight:Int=1080): Int {
            val (width,height)=base(aspect,screenWidth,screenHeight)
            val limit = minOf(1536.0 / width, 800.0 / height)
            return floor(ln(limit) / ln(2.0.pow(1.0 / 6))).toInt().coerceIn(0, 6)
        }
        /** Extend only the menu canvas, retaining the native foreground scale. */
        fun menu(screenWidth:Int,screenHeight:Int):ViewConfig {
            val scale=minOf(screenWidth.coerceAtLeast(1)/240.0,screenHeight.coerceAtLeast(1)/160.0)
            return ViewConfig(ceil((screenWidth/scale-240)/2).toInt().coerceIn(0,648),
                ceil((screenHeight/scale-160)/2).toInt().coerceIn(0,320),true)
        }
        fun create(aspect: AspectMode, zoom: Int,screenWidth:Int=2280,screenHeight:Int=1080): ViewConfig {
            val (width,height)=base(aspect,screenWidth,screenHeight)
            val scale = 2.0.pow(zoom.coerceIn(0, maxZoom(aspect,screenWidth,screenHeight)) / 6.0)
            return ViewConfig(
                ceil((width * scale - 240) / 2).toInt().coerceIn(0, 648),
                ceil((height * scale - 160) / 2).toInt().coerceIn(0, 320),aspect==AspectMode.DEVICE)
        }
    }
}

object NativeBridge {
    external fun gbaBenchmarkSnapshot(load:Boolean):Int
    external fun gbaNameEntry():Boolean
    external fun gbaVideoPalette(out: java.nio.ByteBuffer, screen: Int, darken: Float, brightness: Float): Int
    external fun gbaSaveScreen(): Boolean
    external fun gbaMenuBackground(): Boolean
    val loadError: String?
    @Volatile var romVerified = false
        private set
    @Volatile var compiled = false
        private set
    private var hashes = "Not loaded"

    external fun gbaLoadRom(bytes: ByteArray, savePath: String): Int
    external fun gbaPrepare(): Int
    external fun gbaFrame(keys: Int, extraX: Int, extraY: Int, speed: Float,
                          sound: Boolean, audioMode: Int, pixels: ByteBuffer, audio: ShortArray,
                          touch:Boolean,baseWidth:Float,uiSize:Float,uiLength:Float,
                          screenWidth:Int,screenHeight:Int,fill:Boolean,ui:ByteBuffer,uiParams:FloatArray,uiBackground:ByteBuffer): Int
    external fun gbaTap(x:Float,y:Float,back:Boolean)
    external fun gbaUiProfile(enabled:Boolean,reference:Boolean)
    external fun gbaReferenceFrame(reference:Boolean)
    external fun gbaReferenceDialogue(reference:Boolean)
    external fun gbaPreferPerformanceCpu():Boolean
    external fun gbaUiTimings(out:LongArray)
    external fun gbaNormalAudioReader(enabled:Boolean)
    external fun gbaPullNormalAudio(audio:ShortArray):Int
    external fun gbaCharacterMetrics(enabled:Boolean)
    external fun gbaCharacters():String
    external fun gbaZoomWorkers(limit:Int)
    external fun gbaParallelZoom(enabled:Boolean)
    external fun gbaCamera():Long
    external fun gbaSave(): Int
    external fun gbaStatus(): String
    external fun gbaDiagnostics(): String

    init {
        var failure: String? = null
        try {
            // Keep TinyCC separately replaceable; it is dynamically loaded by Rust.
            System.loadLibrary("tcc")
            System.loadLibrary("gba_android")
        } catch (error: Throwable) { failure = error.stackTraceToString() }
        loadError = failure
    }

    fun load(file: File, save: File): Boolean {
        check(loadError == null) { loadError ?: "Native core unavailable" }
        romVerified = false
        compiled = false
        require(file.length() in 192..32L * 1024 * 1024) { "Invalid ROM size" }
        val bytes = file.readBytes()
        fun digest(algorithm: String) = MessageDigest.getInstance(algorithm).digest(bytes)
            .joinToString("") { "%02x".format(it.toInt() and 255) }
        val sha1 = digest("SHA-1")
        val sha256 = digest("SHA-256")
        hashes = "SHA1: $sha1\nSHA256: $sha256"
        val usa = sha1 == "4ac05441f4de70a4ec3dd932116346c61b8783d9" &&
            sha256 == "43fc8204c6dceee58828aebc7af0c72eb807e99f35ad641c8bb0a4fa8b6edc19"
        val europe = sha1 == "9efaf328cbbcbc830be14940e42e4b92d90dfb58" &&
            sha256 == "8fab1e390cb9d5e695bfdb86e81246892e1aa8c0cb4014db8235ff76538ea632"
        require(usa || europe) {
            "ROM mismatch. Select an unmodified FFTA USA or Europe ROM."
        }
        check(gbaLoadRom(bytes, save.absolutePath) == 1) { gbaStatus() }
        romVerified = true
        return true
    }

    fun prepare(): Boolean {
        check(romVerified) { "Load a verified ROM first" }
        compiled = gbaPrepare() == 1
        check(compiled) { gbaStatus() }
        return true
    }

    fun report(performance: String = ""): String = buildString {
        append("FFTARecomp Android preview\n")
        append("Android ${Build.VERSION.RELEASE} / SDK ${Build.VERSION.SDK_INT}\n")
        append("Device: ${Build.MANUFACTURER} ${Build.MODEL}\nABI: ${Build.SUPPORTED_ABIS.joinToString()}\n")
        append("ROM verified: $romVerified\n$hashes\n")
        append("Recompiled: $compiled\n")
        append("Native heap allocated: ${Debug.getNativeHeapAllocatedSize() / 1048576} MiB\n")
        append("Java heap allocated: ${(Runtime.getRuntime().totalMemory() - Runtime.getRuntime().freeMemory()) / 1048576} MiB\n")
        if (loadError != null) append("Library error: $loadError\n")
        else { append("Status: ${gbaStatus()}\n"); append(gbaDiagnostics()) }
        append(performance)
    }
}
