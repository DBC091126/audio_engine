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
        TUBE(0, "胆机"),
        VINYL(1, "黑胶"),
        HYBRID(2, "混合胆味"),
        A_CLASS_SE(3, "A类石机-单端"),
        A_CLASS_PP(4, "A类石机-推挽"),
        AB_CLASS(5, "AB类石机"),
        D_CLASS(6, "高端D类"),
        VINTAGE_SOLID_STATE(7, "老式AB/B石机"),
        TUBE_PUSH_PULL(8, "胆机推挽"),
        FERRITE_TAPE(9, "铁氧体磁带"),
        PHONO_STAGE(10, "唱放"),
        POWER_TRANSFORMER_SATURATION(11, "电源牛磁饱和"),
        CATHODE_FOLLOWER(12, "阴随"),
        OPAMP_PREAMP(13, "运放前级"),
        PHONO_CARTRIDGE_RESONANCE(14, "唱头共振"),
        DAC_FILTER_ROLLOFF(15, "DAC滤波滚降");
        private final int code;
        private final String label;

        AteStyle(int code, String label) {
            this.code = code;
            this.label = label;
        }

        public int getCode() {
            return code;
        }

        public String label() {
            return label;
        }

        @Override
        public String toString() {
            return label;
        }

        public String display(String language) {
            if ("English".equals(language)) {
                return switch (this) {
                    case TUBE -> "Tube";
                    case VINYL -> "Vinyl";
                    case HYBRID -> "Hybrid Tube";
                    case A_CLASS_SE -> "Class A SE";
                    case A_CLASS_PP -> "Class A PP";
                    case AB_CLASS -> "Class AB";
                    case D_CLASS -> "Class D";
                    case VINTAGE_SOLID_STATE -> "Vintage Solid State";
                    case TUBE_PUSH_PULL -> "Tube Push-Pull";
                    case FERRITE_TAPE -> "Ferrite Tape";
                    case PHONO_STAGE -> "Phono Stage";
                    case POWER_TRANSFORMER_SATURATION -> "Power Transformer";
                    case CATHODE_FOLLOWER -> "Cathode Follower";
                    case OPAMP_PREAMP -> "Opamp Preamp";
                    case PHONO_CARTRIDGE_RESONANCE -> "Phono Cartridge";
                    case DAC_FILTER_ROLLOFF -> "DAC Filter Rolloff";
                };
            }
            if ("繁體中文".equals(language)) {
                return switch (this) {
                    case TUBE -> "膽機";
                    case VINYL -> "黑膠";
                    case HYBRID -> "混合膽味";
                    case A_CLASS_SE -> "A類石機-單端";
                    case A_CLASS_PP -> "A類石機-推挽";
                    case AB_CLASS -> "AB類石機";
                    case D_CLASS -> "高端D類";
                    case VINTAGE_SOLID_STATE -> "老式AB/B石機";
                    case TUBE_PUSH_PULL -> "膽機推挽";
                    case FERRITE_TAPE -> "鐵氧體磁帶";
                    case PHONO_STAGE -> "唱放";
                    case POWER_TRANSFORMER_SATURATION -> "電源牛磁飽和";
                    case CATHODE_FOLLOWER -> "陰極隨耦";
                    case OPAMP_PREAMP -> "運放前級";
                    case PHONO_CARTRIDGE_RESONANCE -> "唱頭共振";
                    case DAC_FILTER_ROLLOFF -> "DAC濾波滾降";
                };
            }
            return label;
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
    private double ateNoiseDb = 0;
    private double ateJitterPs = 0;
    private double atePhaseDeg = 0;
    private double ateCrossoverDepth = 0;
    private double ateEvenHarmonics = 1.0;
    private double ateOddHarmonics = 1.0;

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

    public double getAteNoiseDb() {
        return ateNoiseDb;
    }

    public void setAteNoiseDb(double ateNoiseDb) {
        this.ateNoiseDb = ateNoiseDb;
    }

    public double getAteJitterPs() {
        return ateJitterPs;
    }

    public void setAteJitterPs(double ateJitterPs) {
        this.ateJitterPs = ateJitterPs;
    }

    public double getAtePhaseDeg() {
        return atePhaseDeg;
    }

    public void setAtePhaseDeg(double atePhaseDeg) {
        this.atePhaseDeg = atePhaseDeg;
    }

    public double getAteCrossoverDepth() {
        return ateCrossoverDepth;
    }

    public void setAteCrossoverDepth(double ateCrossoverDepth) {
        this.ateCrossoverDepth = ateCrossoverDepth;
    }

    public double getAteEvenHarmonics() {
        return ateEvenHarmonics;
    }

    public void setAteEvenHarmonics(double ateEvenHarmonics) {
        this.ateEvenHarmonics = ateEvenHarmonics;
    }

    public double getAteOddHarmonics() {
        return ateOddHarmonics;
    }

    public void setAteOddHarmonics(double ateOddHarmonics) {
        this.ateOddHarmonics = ateOddHarmonics;
    }

    public int outputFormatCode() {
        return mode == OutputMode.PCM ? pcmFormat.getCode() : dsdFormat.getCode();
    }

    public short dsdModeCode() {
        return (short) dsdMode.getRate();
    }

    public ConversionSettings copyWithPcmRate(int pcmRate) {
        ConversionSettings copy = new ConversionSettings();
        copy.mode = mode;
        copy.pcmRate = pcmRate;
        copy.bitDepth = bitDepth;
        copy.pcmFormat = pcmFormat;
        copy.dsdMode = dsdMode;
        copy.dsdFormat = dsdFormat;
        copy.ateEnabled = ateEnabled;
        copy.ateStyle = ateStyle;
        copy.ateIntensity = ateIntensity;
        copy.ateNoiseDb = ateNoiseDb;
        copy.ateJitterPs = ateJitterPs;
        copy.atePhaseDeg = atePhaseDeg;
        copy.ateCrossoverDepth = ateCrossoverDepth;
        copy.ateEvenHarmonics = ateEvenHarmonics;
        copy.ateOddHarmonics = ateOddHarmonics;
        return copy;
    }
}
