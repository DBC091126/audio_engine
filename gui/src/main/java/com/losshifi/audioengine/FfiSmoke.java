package com.losshifi.audioengine;

public final class FfiSmoke {
    private FfiSmoke() {
    }

    public static void main(String[] args) throws Exception {
        if (args.length == 0) {
            System.err.println("usage: FfiSmoke <input> [output]");
            System.exit(2);
        }

        AudioEngineService service = new AudioEngineService();
        AudioInfo info = service.readInfo(args[0]);
        System.out.printf("info=%d Hz, %d ch, %d bit, %.3fs, metadata=%d%n",
                info.getSampleRate(),
                info.getChannels(),
                info.getBits(),
                info.getDuration(),
                info.getMetadata().size());

        if (args.length > 1) {
            ConversionSettings settings = new ConversionSettings();
            settings.setMode(ConversionSettings.OutputMode.PCM);
            settings.setPcmRate(176400);
            settings.setBitDepth(24);
            settings.setPcmFormat(ConversionSettings.PcmFormat.WAV);
            service.convert(args[0], args[1], settings);
            System.out.println("converted=" + args[1]);
            ResponseCurve curve = service.readAteResponseCurve(args[0], settings);
            System.out.println("response_curve_points=" + curve.points().size());
            AudioEngineService.ReferenceProfile profile = service.analyzeReference(args[0]);
            System.out.printf("reference even=%.1f odd=%.1f noise=%.1f thd=%.3f%n",
                    profile.evenDb, profile.oddDb, profile.noiseFloorDb, profile.thdPercent);
        }
    }
}
