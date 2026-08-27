package com.losshifi.audioengine;

import javafx.beans.property.DoubleProperty;
import javafx.beans.property.SimpleDoubleProperty;
import javafx.beans.property.SimpleStringProperty;
import javafx.beans.property.StringProperty;

import java.nio.file.Path;

public final class BatchItem {
    private final Path input;
    private final StringProperty status = new SimpleStringProperty("待处理");
    private final DoubleProperty progress = new SimpleDoubleProperty(0);
    private final StringProperty log = new SimpleStringProperty("");
    private AudioInfo info;

    public BatchItem(Path input) {
        this.input = input;
    }

    public Path getInput() {
        return input;
    }

    public StringProperty statusProperty() {
        return status;
    }

    public String getStatus() {
        return status.get();
    }

    public void setStatus(String value) {
        status.set(value);
    }

    public DoubleProperty progressProperty() {
        return progress;
    }

    public double getProgress() {
        return progress.get();
    }

    public void setProgress(double value) {
        progress.set(value);
    }

    public StringProperty logProperty() {
        return log;
    }

    public String getLog() {
        return log.get();
    }

    public void appendLog(String text) {
        log.set(log.get().isEmpty() ? text : log.get() + "\n" + text);
    }

    public AudioInfo getInfo() {
        return info;
    }

    public void setInfo(AudioInfo info) {
        this.info = info;
    }

    public String getFileName() {
        return input.getFileName().toString();
    }
}
