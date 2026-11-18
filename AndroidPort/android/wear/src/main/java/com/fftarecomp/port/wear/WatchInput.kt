package com.fftarecomp.port.wear

import android.os.SystemClock
import java.util.concurrent.atomic.AtomicInteger
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.hypot
import kotlin.math.min

/** GBA key bits, as GameSurface/NativeBridge expect them (active high here, inverted on send). */
object Keys {
    const val A = 1; const val B = 2; const val SELECT = 4; const val START = 8
    const val RIGHT = 16; const val LEFT = 32; const val UP = 64; const val DOWN = 128
    const val R = 256; const val L = 512
    const val GBA = 0x3ff
    /** Watch-only buttons, never sent to the game. */
    const val FAST = 1 shl 16
    const val MODE = 1 shl 17
}

/**
 * Touch gestures, rim buttons, crown and back button -> one GBA key mask per display frame.
 * Runs on the UI thread; [frame] is called once per vsync and publishes to [output], which the
 * game thread reads (GameSurface.inputSource).
 */
class WatchInput(val output: AtomicInteger) {
    interface Actions {
        fun toggleFast()
        fun cycleScreen()
        fun toggleIso()
    }
    var actions: Actions? = null
    var hit: (Float, Float) -> Int = { _, _ -> 0 }

    private class Finger(val id: Int, val x0: Float, val y0: Float, val down: Long, val bits: Int) {
        val gesture get() = bits == 0
        var moved = false
        var consumed = false
    }

    class Stick {
        var active = false
        var ax = 0f; var ay = 0f; var x = 0f; var y = 0f
        var radius = 1f
        var dir = 0
    }

    private val fingers = HashMap<Int, Finger>()
    val stick = Stick()
    private var stickId = -1
    private var shortSide = 1f

    var iso = false
    private var modeDown = 0L
    private var modeHeld = false
    private var modeHoldFired = false

    private val pulses = ArrayDeque<Int>()
    private var pulsePhase = 0
    var pulseNow = 0; private set
    private var tapLatch = 0
    private var tapFrames = 0
    private var stickLatched = 0
    private var stickMinFrames = 0
    /** Physical keys (gamepad / keyboard), merged every frame. */
    var hardware = 0
    var lastTouch = 0L; private set

    fun setScreen(w: Int, h: Int) {
        shortSide = min(w, h).toFloat()
        stick.radius = STICK_RADIUS * shortSide
    }

    fun pulse(key: Int) { if (pulses.size < MAX_PULSES) pulses.addLast(key) }

    /** One crown detent = one cursor step; a fast spin never queues a long backlog. */
    fun crown(detents: Int) {
        val key = if (detents > 0) Keys.DOWN else Keys.UP
        repeat(abs(detents)) { if (pulses.size < 4) pulses.addLast(key) }
    }

    fun clear() {
        fingers.clear(); stick.active = false; stick.dir = 0; pulses.clear()
        tapLatch = 0; tapFrames = 0; hardware = 0; modeHeld = false; output.set(0)
    }

    val touchHeld: Int get() = fingers.values.fold(0) { acc, f -> acc or f.bits }

    fun down(id: Int, x: Float, y: Float) {
        val now = SystemClock.uptimeMillis()
        lastTouch = now
        val bits = hit(x, y)
        fingers[id] = Finger(id, x, y, now, bits)
        if (bits and Keys.MODE != 0) { modeHeld = true; modeHoldFired = false; modeDown = now }
    }

    fun move(id: Int, x: Float, y: Float) {
        val f = fingers[id] ?: return
        lastTouch = SystemClock.uptimeMillis()
        if (!f.gesture || f.consumed) return
        if (!f.moved) {
            val slop = TAP_SLOP * shortSide
            if (hypot(x - f.x0, y - f.y0) < slop || stick.active) return
            f.moved = true
            stickId = id
            stick.active = true; stick.ax = f.x0; stick.ay = f.y0; stick.dir = 0
        }
        if (!stick.active || stickId != id) return
        stick.x = x; stick.y = y
        val dx = x - stick.ax; val dy = y - stick.ay
        val d = hypot(dx, dy)
        if (d > stick.radius) {
            // Trail: the anchor follows the thumb, so direction changes are instant.
            stick.ax = x - dx / d * stick.radius
            stick.ay = y - dy / d * stick.radius
        }
    }

    fun up(id: Int, canceled: Boolean) {
        val f = fingers[id] ?: return
        val now = SystemClock.uptimeMillis()
        lastTouch = now
        if (f.gesture) {
            if (stick.active && stickId == id && f.moved) { stick.active = false; stick.dir = 0 }
            val others = fingers.values.filter { it.gesture && it !== f }
            if (!canceled && !f.consumed && !f.moved) {
                val other = others.firstOrNull()
                when {
                    other == null -> if (now - f.down <= TAP_MAX_MS) pulse(Keys.A)       // tap = confirm
                    other.moved -> pulse(Keys.A)                                          // drag + tap = confirm
                    !other.consumed && now - other.down <= TWO_TAP_MS + TAP_MAX_MS -> {   // two-finger tap = cancel
                        pulse(Keys.B); other.consumed = true
                    }
                }
            }
        }
        if (f.bits and Keys.GBA != 0) { tapLatch = tapLatch or (f.bits and Keys.GBA); tapFrames = PULSE_ON }
        if (f.bits and Keys.FAST != 0 && !canceled) actions?.toggleFast()
        if (f.bits and Keys.MODE != 0 && modeHeld) {
            modeHeld = false
            if (!modeHoldFired && !canceled) actions?.cycleScreen()
        }
        fingers.remove(id)
    }

    fun frame() {
        val now = SystemClock.uptimeMillis()
        if (modeHeld && !modeHoldFired && now - modeDown > HOLD_MS) { modeHoldFired = true; actions?.toggleIso() }

        var keys = hardware or (touchHeld and Keys.GBA)
        if (tapFrames > 0) { keys = keys or tapLatch; if (--tapFrames == 0) tapLatch = 0 }

        pulseNow = 0
        if (pulses.isNotEmpty()) {
            if (pulsePhase < PULSE_ON) { pulseNow = pulses.first(); keys = keys or pulseNow }
            if (++pulsePhase >= PULSE_ON + PULSE_OFF) { pulsePhase = 0; pulses.removeFirst() }
        }

        if (stick.active) {
            var dx = (stick.x - stick.ax) / stick.radius
            var dy = (stick.y - stick.ay) / stick.radius
            if (iso) {
                // Isometric maps: up-right = UP, up-left = LEFT, down-left = DOWN, down-right = RIGHT.
                val k = 0.70710678f
                val rx = (dx + dy) * k; val ry = (dy - dx) * k
                dx = rx; dy = ry
            }
            val d = quantize(dx.coerceIn(-1.5f, 1.5f), dy.coerceIn(-1.5f, 1.5f), stick.dir)
            if (d != 0 && d != stick.dir) { stickLatched = d; stickMinFrames = 2 }
            stick.dir = d
        } else stick.dir = 0
        keys = keys or if (stick.dir != 0) stick.dir else if (stickMinFrames > 0) stickLatched else 0
        if (stickMinFrames > 0) stickMinFrames--

        if (keys and (Keys.LEFT or Keys.RIGHT) == Keys.LEFT or Keys.RIGHT) keys = keys and (Keys.LEFT or Keys.RIGHT).inv()
        if (keys and (Keys.UP or Keys.DOWN) == Keys.UP or Keys.DOWN) keys = keys and (Keys.UP or Keys.DOWN).inv()
        output.set(keys and Keys.GBA)
    }

    /** Analog vector -> one D-pad direction with hysteresis (no jitter on diagonals). */
    private fun quantize(x: Float, y: Float, prev: Int): Int {
        if (hypot(x, y) < if (prev != 0) STICK_KEEP else STICK_ON) return 0
        val deg = Math.toDegrees(atan2(y, x).toDouble()).toFloat()  // 0 = right, 90 = down
        var best = 0; var bestD = Float.MAX_VALUE
        for ((c, bit) in DIRS) {
            var d = abs(deg - c)
            if (bit == prev) d -= STICK_SWITCH_DEG
            if (d < bestD) { bestD = d; best = bit }
        }
        return best
    }

    companion object {
        private const val HOLD_MS = 600L
        private const val TAP_MAX_MS = 320L
        private const val TWO_TAP_MS = 350L
        private const val TAP_SLOP = 0.055f
        const val STICK_RADIUS = 0.15f
        private const val STICK_ON = 0.38f
        private const val STICK_KEEP = 0.26f
        private const val STICK_SWITCH_DEG = 12f
        private const val PULSE_ON = 3
        private const val PULSE_OFF = 3
        private const val MAX_PULSES = 16
        private val DIRS = listOf(0f to Keys.RIGHT, 90f to Keys.DOWN, 180f to Keys.LEFT, -90f to Keys.UP, -180f to Keys.LEFT)
    }
}
