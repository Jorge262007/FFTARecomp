package com.fftarecomp.port.wear;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.RectF;
import android.graphics.Typeface;
import android.os.Build;
import android.provider.Settings;
import android.view.MotionEvent;
import android.view.View;
import android.view.animation.LinearInterpolator;

import java.util.Locale;

/**
 * Full-screen ROM transfer screen sized to the watch face: no scrolling, everything is
 * laid out relative to the display radius so it fits any round or square watch.
 * Top: game name. Middle: QR code (or upload percentage). Bottom: IP:PORT + status.
 * The rim shows a spinner while waiting for Wi-Fi and the upload progress afterwards.
 */
public class RomTransferView extends View {

    private final boolean pt = Locale.getDefault().getLanguage().equals("pt");
    private final int accent = RomInboxConfig.ACCENT;

    private final Paint bg = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint qrPaint = new Paint();
    private final Paint card = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint title = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint urlPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint statusPaint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint ring = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint ringTrack = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint big = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF rect = new RectF();

    private boolean[][] qr;
    private String url;
    private String status;
    private int statusColor = 0xFFB4B4B4;
    private int progress = -1;
    private float spin;
    private final ValueAnimator spinner;
    private final boolean animate;

    public RomTransferView(Context context) {
        super(context);
        setKeepScreenOn(true);
        animate = !reducedMotion(context);
        bg.setColor(Color.BLACK);
        qrPaint.setColor(Color.BLACK);
        qrPaint.setAntiAlias(false);
        card.setColor(Color.WHITE);
        title.setColor(accent);
        title.setTypeface(Typeface.create(Typeface.DEFAULT, Typeface.BOLD));
        title.setTextAlign(Paint.Align.CENTER);
        title.setLetterSpacing(0.04f);
        urlPaint.setColor(Color.WHITE);
        urlPaint.setTypeface(Typeface.create(Typeface.MONOSPACE, Typeface.BOLD));
        urlPaint.setTextAlign(Paint.Align.CENTER);
        statusPaint.setTextAlign(Paint.Align.CENTER);
        big.setColor(Color.WHITE);
        big.setTypeface(Typeface.create(Typeface.DEFAULT, Typeface.BOLD));
        big.setTextAlign(Paint.Align.CENTER);
        ring.setStyle(Paint.Style.STROKE);
        ring.setStrokeCap(Paint.Cap.ROUND);
        ring.setColor(accent);
        ringTrack.setStyle(Paint.Style.STROKE);
        ringTrack.setColor(0xFF262626);
        status = pt ? "Conectando ao Wi-Fi..." : "Connecting to Wi-Fi...";

        spinner = ValueAnimator.ofFloat(0f, 360f);
        spinner.setDuration(1400);
        spinner.setRepeatCount(ValueAnimator.INFINITE);
        spinner.setInterpolator(new LinearInterpolator());
        spinner.addUpdateListener(a -> {
            spin = (float) a.getAnimatedValue();
            if (qr == null && progress < 0) invalidate();
        });
    }

    private static boolean reducedMotion(Context c) {
        try {
            return Settings.Global.getFloat(c.getContentResolver(), Settings.Global.ANIMATOR_DURATION_SCALE, 1f) == 0f;
        } catch (Exception e) {
            return false;
        }
    }

    @Override
    protected void onAttachedToWindow() {
        super.onAttachedToWindow();
        if (animate) spinner.start();
    }

    @Override
    protected void onDetachedFromWindow() {
        spinner.cancel();
        super.onDetachedFromWindow();
    }

    /** Swallow touches so the game/overlay underneath never reacts while this screen is up. */
    @Override
    public boolean onTouchEvent(MotionEvent e) {
        return true;
    }

    private static boolean isPt() { return Locale.getDefault().getLanguage().equals("pt"); }

    /** Shown once the file is on the watch, while the game starts. */
    public static String okText() { return isPt() ? "ROM recebida! Abrindo o jogo..." : "ROM received! Starting game..."; }

    /** Shown if the received file could not be imported. */
    public static String failText() { return isPt() ? "Arquivo recusado. Envie outra ROM." : "File refused. Send another ROM."; }

    public void setQrCode(String url, boolean[][] qr) {
        this.url = url;
        this.qr = qr;
        if (progress < 0) {
            status = pt ? "Escaneie ou digite no navegador" : "Scan or type it in a browser";
            statusColor = 0xFFB4B4B4;
        }
        invalidate();
    }

    public void setProgress(int percent) {
        progress = Math.max(0, Math.min(100, percent));
        status = (pt ? "Recebendo ROM " : "Receiving ROM ") + progress + "%";
        statusColor = 0xFFB4B4B4;
        invalidate();
    }

    public void setStatus(String text, int color) {
        status = text;
        statusColor = color;
        // Error (reddish colour): drop the percentage and show the QR again so the user can retry.
        if (Color.red(color) > Color.green(color)) progress = -1;
        invalidate();
    }

    @Override
    protected void onDraw(Canvas c) {
        int w = getWidth(), h = getHeight();
        if (w == 0 || h == 0) return;
        c.drawRect(0, 0, w, h, bg);
        float cx = w / 2f, cy = h / 2f;
        float r = Math.min(w, h) / 2f;
        boolean round = Build.VERSION.SDK_INT >= 23 && getResources().getConfiguration().isScreenRound();

        // Rim: spinner while waiting for Wi-Fi, progress while uploading.
        float stroke = Math.max(3f, r * 0.03f);
        ring.setStrokeWidth(stroke);
        ringTrack.setStrokeWidth(stroke);
        float rr = r - stroke * 1.2f;
        rect.set(cx - rr, cy - rr, cx + rr, cy + rr);
        if (progress >= 0) {
            c.drawArc(rect, 0, 360, false, ringTrack);
            c.drawArc(rect, -90, progress * 3.6f, false, ring);
        } else if (qr == null) {
            c.drawArc(rect, spin - 90, 70, false, ring);
        }

        // Title (top band).
        fitText(title, RomInboxConfig.GAME.toUpperCase(Locale.ROOT), chord(r, 0.67f, round) * 0.88f, r * 0.085f);
        c.drawText(RomInboxConfig.GAME.toUpperCase(Locale.ROOT), cx, cy - r * 0.585f, title);

        // Middle: QR on a white card (quiet zone), or the big percentage while uploading.
        float side = r * 0.98f;
        // QR card exactly on the centre of the face; text bands sit above and below it.
        float top = cy - side / 2f;
        if (progress >= 0) {
            big.setTextSize(r * 0.42f);
            c.drawText(progress + "%", cx, top + side / 2f + r * 0.15f, big);
        } else if (qr != null) {
            float pad = side * 0.06f;
            rect.set(cx - side / 2f, top, cx + side / 2f, top + side);
            c.drawRoundRect(rect, side * 0.06f, side * 0.06f, card);
            int n = qr.length;
            float mod = (float) Math.floor((side - 2 * pad) / n);
            if (mod < 1f) mod = (side - 2 * pad) / n;
            float q = mod * n;
            float x0 = Math.round(cx - q / 2f), y0 = Math.round(top + side / 2f - q / 2f);
            for (int y = 0; y < n; y++) {
                boolean[] row = qr[y];
                for (int x = 0; x < n; x++) {
                    if (row[x]) c.drawRect(x0 + x * mod, y0 + y * mod, x0 + (x + 1) * mod, y0 + (y + 1) * mod, qrPaint);
                }
            }
        } else {
            statusPaint.setColor(0xFF707070);
            statusPaint.setTextSize(r * 0.07f);
            c.drawText("Wi-Fi", cx, cy, statusPaint);
        }

        // Address the user can type (no "http://": browsers accept it, and it reads bigger).
        float urlY = cy + r * 0.635f;
        if (url != null) {
            String shown = url.replace("http://", "");
            fitText(urlPaint, shown, chord(r, 0.66f, round) * 0.9f, r * 0.11f);
            c.drawText(shown, cx, urlY, urlPaint);
        }

        if (status != null) {
            statusPaint.setColor(statusColor);
            float sy = cy + r * 0.785f;
            fitText(statusPaint, status, chord(r, 0.82f, round) * 0.9f, r * 0.066f);
            c.drawText(status, cx, sy, statusPaint);
        }
    }

    /** Width available at vertical offset k*r from the centre (full width on square screens). */
    private static float chord(float r, float k, boolean round) {
        if (!round) return 2f * r;
        return 2f * r * (float) Math.sqrt(Math.max(0.0, 1.0 - k * k));
    }

    private static void fitText(Paint p, String s, float maxW, float maxSize) {
        p.setTextSize(maxSize);
        float tw = p.measureText(s);
        if (tw > maxW && tw > 0) p.setTextSize(Math.max(maxSize * 0.45f, maxSize * maxW / tw));
    }
}
