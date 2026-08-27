package com.losshifi.audioengine;

import com.sun.jna.Library;
import com.sun.jna.Native;
import com.sun.jna.Pointer;
import com.sun.jna.ptr.DoubleByReference;
import com.sun.jna.ptr.IntByReference;
import com.sun.jna.ptr.ShortByReference;

import java.io.File;
import java.net.URI;
import java.nio.file.Files;
import java.nio.file.Path;

public interface AudioEngineLibrary extends Library {
    int process_file(
            String inputPath,
            String outputPath,
            int targetRate,
            short bitDepth,
            byte outputFormat,
            short dsdMode,
            byte ateEnable,
            byte ateStyle,
            float ateIntensity);

    int get_file_info(
            String path,
            IntByReference sampleRate,
            ShortByReference channels,
            ShortByReference bits,
            DoubleByReference duration);

    int get_file_metadata(String path, Pointer buffer, long bufferSize);

    static AudioEngineLibrary load() {
        String configured = System.getProperty("audio_engine.library.path");
        if (configured != null && !configured.isBlank()) {
            System.setProperty("jna.library.path", configured);
        } else {
            Path nativeDir = nativeDirFromProtectionDomain();
            if (nativeDir != null) {
                System.setProperty("jna.library.path", nativeDir.toString());
            } else {
                File direct = new File("target/release");
                if (!direct.isDirectory()) {
                    direct = new File("../target/release");
                }
                if (!direct.isDirectory()) {
                    direct = new File("audio_engine/target/release");
                }
                if (direct.isDirectory()) {
                    System.setProperty("jna.library.path", direct.getAbsolutePath());
                }
            }
        }
        return Native.load("audio_engine", AudioEngineLibrary.class);
    }

    private static Path nativeDirFromProtectionDomain() {
        try {
            var location = AudioEngineLibrary.class
                    .getProtectionDomain()
                    .getCodeSource()
                    .getLocation();
            if (location == null) {
                return null;
            }
            URI uri = location.toURI();
            Path codeSource = Path.of(uri);
            Path dir = Files.isDirectory(codeSource) ? codeSource : codeSource.getParent();
            Path nativeDir = dir == null ? null : dir.resolve("native");
            if (nativeDir != null && Files.isDirectory(nativeDir)) {
                return nativeDir;
            }
        } catch (Exception ignored) {
            // Fall back to development directory probing below.
        }
        return null;
    }
}
