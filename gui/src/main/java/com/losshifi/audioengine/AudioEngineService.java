package com.losshifi.audioengine;

import com.sun.jna.Memory;
import com.sun.jna.ptr.DoubleByReference;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.ShortByReference;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.util.LinkedHashMap;
import java.util.Map;

public final class AudioEngineService {
    private final AudioEngineLibrary library;

    public AudioEngineService() {
        this.library = AudioEngineLibrary.load();
    }

    public AudioInfo readInfo(String path) throws IOException {
        IntByReference sampleRate = new IntByReference();
        ShortByReference channels = new ShortByReference();
        ShortByReference bits = new ShortByReference();
        DoubleByReference duration = new DoubleByReference();
        Memory buffer = new Memory(65536);

        int rc = library.get_file_info_ex(
                path,
                sampleRate,
                channels,
                bits,
                duration,
                buffer,
                65536);
        if (rc != 0) {
            throw new IOException("get_file_info_ex failed with code " + rc);
        }
        Map<String, String> metadata = parseMetadata(buffer.getString(0, StandardCharsets.UTF_8.name()));

        return new AudioInfo(
                sampleRate.getValue(),
                channels.getValue(),
                bits.getValue(),
                duration.getValue(),
                metadata);
    }

    public int convert(String input, String output, ConversionSettings settings) throws IOException {
        int rc = library.process_file(
                input,
                output,
                settings.getPcmRate(),
                (short) settings.getBitDepth(),
                (byte) settings.outputFormatCode(),
                settings.dsdModeCode(),
                (byte) (settings.isAteEnabled() ? 1 : 0),
                (byte) settings.getAteStyle().getCode(),
                (float) settings.getAteIntensity());
        if (rc != 0) {
            throw new IOException("process_file failed with code " + rc);
        }
        return rc;
    }

    public ResponseCurve readAteResponseCurve(String input, ConversionSettings settings)
            throws IOException {
        Memory buffer = new Memory(1 << 20);
        int rc = library.get_ate_response_curve(
                input,
                (byte) (settings.isAteEnabled() ? 1 : 0),
                (byte) settings.getAteStyle().getCode(),
                (float) settings.getAteIntensity(),
                buffer,
                buffer.size());
        if (rc != 0) {
            throw new IOException("get_ate_response_curve failed with code " + rc);
        }
        return ResponseCurve.parse(buffer.getString(0, StandardCharsets.UTF_8.name()));
    }

    private static Map<String, String> parseMetadata(String text) {
        Map<String, String> metadata = new LinkedHashMap<>();
        if (text == null || text.isBlank()) {
            return metadata;
        }
        for (String line : text.split("\\n")) {
            if (line.isBlank()) {
                continue;
            }
            int tab = line.indexOf('\t');
            if (tab > 0) {
                metadata.put(line.substring(0, tab), line.substring(tab + 1));
            }
        }
        return metadata;
    }
}
