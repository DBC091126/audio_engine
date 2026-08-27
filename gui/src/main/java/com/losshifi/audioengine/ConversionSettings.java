package com.losshifi.audioengine;

public final class ConversionSettings {
    public enum OutputMode {
        PCM, DSD
    }

    public enum PcmFormat {
        WAV(0), FLAC(1);
        private final int code;

        PcmFormat(int code) {
            this.code = code;
        }

        public int getCode() {
            return code;
        }
    }

    public enum DsdFormat {
        DSF(2), DFF(3);
        private final int code;

        DsdFormat(int code) {
            this.code = code;
        }

        public int getCode() {
            return code;
        }
    }

    public enum DsdMode {
        DSD64(64), DSD128(128), DSD256(256);
        private final int rate;

        DsdMode(int rate) {
            this.rate = rate;
        }

        public int getRate() {
            return rate;
        }
    }

    public enum AteStyle {
        TUBE(0), VINYL(1), HYBRID(2);
        private final int code;

        AteStyle(int code) {
            this.code = code;
        }

        public int getCode() {
            return code;
        }
    }

    private OutputMode mode = OutputMode.PCM;
    private int pcmRate = 176400;
    private int bitDepth = 24;
    private PcmFormat pcmFormat = PcmFormat.WAV;
    private DsdMode dsdMode = DsdMode.DSD256;
    private DsdFormat dsdFormat = DsdFormat.DSF;
    private boolean ateEnabled = false;
    private AteStyle ateStyle = AteStyle.TUBE;
    private double ateIntensity = 0.5;

    public OutputMode getMode() {
        return mode;
    }

    public void setMode(OutputMode mode) {
        this.mode = mode;
    }

    public int getPcmRate() {
        return pcmRate;
    }

    public void setPcmRate(int pcmRate) {
        this.pcmRate = pcmRate;
    }

    public int getBitDepth() {
        return bitDepth;
    }

    public void setBitDepth(int bitDepth) {
        this.bitDepth = bitDepth;
    }

    public PcmFormat getPcmFormat() {
        return pcmFormat;
    }

    public void setPcmFormat(PcmFormat pcmFormat) {
        this.pcmFormat = pcmFormat;
    }

    public DsdMode getDsdMode() {
        return dsdMode;
    }

    public void setDsdMode(DsdMode dsdMode) {
        this.dsdMode = dsdMode;
    }

    public DsdFormat getDsdFormat() {
        return dsdFormat;
    }

    public void setDsdFormat(DsdFormat dsdFormat) {
        this.dsdFormat = dsdFormat;
    }

    public boolean isAteEnabled() {
        return ateEnabled;
    }

    public void setAteEnabled(boolean ateEnabled) {
        this.ateEnabled = ateEnabled;
    }

    public AteStyle getAteStyle() {
        return ateStyle;
    }

    public void setAteStyle(AteStyle ateStyle) {
        this.ateStyle = ateStyle;
    }

    public double getAteIntensity() {
        return ateIntensity;
    }

    public void setAteIntensity(double ateIntensity) {
        this.ateIntensity = ateIntensity;
    }

    public int outputFormatCode() {
        return mode == OutputMode.PCM ? pcmFormat.getCode() : dsdFormat.getCode();
    }

    public short dsdModeCode() {
        return (short) dsdMode.getRate();
    }
}
