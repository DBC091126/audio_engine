package com.losshifi.audioengine;

import java.util.LinkedHashMap;
import java.util.Map;

public final class AudioInfo {
    private final int sampleRate;
    private final short channels;
    private final short bits;
    private final double duration;
    private final Map<String, String> metadata;

    public AudioInfo(int sampleRate, short channels, short bits, double duration,
                     Map<String, String> metadata) {
        this.sampleRate = sampleRate;
        this.channels = channels;
        this.bits = bits;
        this.duration = duration;
        this.metadata = new LinkedHashMap<>(metadata);
    }

    public int getSampleRate() {
        return sampleRate;
    }

    public short getChannels() {
        return channels;
    }

    public short getBits() {
        return bits;
    }

    public double getDuration() {
        return duration;
    }

    public Map<String, String> getMetadata() {
        return metadata;
    }

    public boolean is44100Family() {
        return sampleRate > 0 && sampleRate % 44100 == 0;
    }

    public boolean is48000Family() {
        return sampleRate > 0 && sampleRate % 48000 == 0;
    }
}
