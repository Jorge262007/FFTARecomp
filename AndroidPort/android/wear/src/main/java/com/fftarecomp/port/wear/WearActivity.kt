package com.fftarecomp.port.wear

import android.app.Activity
import android.content.Context
import android.graphics.Color
import android.graphics.Typeface
import android.os.Bundle
import android.view.Choreographer
import android.view.Gravity
import android.view.InputDevice
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.ViewConfiguration
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.view.InputDeviceCompat
import androidx.core.view.ViewConfigurationCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import com.fftarecomp.port.AspectMode
import com.fftarecomp.port.GameSurface
import com.fftarecomp.port.NativeBridge
import com.fftarecomp.port.VideoOptions
import com.fftarecomp.port.ViewRequest
import java.io.File
import java.util.concurrent.atomic.AtomicInteger
import kotlin.math.roundToInt

/**
 * Wear OS entry point: imports the player's ROM (Wi-Fi/QR, since Wear OS has no file picker),
 * verifies it and recompiles it on the watch with the same native core as the phone port
 * (NativeBridge.load + NativeBridge.prepare -> TinyCC -> ARM64), then runs it in a round layout.
 */
class WearActivity : Activity(), WatchInput.Actions, Choreographer.FrameCallback {
    private val keys = AtomicInteger(0)
    private val input = WatchInput(keys).also { it.actions = this }
    private lateinit var root: FrameLayout
    private var surface: GameSurface? = null
    private var controls: WatchControlsView? = null
    private var inbox: RomInbox? = null
    private var preparing: Thread? = null
    private var screen = 0
    private var crownAccum = 0f
    private val prefs by lazy { getSharedPreferences("wear", Context.MODE_PRIVATE) }
    private val rom by lazy { File(filesDir, "ffta.gba") }
    private val save by lazy { File(filesDir, "ffta.sav") }
    private val drop by lazy { File(filesDir, "inbox") }

    override fun onCreate(state: Bundle?) {
        super.onCreate(state)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        WindowCompat.setDecorFitsSystemWindows(window, false)
        WindowInsetsControllerCompat(window, window.decorView).apply {
            hide(WindowInsetsCompat.Type.systemBars())
            systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
        root = FrameLayout(this).apply { setBackgroundColor(Color.BLACK) }
        setContentView(root)
        screen = prefs.getInt("screen", 0).coerceIn(0, PRESETS.size - 1)
        input.iso = prefs.getBoolean("iso", false)
        start()
    }

    private fun start() {
        NativeBridge.loadError?.let { showMessage("Native core unavailable", it); return }
        if (NativeBridge.compiled) { showGame(); return }
        if (!rom.isFile) importBundled()
        if (rom.isFile) prepare() else showTransfer(null)
    }

    /** Debug builds may carry a ROM from wear/dev-rom/; release builds never do. */
    private fun importBundled() {
        val name = runCatching { assets.list("devrom")?.firstOrNull { it.endsWith(".gba", true) } }.getOrNull() ?: return
        runCatching { assets.open("devrom/$name").use { src -> rom.outputStream().use { src.copyTo(it) } } }
            .onFailure { rom.delete() }
    }

    // ------------------------------------------------------------------ ROM transfer

    private fun showTransfer(error: String?) {
        stopInbox()
        val view = RomTransferView(this)
        root.removeAllViews(); root.addView(view)
        error?.let { view.setStatus(it, 0xFFFF7366.toInt()) }
        drop.mkdirs()
        inbox = RomInbox(this, drop, object : RomInbox.Listener {
            override fun onServerReady(url: String, qr: Array<BooleanArray>) = view.setQrCode(url, qr)
            override fun onProgress(percent: Int) = view.setProgress(percent)
            override fun onError(msg: String) = view.setStatus(msg, 0xFFFF7366.toInt())
            override fun onFileReceived(file: File) {
                view.setStatus(RomTransferView.okText(), 0xFF66F28C.toInt())
                view.postDelayed({
                    stopInbox()
                    file.copyTo(rom, overwrite = true)
                    drop.listFiles()?.forEach { it.delete() }
                    prepare()
                }, 600)
            }
        }).also { it.start() }
    }

    private fun stopInbox() { inbox?.stop(); inbox = null }

    // ------------------------------------------------------------------ verify + recompile

    private fun prepare() {
        val status = showLoading()
        val thread = Thread({
            try {
                if (!NativeBridge.romVerified) NativeBridge.load(rom, save)
                if (!NativeBridge.compiled) NativeBridge.prepare()
                runOnUiThread { showGame() }
            } catch (failure: Throwable) {
                val message = failure.message ?: failure.toString()
                runOnUiThread {
                    if (!NativeBridge.romVerified) { rom.delete(); showTransfer(message) }
                    else showMessage("Recompilation failed", message + "\n\n" + NativeBridge.report())
                }
            }
        }, "FFTA-prepare")
        preparing = thread
        thread.start()
        val poll = object : Runnable {
            override fun run() {
                if (preparing !== thread || !thread.isAlive) return
                status.text = NativeBridge.gbaStatus()
                status.postDelayed(this, 250)
            }
        }
        status.post(poll)
    }

    private fun column(): LinearLayout {
        val pad = (resources.displayMetrics.widthPixels * 0.14f).roundToInt()
        return LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            gravity = Gravity.CENTER
            setPadding(pad, pad, pad, pad)
        }
    }

    private fun label(text: String, size: Float, color: Int, bold: Boolean = false) = TextView(this).apply {
        this.text = text; textSize = size; setTextColor(color); gravity = Gravity.CENTER
        if (bold) typeface = Typeface.create(Typeface.SERIF, Typeface.BOLD)
    }

    private fun showLoading(): TextView {
        val col = column()
        col.addView(label("FFTARecomp", 18f, GOLD, true))
        col.addView(label("Recompiling for this watch", 12f, Color.WHITE))
        val spinner = (resources.displayMetrics.widthPixels * 0.12f).roundToInt()
        col.addView(Spinner(this), LinearLayout.LayoutParams(spinner, spinner).apply { topMargin = spinner / 3; bottomMargin = spinner / 3 })
        val status = label("", 10f, Color.argb(170, 255, 255, 255))
        col.addView(status)
        root.removeAllViews(); root.addView(col)
        return status
    }

    private fun showMessage(title: String, body: String) {
        val col = column()
        col.addView(label(title, 15f, GOLD, true))
        col.addView(label(body.take(600), 10f, Color.WHITE))
        root.removeAllViews()
        root.addView(android.widget.ScrollView(this).apply { addView(col) })
    }

    // ------------------------------------------------------------------ game

    private fun showGame() {
        root.removeAllViews()
        val game = GameSurface(this) { error -> runOnUiThread { showMessage("Game stopped", error) } }
        game.inputSource = keys
        game.viewRequest = ViewRequest(AspectMode.GBA, 0)
        // "Sharp" scaling keeps pixels even at the watch's fractional scale.
        game.videoOptions = VideoOptions(scaling = 2)
        val overlay = WatchControlsView(this, input)
        root.addView(game, FrameLayout.LayoutParams(1, 1, Gravity.CENTER))
        root.addView(overlay, FrameLayout.LayoutParams(FrameLayout.LayoutParams.MATCH_PARENT, FrameLayout.LayoutParams.MATCH_PARENT))
        surface = game; controls = overlay
        overlay.addOnLayoutChangeListener { _, _, _, _, _, _, _, _, _ -> layoutPicture() }
        overlay.isFocusable = true
        overlay.requestFocus()
        Choreographer.getInstance().postFrameCallback(this)
    }

    /** Picture = inscribed 3:2 rectangle of the round glass, times the screen preset zoom. */
    private fun layoutPicture() {
        val game = surface ?: return
        val overlay = controls ?: return
        if (overlay.width == 0) return
        val (hw, hh) = overlay.imageHalf()
        val zoom = PRESETS[screen].second
        val lp = game.layoutParams as FrameLayout.LayoutParams
        val w = (2 * hw * zoom).roundToInt(); val h = (2 * hh * zoom).roundToInt()
        if (lp.width != w || lp.height != h) { lp.width = w; lp.height = h; game.layoutParams = lp }
    }

    override fun doFrame(frameTimeNanos: Long) {
        val overlay = controls ?: return
        input.frame()
        overlay.invalidate()
        Choreographer.getInstance().postFrameCallback(this)
    }

    override fun toggleFast() {
        val game = surface ?: return
        val on = game.speed <= 1f
        game.speed = if (on) FAST_SPEED else 1f
        controls?.fast = on
        controls?.toast(if (on) "Speed 3x" else "Speed 1x")
    }

    override fun cycleScreen() {
        screen = (screen + 1) % PRESETS.size
        prefs.edit().putInt("screen", screen).apply()
        layoutPicture()
        controls?.toast(PRESETS[screen].first)
    }

    override fun toggleIso() {
        input.iso = !input.iso
        prefs.edit().putBoolean("iso", input.iso).apply()
        controls?.toast(if (input.iso) "Diagonal drag on" else "Diagonal drag off")
    }

    // ------------------------------------------------------------------ hardware input

    override fun onGenericMotionEvent(event: MotionEvent): Boolean {
        if (controls != null && event.action == MotionEvent.ACTION_SCROLL &&
            event.isFromSource(InputDeviceCompat.SOURCE_ROTARY_ENCODER)) {
            val px = -event.getAxisValue(MotionEvent.AXIS_SCROLL) *
                ViewConfigurationCompat.getScaledVerticalScrollFactor(ViewConfiguration.get(this), this)
            crownAccum += px
            val step = resources.displayMetrics.density * 24f
            val detents = (crownAccum / step).toInt()
            if (detents != 0) { input.crown(detents); crownAccum -= detents * step }
            return true
        }
        if (controls != null && event.isFromSource(InputDevice.SOURCE_JOYSTICK) && event.action == MotionEvent.ACTION_MOVE) {
            val x = event.getAxisValue(MotionEvent.AXIS_X); val y = event.getAxisValue(MotionEvent.AXIS_Y)
            val hx = event.getAxisValue(MotionEvent.AXIS_HAT_X); val hy = event.getAxisValue(MotionEvent.AXIS_HAT_Y)
            var dirs = 0
            if (x < -.5f || hx < -.5f) dirs = dirs or Keys.LEFT
            if (x > .5f || hx > .5f) dirs = dirs or Keys.RIGHT
            if (y < -.5f || hy < -.5f) dirs = dirs or Keys.UP
            if (y > .5f || hy > .5f) dirs = dirs or Keys.DOWN
            input.hardware = (input.hardware and DIRS.inv()) or dirs
            return true
        }
        return super.onGenericMotionEvent(event)
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        if (controls == null) return super.dispatchKeyEvent(event)
        val down = event.action == KeyEvent.ACTION_DOWN
        if (event.keyCode == KeyEvent.KEYCODE_BACK) {
            // Watch back button = B (cancel / close menu); swipe-to-dismiss is disabled.
            if (down && event.repeatCount == 0) input.pulse(Keys.B)
            return true
        }
        val bit = when (event.keyCode) {
            KeyEvent.KEYCODE_BUTTON_A, KeyEvent.KEYCODE_Z -> Keys.A
            KeyEvent.KEYCODE_BUTTON_B, KeyEvent.KEYCODE_X -> Keys.B
            KeyEvent.KEYCODE_BUTTON_SELECT, KeyEvent.KEYCODE_TAB -> Keys.SELECT
            KeyEvent.KEYCODE_BUTTON_START, KeyEvent.KEYCODE_ENTER -> Keys.START
            KeyEvent.KEYCODE_DPAD_RIGHT -> Keys.RIGHT
            KeyEvent.KEYCODE_DPAD_LEFT -> Keys.LEFT
            KeyEvent.KEYCODE_DPAD_UP -> Keys.UP
            KeyEvent.KEYCODE_DPAD_DOWN -> Keys.DOWN
            KeyEvent.KEYCODE_BUTTON_R1, KeyEvent.KEYCODE_BUTTON_R2, KeyEvent.KEYCODE_S -> Keys.R
            KeyEvent.KEYCODE_BUTTON_L1, KeyEvent.KEYCODE_BUTTON_L2, KeyEvent.KEYCODE_A -> Keys.L
            else -> 0
        }
        if (bit != 0) {
            input.hardware = if (down) input.hardware or bit else input.hardware and bit.inv()
            return true
        }
        if (event.keyCode == KeyEvent.KEYCODE_BUTTON_MODE && down && event.repeatCount == 0) { toggleFast(); return true }
        return super.dispatchKeyEvent(event)
    }

    // ------------------------------------------------------------------ lifecycle

    override fun onResume() {
        super.onResume()
        surface?.let { it.onResume(); it.appActive = true }
        if (controls != null) Choreographer.getInstance().postFrameCallback(this)
    }

    override fun onPause() {
        // GameSurface saves (NativeBridge.gbaSave) as soon as appActive goes false.
        input.clear()
        surface?.let { it.appActive = false; it.onPause() }
        Choreographer.getInstance().removeFrameCallback(this)
        super.onPause()
    }

    override fun onDestroy() {
        stopInbox()
        Choreographer.getInstance().removeFrameCallback(this)
        super.onDestroy()
    }

    /** Rotating gold arc; the platform Wear ProgressBar does not draw in this theme. */
    private class Spinner(context: Context) : View(context) {
        private val paint = android.graphics.Paint(android.graphics.Paint.ANTI_ALIAS_FLAG).apply {
            style = android.graphics.Paint.Style.STROKE; strokeCap = android.graphics.Paint.Cap.ROUND; color = GOLD
        }
        private val box = android.graphics.RectF()
        override fun onDraw(canvas: android.graphics.Canvas) {
            paint.strokeWidth = width * 0.1f
            val inset = paint.strokeWidth
            box.set(inset, inset, width - inset, height - inset)
            val angle = (android.os.SystemClock.uptimeMillis() % 1000L) * 0.36f
            canvas.drawArc(box, angle, 100f, false, paint)
            postInvalidateOnAnimation()
        }
    }

    companion object {
        private val GOLD = Color.rgb(237, 194, 97)
        private const val FAST_SPEED = 3f
        private const val DIRS = Keys.LEFT or Keys.RIGHT or Keys.UP or Keys.DOWN
        private val PRESETS = listOf("Whole screen" to 1f, "Zoom" to 1.15f, "Strong zoom" to 1.3f)
    }
}
