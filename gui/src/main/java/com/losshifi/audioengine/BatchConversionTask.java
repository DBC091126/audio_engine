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

            AtomicBoolean stop = null;
            Thread monitor = null;
            try {
                Path input = item.getInput();
                AudioInfo info = item.getInfo() != null
                        ? item.getInfo()
                        : service.readInfo(input.toString());
                item.setInfo(info);

                ConversionSettings effectiveSettings = settings;
                if (settings.getMode() == ConversionSettings.OutputMode.PCM) {
                    int resolvedRate = compatiblePcmRate(info, settings);
                    if (resolvedRate != settings.getPcmRate()) {
                        effectiveSettings = settings.copyWithPcmRate(resolvedRate);
                        logSink.accept("目标采样率调整为 " + resolvedRate
                                + " Hz（" + familyLabel(info) + "）");
                    }
                }
                String output = deriveOutput(input, outputDir, effectiveSettings);
                Files.deleteIfExists(Path.of(output));

                long expectedBytes = estimateBytes(info, effectiveSettings);
                AtomicBoolean threadStop = new AtomicBoolean(false);
                stop = threadStop;
                int itemIndex = done;
                Thread localMonitor = new Thread(() ->
                        monitorOutput(output, expectedBytes, item, itemIndex, total, threadStop));
                monitor = localMonitor;
                monitor.setDaemon(true);
                monitor.start();

                Platform.runLater(() -> item.setProgress(0.02));
                updateMessage("开始转换: " + item.getFileName());
                logSink.accept("开始转换: " + item.getFileName());

                service.convert(input.toString(), output, effectiveSettings);

                if (Thread.interrupted()) {
                    throw new InterruptedException("cancelled after native call");
                }

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
            } finally {
                if (stop != null) {
                    stop.set(true);
                }
                if (monitor != null) {
                    monitor.interrupt();
                }
            }

            done++;
            updateProgress((double) done / total, 1.0);
        }
        updateMessage("全部完成");
        return null;
    }

    private void monitorOutput(
            String output,
            long expectedBytes,
            BatchItem item,
            int itemIndex,
            int total,
            AtomicBoolean stop) {
        boolean statusSet = false;
        while (!stop.get() && !Thread.currentThread().isInterrupted()) {
            try {
                Path outputPath = Path.of(output);
                long size = Files.exists(outputPath) ? Files.size(outputPath) : 0;
                double progress = expectedBytes > 0
                        ? Math.min(0.02 + 0.95 * size / (double) expectedBytes, 0.97)
                        : 0.02;
                Platform.runLater(() -> item.setProgress(progress));
                updateProgress((itemIndex + progress) / Math.max(1, total), 1.0);
                if (size > 0 && !statusSet) {
                    statusSet = true;
                    Platform.runLater(() -> item.setStatus("写入中"));
                }
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

    private static int compatiblePcmRate(AudioInfo info, ConversionSettings settings) {
        int requestedRate = settings.getPcmRate();
        boolean flac = settings.getPcmFormat() == ConversionSettings.PcmFormat.FLAC;
        if (info.getSampleRate() <= 0) {
            return flac ? Math.min(requestedRate, 96000) : requestedRate;
        }
        boolean source441 = info.is44100Family();
        boolean source480 = info.is48000Family();
        boolean requested441 = requestedRate % 44100 == 0;
        boolean requested480 = requestedRate % 48000 == 0;
        boolean familyMatches = (source441 && requested441) || (source480 && requested480);
        if (familyMatches && (!flac || requestedRate <= 96000)) {
            return requestedRate;
        }
        if (source441) {
            return flac ? 88200 : 176400;
        }
        if (source480) {
            return flac ? 96000 : 192000;
        }
        return flac ? Math.min(requestedRate, 96000) : requestedRate;
    }

    private static String familyLabel(AudioInfo info) {
        if (info.is44100Family()) {
            return "44.1k 家族";
        }
        if (info.is48000Family()) {
            return "48k 家族";
        }
        return info.getSampleRate() + " Hz";
    }
}
