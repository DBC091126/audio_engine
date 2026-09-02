package com.losshifi.audioengine;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

public final class ResponseCurve {
    public record Point(double frequency, double beforeDb, double afterDb) {
    }

    private final List<Point> points;

    public ResponseCurve(List<Point> points) {
        this.points = List.copyOf(points);
    }

    public List<Point> points() {
        return points;
    }

    public static ResponseCurve parse(String text) throws IOException {
        List<Point> parsed = new ArrayList<>();
        for (String line : text.split("\\R")) {
            if (line.isBlank()) {
                continue;
            }
            String[] parts = line.split("\\t");
            if (parts.length < 3) {
                continue;
            }
            try {
                parsed.add(new Point(
                        Double.parseDouble(parts[0]),
                        Double.parseDouble(parts[1]),
                        Double.parseDouble(parts[2])));
            } catch (NumberFormatException ex) {
                throw new IOException("响应曲线数据无法解析: " + line, ex);
            }
        }
        if (parsed.isEmpty()) {
            throw new IOException("响应曲线数据为空");
        }
        return new ResponseCurve(parsed);
    }
}
