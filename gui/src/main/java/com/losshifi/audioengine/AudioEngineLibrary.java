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

    int get_file_info_ex(
            String path,
            IntByReference sampleRate,
            ShortByReference channels,
            ShortByReference bits,
            DoubleByReference duration,
            Pointer buffer,
            long bufferSize);

    int get_file_metadata(String path, Pointer buffer, long bufferSize);

    int get_ate_response_curve(
            String inputPath,
            byte ateEnable,
            byte ateStyle,
            float ateIntensity,
            Pointer buffer,
            long bufferSize);

    static AudioEngineLibrary load() {
        String configured = System.getProperty("audio_engine.library.path");
        if (configured != null && !configured.isBlank()) {
            System.setProperty("jna.library.path", configured);
            System.setProperty("java.library.path", configured);
        } else {
            Path nativeDir = nativeDirFromProtectionDomain();
            if (nativeDir != null) {
                System.setProperty("jna.library.path", nativeDir.toString());
                System.setProperty("java.library.path", nativeDir.toString());
            } else {
                File direct = findDevelopmentNativeDir();
                if (direct != null) {
                    System.setProperty("jna.library.path", direct.getAbsolutePath());
                    System.setProperty("java.library.path", direct.getAbsolutePath());
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
            if (dir != null) {
                for (int depth = 0; depth < 4; depth++) {
                    Path nativeDir = dir.resolve("native");
                    if (Files.isDirectory(nativeDir) && hasNativeLibrary(nativeDir)) {
                        return nativeDir;
                    }
                    dir = dir.getParent();
                    if (dir == null) {
                        break;
                    }
                }
            }
        } catch (Exception ignored) {
            // Fall back to development directory probing below.
        }
        return null;
    }

    private static File findDevelopmentNativeDir() {
        String userDir = System.getProperty("user.dir", ".");
        String[] candidates = {
                "target/release",
                "../target/release",
                "audio_engine/target/release",
                "audio_engine/gui/target/audio-engine-gui-0.1.0-dist/native",
        };
        for (String candidate : candidates) {
            File dir = new File(userDir, candidate);
            if (dir.isDirectory() && hasNativeLibrary(dir.toPath())) {
                return dir;
            }
        }
        return null;
    }

    private static boolean hasNativeLibrary(Path nativeDir) {
        return Files.isRegularFile(nativeDir.resolve(nativeLibraryName()));
    }

    private static String nativeLibraryName() {
        String os = System.getProperty("os.name", "").toLowerCase();
        if (os.contains("win")) {
            return "audio_engine.dll";
        }
        if (os.contains("mac")) {
            return "libaudio_engine.dylib";
        }
        return "libaudio_engine.so";
    }
}
