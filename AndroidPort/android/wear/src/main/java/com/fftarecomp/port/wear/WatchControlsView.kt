package com.fftarecomp.port.wear

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.graphics.Typeface
import android.os.SystemClock
import android.view.MotionEvent
import android.view.View
import kotlin.math.abs
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.min
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * Round-watch layout: the whole GBA frame, undistorted, is the largest 3:2 rectangle that fits the
 * glass. Buttons fill the free caps above and below it and the crescents at its sides; their inner
 * edge follows the picture border, so no button ever covers a game pixel at the default preset.
 */
class WatchControlsView(context: Context, private val input: WatchInput) : View(context) {
    private class Btn(val a0: Float, val a1: Float, val bits: Int, val label: String)

    // Degrees, 0 = right, 90 = bottom. Picture corners sit near +-34 / 180+-34 degrees.
    private val buttons = listOf(
        Btn(36f, 70f, Keys.A, "A"), Btn(110f, 144f, Keys.B, "B"), Btn(76f, 104f, Keys.FAST, ">>"),
        Btn(216f, 250f, Keys.L, "L"), Btn(290f, 324f, Keys.R, "R"), Btn(256f, 284f, Keys.MODE, "M"),
        Btn(-24f, 24f, Keys.START, "START"), Btn(156f, 204f, Keys.SELECT, "SELECT"),
    )

    var fast = false
    private var toast: String? = null
    private var toastAt = 0L

    private var cx = 0f; private var cy = 0f; private var r = 1f

    private val fill = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val stroke = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val text = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
        textAlign = Paint.Align.CENTER
    }
    private val path = Path()
    private val oval = RectF()

    init { input.hit = ::hit }

    fun toast(message: String) { toast = message; toastAt = SystemClock.uptimeMillis(); invalidate() }

    /** Half size of the undistorted picture (zoom 1). */
    fun imageHalf(): Pair<Float, Float> {
        val d = sqrt(240f * 240f + 160f * 160f)
        val ri = r * IMG_RHO
        return ri * 240f / d to ri * 160f / d
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        cx = w / 2f; cy = h / 2f; r = min(w, h) / 2f
        input.setScreen(w, h)
    }

    private fun borderDist(deg: Float): Float {
        val (hw, hh) = imageHalf()
        val a = Math.toRadians(deg.toDouble())
        val c = abs(cos(a)).toFloat(); val s = abs(sin(a)).toFloat()
        val dx = if (c > 1e-6f) hw / c else 1e9f
        val dy = if (s > 1e-6f) hh / s else 1e9f
        return min(dx, dy)
    }

    private fun inner(deg: Float) = min(borderDist(deg) + BTN_GAP * r, ARC_OUT * r - 2f)

    private fun angleIn(deg: Float, a0: Float, a1: Float): Boolean {
        var d = (deg - a0) % 360f
        if (d < 0) d += 360f
        return d <= a1 - a0
    }

    /** The whole cap or crescent is the target; the picture itself stays gesture input. */
    fun hit(x: Float, y: Float): Int {
        val dx = x - cx; val dy = y - cy
        val dist = hypot(dx, dy)
        val deg = Math.toDegrees(atan2(dy, dx).toDouble()).toFloat()
        for (b in buttons) {
            if (dist >= borderDist(deg) + 0.5f * BTN_GAP * r && angleIn(deg, b.a0 - 3f, b.a1 + 3f)) return b.bits
        }
        return 0
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val i = e.actionIndex
                input.down(e.getPointerId(i), e.getX(i), e.getY(i))
            }
            MotionEvent.ACTION_MOVE -> for (i in 0 until e.pointerCount) input.move(e.getPointerId(i), e.getX(i), e.getY(i))
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> input.up(e.getPointerId(e.actionIndex), false)
            MotionEvent.ACTION_CANCEL -> for (i in 0 until e.pointerCount) input.up(e.getPointerId(i), true)
        }
        return true
    }

    private fun band(a0: Float, a1: Float) {
        path.reset()
        val ro = ARC_OUT * r
        val n = 32
        for (i in 0..n) {
            val deg = a0 + (a1 - a0) * i / n
            val a = Math.toRadians(deg.toDouble())
            val ri = inner(deg)
            val x = cx + cos(a).toFloat() * ri; val y = cy + sin(a).toFloat() * ri
            if (i == 0) path.moveTo(x, y) else path.lineTo(x, y)
        }
        oval.set(cx - ro, cy - ro, cx + ro, cy + ro)
        path.arcTo(oval, a1, a0 - a1)
        path.close()
    }

    override fun onDraw(canvas: Canvas) {
        val now = SystemClock.uptimeMillis()
        val held = input.touchHeld or input.pulseNow or input.stick.dir or (if (fast) Keys.FAST else 0) or
            (if (input.iso) Keys.MODE else 0)
        // Controls dim after a few seconds without touches so the picture stays clean.
        val k = if (now - input.lastTouch > IDLE_FADE_MS) 0.5f else 1f
        val ro = ARC_OUT * r
        for (b in buttons) {
            val on = held and b.bits != 0
            band(b.a0, b.a1)
            fill.color = if (on) ON else Color.argb((0.11f * k * 255).toInt(), 255, 255, 255)
            canvas.drawPath(path, fill)
            val mid = (b.a0 + b.a1) / 2f
            val a = Math.toRadians(mid.toDouble())
            val ri = inner(mid)
            val lr = (ri + ro) / 2f
            val thick = ro - ri
            text.color = Color.argb(((if (on) 1f else 0.75f * k) * 230).toInt(), 255, 255, 255)
            val lx = cx + cos(a).toFloat() * lr; val ly = cy + sin(a).toFloat() * lr
            if (b.label.length > 2 && thick < r * 0.2f) {
                // Narrow side crescent: stack the letters so the label stays inside it.
                text.textSize = (thick * 0.42f).coerceIn(r * 0.045f, r * 0.07f)
                val step = text.textSize * 1.02f
                val top = ly - step * (b.label.length - 1) / 2f
                b.label.forEachIndexed { i, ch -> canvas.drawText(ch.toString(), lx, top + i * step - (text.ascent() + text.descent()) / 2f, text) }
            } else {
                text.textSize = (thick * 0.42f).coerceIn(r * 0.06f, r * 0.12f)
                canvas.drawText(b.label, lx, ly - (text.ascent() + text.descent()) / 2f, text)
            }
        }
        if (input.stick.active) drawStick(canvas)
        toast?.let { message ->
            if (now - toastAt > TOAST_MS) toast = null else {
                text.textSize = r * 0.075f
                val w = text.measureText(message) + r * 0.12f
                val h = r * 0.14f
                val y = cy - r * 0.42f
                fill.color = Color.argb(210, 13, 13, 18)
                oval.set(cx - w / 2, y - h / 2, cx + w / 2, y + h / 2)
                canvas.drawRoundRect(oval, h / 2, h / 2, fill)
                text.color = Color.WHITE
                canvas.drawText(message, cx, y - (text.ascent() + text.descent()) / 2f, text)
            }
        }
    }

    private fun drawStick(canvas: Canvas) {
        val s = input.stick
        stroke.strokeWidth = 2f
        stroke.color = Color.argb(61, 255, 255, 255)
        canvas.drawCircle(s.ax, s.ay, s.radius - 1f, stroke)
        val off = if (input.iso) -45f else 0f
        for ((deg, bit) in listOf(0f to Keys.RIGHT, 90f to Keys.DOWN, 180f to Keys.LEFT, 270f to Keys.UP)) {
            val on = s.dir == bit
            stroke.color = if (on) ON else Color.argb(61, 255, 255, 255)
            stroke.strokeWidth = if (on) 9f else 4f
            val rr = s.radius + if (on) 7.5f else 5f
            oval.set(s.ax - rr, s.ay - rr, s.ax + rr, s.ay + rr)
            canvas.drawArc(oval, deg + off - 14f, 28f, false, stroke)
        }
        fill.color = Color.argb(166, 237, 194, 97)
        canvas.drawCircle(s.x, s.y, s.radius * 0.3f, fill)
    }

    companion object {
        const val IMG_RHO = 0.985f
        private const val BTN_GAP = 0.022f
        private const val ARC_OUT = 0.985f
        private const val IDLE_FADE_MS = 4000L
        private const val TOAST_MS = 1800L
        private val ON = Color.argb(191, 237, 194, 97)
    }
}
