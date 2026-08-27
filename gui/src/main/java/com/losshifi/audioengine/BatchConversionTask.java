package com.losshifi.audioengine;

import javafx.application.Platform;
import javafx.concurrent.Task;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.function.Consumer;

public final class BatchConversionTask extends Task<Void> {
    private final List<BatchItem> items;
    private final AudioEngineService service;
    private final ConversionSettings settings;
    private final Path outputDir;
    private final Consumer<String> logSink;

    public BatchConversionTask(
            List<BatchItem> items,
            AudioEngineService service,
            ConversionSettings settings,
            Path outputDir,
            Consumer<String> logSink) {
        this.items = items;
        this.service = service;
        this.settings = settings;
        this.outputDir = outputDir;
        this.logSink = logSink;
    }

    @Override
    protected Void call() throws Exception {
        int total = items.size();
        int done = 0;
        for (BatchItem item : items) {
            if (isCancelled()) {
                Platform.runLater(() -> item.setStatus("已取消"));
                updateMessage("任务已取消");
                throw new InterruptedException("cancelled");
            }

            Platform.runLater(() -> item.setStatus("处理中"));
            updateProgress(Math.max(0.01, (double) done / total), 1.0);

            try {
                Path input = item.getInput();
                String output = deriveOutput(input, outputDir, settings);
                Files.deleteIfExists(Path.of(output));
                AudioInfo info = item.getInfo() != null
                        ? item.getInfo()
                        : service.readInfo(input.toString());
                item.setInfo(info);

                long expectedBytes = estimateBytes(info, settings);
                AtomicBoolean stop = new AtomicBoolean(false);
                Thread monitor = new Thread(() -> monitorOutput(output, expectedBytes, item, stop));
                monitor.setDaemon(true);
                monitor.start();

                Platform.runLater(() -> item.setProgress(0.02));
                updateMessage("开始转换: " + item.getFileName());
                logSink.accept("开始转换: " + item.getFileName());

                service.convert(input.toString(), output, settings);

                if (Thread.interrupted()) {
                    throw new InterruptedException("cancelled after native call");
                }

                stop.set(true);
                monitor.interrupt();
                Platform.runLater(() -> {
                    item.setProgress(1.0);
                    item.setStatus("完成");
                    item.appendLog("输出: " + output);
                });
                logSink.accept("完成: " + item.getFileName() + " -> " + output);
            } catch (IOException | RuntimeException ex) {
                Platform.runLater(() -> {
                    item.setStatus("失败");
                    item.appendLog(ex.getMessage() == null ? ex.toString() : ex.getMessage());
                });
                logSink.accept("失败: " + item.getFileName() + " - "
                        + (ex.getMessage() == null ? ex.toString() : ex.getMessage()));
            }

            done++;
            updateProgress((double) done / total, 1.0);
        }
        updateMessage("全部完成");
        return null;
    }

    private void monitorOutput(String output, long expectedBytes, BatchItem item, AtomicBoolean stop) {
        while (!stop.get() && !Thread.currentThread().isInterrupted()) {
            try {
                Path outputPath = Path.of(output);
                long size = Files.exists(outputPath) ? Files.size(outputPath) : 0;
                double progress = expectedBytes > 0
                        ? Math.min(0.02 + 0.95 * size / (double) expectedBytes, 0.97)
                        : 0.02;
                Platform.runLater(() -> item.setProgress(progress));
            } catch (IOException ignored) {
                // The output file may not exist until the encoder starts.
            }
            try {
                Thread.sleep(100);
            } catch (InterruptedException ex) {
                Thread.currentThread().interrupt();
                return;
            }
        }
    }

    private static String deriveOutput(Path input, Path outputDir, ConversionSettings settings) {
        String name = input.getFileName().toString();
        int dot = name.lastIndexOf('.');
        String base = dot > 0 ? name.substring(0, dot) : name;
        String ext;
        if (settings.getMode() == ConversionSettings.OutputMode.PCM) {
            ext = settings.getPcmFormat() == ConversionSettings.PcmFormat.WAV ? "wav" : "flac";
        } else {
            ext = settings.getDsdFormat() == ConversionSettings.DsdFormat.DSF ? "dsf" : "dff";
        }
        return outputDir.resolve(base + "." + ext).toString();
    }

    private static long estimateBytes(AudioInfo info, ConversionSettings settings) {
        double duration = info.getDuration();
        double bytesPerSecond;
        if (settings.getMode() == ConversionSettings.OutputMode.PCM) {
            double bytesPerSample = settings.getBitDepth() / 8.0;
            bytesPerSecond = settings.getPcmRate() * info.getChannels() * bytesPerSample;
            if (settings.getPcmFormat() == ConversionSettings.PcmFormat.FLAC) {
                bytesPerSecond *= 0.75;
            }
        } else {
            double dsdRate = settings.getDsdMode().getRate()
                    * (info.is44100Family() ? 44100 : 48000);
            bytesPerSecond = dsdRate * info.getChannels() / 8.0;
        }
        return Math.max(4096L, (long) (duration * bytesPerSecond) + 4096L);
    }
}
