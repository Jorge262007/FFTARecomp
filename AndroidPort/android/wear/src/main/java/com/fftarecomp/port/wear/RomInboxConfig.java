package com.fftarecomp.port.wear;

import java.io.File;
import java.io.FileInputStream;
import java.io.IOException;
import java.io.InputStream;
import java.nio.charset.StandardCharsets;
import java.util.Locale;

/** Rules for the Wi-Fi ROM transfer. The exact SHA-1/SHA-256 check happens in NativeBridge.load. */
final class RomInboxConfig {
    private RomInboxConfig() {}

    static final String GAME = "Final Fantasy Tactics Advance";
    static final int ACCENT = 0xFFEDC261;
    static final String ACCEPT = ".gba,.bin,application/octet-stream";
    /** Largest GBA cartridge is 32 MB. */
    static final long MAX_UPLOAD = 32L << 20;

    static String hint(String lang) {
        return "pt".equals(lang)
                ? "Envie a ROM .gba sem modificações do seu cartucho de Final Fantasy Tactics Advance (EUA ou Europa). Arquivos .zip não funcionam: descompacte antes."
                : "Send the unmodified .gba ROM dumped from your Final Fantasy Tactics Advance cartridge (USA or Europe). .zip files do not work: extract it first.";
    }

    /** Name saved in the drop folder, or null to refuse. */
    static String destName(String uploadName, long size) {
        if (size < 0xC0 || size > MAX_UPLOAD) return null;
        String n = uploadName == null ? "" : uploadName.toLowerCase(Locale.ROOT);
        if (n.endsWith(".zip") || n.endsWith(".7z") || n.endsWith(".rar")) return null;
        return "wifi-upload.gba";
    }

    /** null if the header is FFTA USA (AFXE) or Europe (AFXP), otherwise a Page text key. */
    static String validate(File f, String uploadName) {
        String code = gameCode(f);
        return "AFXE".equals(code) || "AFXP".equals(code) ? null : "notRom";
    }

    static String gameCode(File f) {
        byte[] header = new byte[0xB0];
        try (InputStream in = new FileInputStream(f)) {
            int off = 0;
            while (off < header.length) {
                int n = in.read(header, off, header.length - off);
                if (n < 0) return null;
                off += n;
            }
        } catch (IOException e) {
            return null;
        }
        return new String(header, 0xAC, 4, StandardCharsets.US_ASCII);
    }
}
