package com.fftarecomp.port.wear;

import android.app.Activity;
import android.content.Context;
import android.net.ConnectivityManager;
import android.net.LinkProperties;
import android.net.Network;
import android.net.NetworkCapabilities;
import android.net.NetworkRequest;
import android.os.Handler;
import android.os.Looper;
import android.os.StatFs;
import android.util.Log;

import com.google.zxing.BarcodeFormat;
import com.google.zxing.EncodeHintType;
import com.google.zxing.common.BitMatrix;
import com.google.zxing.qrcode.QRCodeWriter;
import com.google.zxing.qrcode.decoder.ErrorCorrectionLevel;

import java.io.BufferedInputStream;
import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.io.OutputStream;
import java.net.Inet4Address;
import java.net.InetAddress;
import java.net.InetSocketAddress;
import java.net.NetworkInterface;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.URLDecoder;
import java.util.Collections;
import java.util.EnumMap;
import java.util.Locale;
import java.util.Map;

/**
 * Wi-Fi ROM transfer: tiny HTTP server on the watch + QR code with its address.
 * The phone/PC opens http://IP:PORT (scan the QR or type it), picks the file and it
 * streams straight to disk. Game specific rules live in {@link RomInboxConfig}.
 * Shared across every Wear OS port; keep the copies identical except for the package.
 */
public class RomInbox {

    private static final String TAG = "RomInbox";
    /** Preferred port; if busy the next free one up to +10 is used, then any free port. */
    private static final int PORT = 8080;

    public interface Listener {
        /** url is "http://IP:PORT"; qr is a matrix (true = dark), never null. */
        void onServerReady(String url, boolean[][] qr);
        void onProgress(int percent);
        void onFileReceived(File file);
        void onError(String msg);
    }

    private final Activity activity;
    private final File targetDir;
    private final Listener listener;
    private final Handler mainHandler;
    private final String lang;

    private volatile ServerSocket server;
    private volatile int boundPort = -1;
    private ConnectivityManager connectivityManager;
    private ConnectivityManager.NetworkCallback wifiCallback;
    private volatile String serverIp;
    private volatile boolean running = false;
    private volatile boolean busy = false;

    public RomInbox(Activity activity, File targetDir, Listener listener) {
        this.activity = activity;
        this.targetDir = targetDir;
        this.listener = listener;
        this.mainHandler = new Handler(Looper.getMainLooper());
        this.lang = Locale.getDefault().getLanguage();
    }

    public synchronized void start() {
        if (running) return;
        running = true;
        // A transfer cut by a crash or a force-stop leaves a partial file behind: drop it.
        new File(targetDir, "upload.part").delete();
        startServer();
        requestWifi();
        // Interfaces can come up late (Wi-Fi wakes on demand on watches): poll the IP for a while.
        mainHandler.postDelayed(ipPoll, 1500);
    }

    public synchronized void stop() {
        running = false;
        mainHandler.removeCallbacks(ipPoll);
        ServerSocket s = server;
        server = null;
        if (s != null) {
            try { s.close(); } catch (IOException ignored) {}
        }
        if (wifiCallback != null && connectivityManager != null) {
            try { connectivityManager.unregisterNetworkCallback(wifiCallback); } catch (Exception ignored) {}
            wifiCallback = null;
        }
    }

    private final Runnable ipPoll = new Runnable() {
        @Override
        public void run() {
            if (!running) return;
            checkAndUpdateIp();
            mainHandler.postDelayed(this, serverIp == null ? 1500 : 5000);
        }
    };

    private void requestWifi() {
        connectivityManager = (ConnectivityManager) activity.getSystemService(Context.CONNECTIVITY_SERVICE);
        if (connectivityManager == null) return;
        NetworkRequest req = new NetworkRequest.Builder()
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .removeCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET)
                .build();
        wifiCallback = new ConnectivityManager.NetworkCallback() {
            @Override public void onAvailable(Network network) { checkAndUpdateIp(); }
            @Override public void onLinkPropertiesChanged(Network network, LinkProperties lp) { checkAndUpdateIp(); }
            @Override public void onLost(Network network) { serverIp = null; checkAndUpdateIp(); }
        };
        try {
            // Keeps Wi-Fi up on the watch (it prefers Bluetooth through the phone otherwise).
            connectivityManager.requestNetwork(req, wifiCallback);
        } catch (Exception e) {
            Log.w(TAG, "requestNetwork error", e);
        }
        checkAndUpdateIp();
    }

    private synchronized void checkAndUpdateIp() {
        String ip = getLocalIpAddress();
        int port = boundPort;
        if (ip == null || port <= 0) {
            if (ip == null && serverIp != null) {
                serverIp = null;
                post(() -> { if (listener != null) listener.onError(Page.t(lang, "noWifi")); });
            }
            return;
        }
        if (ip.equals(serverIp)) return;
        serverIp = ip;
        final String url = "http://" + ip + ":" + port;
        final boolean[][] qr = qrMatrix(url);
        post(() -> { if (listener != null && qr != null) listener.onServerReady(url, qr); });
    }

    private void post(Runnable r) { mainHandler.post(r); }

    private void startServer() {
        new Thread(() -> {
            ServerSocket s = bind();
            if (s == null) {
                post(() -> { if (listener != null) listener.onError(Page.t(lang, "noPort")); });
                return;
            }
            server = s;
            boundPort = s.getLocalPort();
            Log.i(TAG, "ROM inbox listening on port " + boundPort);
            checkAndUpdateIp();
            while (running && !s.isClosed()) {
                try {
                    final Socket client = s.accept();
                    client.setSoTimeout(60000);
                    new Thread(() -> handleClient(client), "RomInbox-Client").start();
                } catch (IOException e) {
                    // closed
                }
            }
        }, "RomInbox-Server").start();
    }

    private static ServerSocket bind() {
        for (int p = PORT; p <= PORT + 10; p++) {
            try {
                ServerSocket s = new ServerSocket();
                s.setReuseAddress(true);
                s.bind(new InetSocketAddress(p));
                return s;
            } catch (IOException ignored) {}
        }
        try {
            ServerSocket s = new ServerSocket();
            s.bind(new InetSocketAddress(0));
            return s;
        } catch (IOException e) {
            return null;
        }
    }

    private void handleClient(Socket client) {
        try (Socket c = client;
             InputStream in = new BufferedInputStream(c.getInputStream(), 65536);
             OutputStream out = c.getOutputStream()) {

            String header = readHeader(in);
            if (header == null) return;
            String[] lines = header.split("\r\n");
            String[] first = lines[0].split(" ");
            if (first.length < 2) return;
            String method = first[0];
            String path = first[1];

            long contentLength = -1;
            for (int i = 1; i < lines.length; i++) {
                int colon = lines[i].indexOf(':');
                if (colon > 0 && lines[i].substring(0, colon).trim().equalsIgnoreCase("content-length")) {
                    try { contentLength = Long.parseLong(lines[i].substring(colon + 1).trim()); }
                    catch (NumberFormatException ignored) {}
                }
            }

            if ("GET".equalsIgnoreCase(method) && (path.equals("/") || path.startsWith("/index") || path.startsWith("/?"))) {
                sendResponse(out, 200, "text/html; charset=utf-8", Page.html(lang).getBytes("UTF-8"));
            } else if ("POST".equalsIgnoreCase(method) && path.startsWith("/upload")) {
                String name = queryParam(path, "name");
                String dest = RomInboxConfig.destName(name, contentLength);
                if (busy) {
                    sendResponse(out, 409, "application/json", json(false, Page.t(lang, "busy")));
                } else if (contentLength <= 0 || contentLength > RomInboxConfig.MAX_UPLOAD || dest == null) {
                    sendResponse(out, 413, "application/json", json(false, Page.t(lang, "notRom")));
                } else if (freeBytes() < contentLength + (8L << 20)) {
                    sendResponse(out, 507, "application/json", json(false, Page.t(lang, "noSpace")));
                } else {
                    busy = true;
                    try {
                        File tmp = receive(in, contentLength);
                        if (tmp == null) {
                            sendResponse(out, 500, "application/json", json(false, Page.t(lang, "cut")));
                            post(() -> { if (listener != null) listener.onError(Page.t(lang, "cut")); });
                            return;
                        }
                        String errKey = RomInboxConfig.validate(tmp, name);
                        if (errKey != null) {
                            final String err = Page.t(lang, errKey);
                            tmp.delete();
                            sendResponse(out, 415, "application/json", json(false, err));
                            post(() -> { if (listener != null) listener.onError(err); });
                            return;
                        }
                        File f = new File(targetDir, dest);
                        f.delete();
                        if (!tmp.renameTo(f)) {
                            tmp.delete();
                            sendResponse(out, 500, "application/json", json(false, Page.t(lang, "cut")));
                            return;
                        }
                        sendResponse(out, 200, "application/json", json(true, Page.t(lang, "okGame")));
                        post(() -> { if (listener != null) listener.onFileReceived(f); });
                    } finally {
                        busy = false;
                    }
                }
            } else if ("GET".equalsIgnoreCase(method) && path.startsWith("/favicon")) {
                sendResponse(out, 204, "text/plain", new byte[0]);
            } else {
                sendResponse(out, 404, "text/plain", "Not Found".getBytes("UTF-8"));
            }
        } catch (Exception e) {
            Log.w(TAG, "client error", e);
        }
    }

    private long freeBytes() {
        try {
            targetDir.mkdirs();
            return new StatFs(targetDir.getAbsolutePath()).getAvailableBytes();
        } catch (Exception e) {
            return Long.MAX_VALUE;
        }
    }

    private File receive(InputStream in, long total) {
        targetDir.mkdirs();
        File tmp = new File(targetDir, "upload.part");
        int lastPct = -1;
        try (FileOutputStream fos = new FileOutputStream(tmp)) {
            byte[] buf = new byte[65536];
            long done = 0;
            while (done < total) {
                int n = in.read(buf, 0, (int) Math.min(buf.length, total - done));
                if (n < 0) break;
                fos.write(buf, 0, n);
                done += n;
                final int pct = (int) (done * 100 / total);
                if (pct != lastPct) {
                    lastPct = pct;
                    post(() -> { if (listener != null) listener.onProgress(pct); });
                }
            }
            if (done < total) { tmp.delete(); return null; }
            fos.getFD().sync();
        } catch (IOException e) {
            tmp.delete();
            return null;
        }
        return tmp;
    }

    private static String queryParam(String path, String key) {
        int q = path.indexOf('?');
        if (q < 0) return "";
        for (String kv : path.substring(q + 1).split("&")) {
            int eq = kv.indexOf('=');
            if (eq > 0 && kv.substring(0, eq).equals(key)) {
                try { return URLDecoder.decode(kv.substring(eq + 1), "UTF-8"); } catch (Exception e) { return ""; }
            }
        }
        return "";
    }

    private static String readHeader(InputStream in) throws IOException {
        ByteArrayOutputStream baos = new ByteArrayOutputStream();
        int state = 0;
        while (baos.size() < 16384) {
            int c = in.read();
            if (c < 0) return null;
            baos.write(c);
            if ((state == 0 || state == 2) && c == '\r') state++;
            else if ((state == 1 || state == 3) && c == '\n') state++;
            else state = (c == '\r') ? 1 : 0;
            if (state == 4) return baos.toString("ISO-8859-1");
        }
        return null;
    }

    private static void sendResponse(OutputStream out, int code, String type, byte[] body) throws IOException {
        String head = "HTTP/1.1 " + code + (code < 300 ? " OK" : " Error") + "\r\n"
                + "Content-Type: " + type + "\r\n"
                + "Content-Length: " + body.length + "\r\n"
                + "Cache-Control: no-store\r\n"
                + "Connection: close\r\n\r\n";
        out.write(head.getBytes("ISO-8859-1"));
        out.write(body);
        out.flush();
    }

    private static byte[] json(boolean ok, String msg) {
        String s = "{\"ok\":" + ok + ",\"msg\":\"" + msg.replace("\\", "\\\\").replace("\"", "\\\"") + "\"}";
        try { return s.getBytes("UTF-8"); } catch (Exception e) { return s.getBytes(); }
    }

    private static String getLocalIpAddress() {
        try {
            String fallback = null;
            for (NetworkInterface nif : Collections.list(NetworkInterface.getNetworkInterfaces())) {
                if (!nif.isUp() || nif.isLoopback()) continue;
                for (InetAddress addr : Collections.list(nif.getInetAddresses())) {
                    if (!(addr instanceof Inet4Address) || addr.isLoopbackAddress()) continue;
                    if (nif.getName().startsWith("wlan")) return addr.getHostAddress();
                    if (fallback == null && addr.isSiteLocalAddress() && !nif.getName().startsWith("rmnet")
                            && !nif.getName().startsWith("bt")) fallback = addr.getHostAddress();
                }
            }
            return fallback;
        } catch (Exception ignored) {}
        return null;
    }

    private static boolean[][] qrMatrix(String content) {
        try {
            Map<EncodeHintType, Object> hints = new EnumMap<>(EncodeHintType.class);
            hints.put(EncodeHintType.MARGIN, 0);
            hints.put(EncodeHintType.ERROR_CORRECTION, ErrorCorrectionLevel.M);
            // size 1 -> zxing returns the bare module matrix; the view scales it crisply.
            BitMatrix m = new QRCodeWriter().encode(content, BarcodeFormat.QR_CODE, 1, 1, hints);
            int w = m.getWidth(), h = m.getHeight();
            boolean[][] out = new boolean[h][w];
            for (int y = 0; y < h; y++) for (int x = 0; x < w; x++) out[y][x] = m.get(x, y);
            return out;
        } catch (Exception e) {
            Log.e(TAG, "QR generation failed", e);
            return null;
        }
    }
}
