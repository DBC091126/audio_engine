package com.losshifi.audioengine;

import javafx.application.Platform;
import javafx.animation.AnimationTimer;
import javafx.collections.FXCollections;
import javafx.collections.ObservableList;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.control.Button;
import javafx.scene.control.Alert;
import javafx.scene.control.ButtonType;
import javafx.scene.control.Label;
import javafx.scene.control.ListCell;
import javafx.scene.control.ListView;
import javafx.scene.control.Slider;
import javafx.scene.control.TextField;
import javafx.scene.canvas.Canvas;
import javafx.scene.canvas.GraphicsContext;
import javafx.scene.image.Image;
import javafx.scene.image.ImageView;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Priority;
import javafx.scene.layout.Region;
import javafx.scene.layout.VBox;
import javafx.scene.media.Media;
import javafx.scene.media.MediaPlayer;
import javafx.scene.paint.Color;
import javafx.stage.DirectoryChooser;
import javafx.stage.FileChooser;
import javafx.stage.Stage;

import javax.sound.sampled.AudioFormat;
import javax.sound.sampled.AudioInputStream;
import javax.sound.sampled.AudioSystem;
import javax.sound.sampled.SourceDataLine;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Random;
import java.util.Set;
import java.util.function.Supplier;
import java.util.stream.Stream;

public final class MusicPlayerView extends BorderPane {
    private static final List<String> SUPPORTED = List.of(
            ".wav", ".flac", ".mp3", ".ogg", ".opus", ".m4a", ".aac", ".mp4",
            ".aiff", ".aif", ".dsf", ".dff");

    private final AudioEngineService service;
    private final Stage stage;
    private final Supplier<ConversionSettings> settingsSupplier;
    private final ObservableList<Path> allFiles = FXCollections.observableArrayList();
    private final ObservableList<Path> visibleFiles = FXCollections.observableArrayList();
    private final ListView<Path> fileList = new ListView<>(visibleFiles);
    private final Button addFolderButton = new Button("Add Folder");
    private final TextField searchField = new TextField();
    private final Button previousButton = new Button("\u23EE");
    private final Button playPauseButton = new Button("\u25B6");
    private final Button nextButton = new Button("\u23ED");
    private final Button shuffleButton = new Button("\uD83D\uDD00");
    private final Button repeatButton = new Button("\uD83D\uDD01");
    private final Button favoriteButton = new Button("☆");
    private final Button infoButton = new Button("Info");
    private final Button savePlaylistButton = new Button("Save Playlist");
    private final Button loadPlaylistButton = new Button("Load Playlist");
    private final Slider progressSlider = new Slider(0, 1, 0);
    private final Slider volumeSlider = new Slider(0, 1, 0.8);
    private final Label nowTitle = new Label("Nothing playing");
    private final Label nowInfo = new Label("-");
    private final Label positionLabel = new Label("0:00 / 0:00");
    private final Canvas spectrumCanvas = new Canvas(640, 90);
    private final double[] levelBars = new double[64];
    private final ImageView coverImage = new ImageView();

    private final Map<Path, Path> wavCache = new HashMap<>();
    private MediaPlayer mediaPlayer;
    private SourceDataLine liveLine;
    private Thread liveThread;
    private final Set<String> favorites = new HashSet<>();
    private final Random random = new Random();
    private boolean shuffle;
    private boolean repeat;
    private volatile boolean userStopped;
    private Path currentPath;
    private long mediaDurationMs;
    private boolean wasPlaying;

    public MusicPlayerView(
            AudioEngineService service,
            Stage stage,
            Supplier<ConversionSettings> settingsSupplier) {
        this.service = service;
        this.stage = stage;
        this.settingsSupplier = settingsSupplier;
        setTop(buildToolbar());
        setCenter(fileList);
        setBottom(buildNowPlaying());
        fileList.setCellFactory(list -> new FileCell());
        fileList.getSelectionModel().selectedItemProperty().addListener((obs, old, selected) -> {
            if (selected != null) {
                playFile(selected);
            }
        });
        volumeSlider.valueProperty().addListener((obs, old, value) -> {
            if (mediaPlayer != null) {
                mediaPlayer.setVolume(value.doubleValue());
            }
        });
        progressSlider.valueProperty().addListener((obs, old, value) -> {
            if (mediaPlayer != null && mediaDurationMs > 0
                    && Math.abs(mediaPlayer.getCurrentTime().toMillis() - value.doubleValue() * 1000.0) > 500) {
                mediaPlayer.seek(javafx.util.Duration.millis(value.doubleValue() * 1000.0));
            }
        });
        searchField.textProperty().addListener((obs, old, text) -> applyFilter());
        loadFavorites();
        AnimationTimer timer = new AnimationTimer() {
            @Override
            public void handle(long now) {
                drawVisualizer();
            }
        };
        timer.start();
    }

    private HBox buildToolbar() {
        addFolderButton.setOnAction(event -> chooseFolder());
        savePlaylistButton.setOnAction(event -> savePlaylist());
        loadPlaylistButton.setOnAction(event -> loadPlaylist());
        searchField.setPromptText("Search");
        searchField.setPrefWidth(260);
        Region spacer = new Region();
        HBox.setHgrow(spacer, Priority.ALWAYS);
        HBox bar = new HBox(10, addFolderButton, savePlaylistButton,
                loadPlaylistButton, searchField, spacer);
        bar.setAlignment(Pos.CENTER_LEFT);
        bar.setPadding(new Insets(12));
        bar.getStyleClass().add("panel");
        return bar;
    }

    private VBox buildNowPlaying() {
        nowTitle.getStyleClass().add("file-name");
        nowTitle.setMaxWidth(Double.MAX_VALUE);
        nowInfo.setStyle("-fx-text-fill: #9aa0a6;");
        previousButton.setOnAction(event -> previous());
        playPauseButton.setOnAction(event -> togglePlay());
        nextButton.setOnAction(event -> next());
        shuffleButton.setOnAction(event -> {
            shuffle = !shuffle;
            shuffleButton.setStyle(shuffle ? "-fx-text-fill: #4dc1ff;" : "");
        });
        repeatButton.setOnAction(event -> {
            repeat = !repeat;
            repeatButton.setStyle(repeat ? "-fx-text-fill: #4dc1ff;" : "");
        });
        favoriteButton.setOnAction(event -> toggleFavorite());
        infoButton.setOnAction(event -> showInfo());
        HBox controls = new HBox(14, previousButton, playPauseButton, nextButton,
                shuffleButton, repeatButton, favoriteButton, infoButton);
        controls.setAlignment(Pos.CENTER);
        HBox timing = new HBox(8, positionLabel, progressSlider);
        timing.setAlignment(Pos.CENTER_LEFT);
        HBox.setHgrow(progressSlider, Priority.ALWAYS);
        VBox left = new VBox(6, nowTitle, nowInfo);
        coverImage.setFitWidth(84);
        coverImage.setFitHeight(84);
        coverImage.setPreserveRatio(true);
        HBox artwork = new HBox(12, coverImage, left);
        artwork.setAlignment(Pos.CENTER_LEFT);
        VBox box = new VBox(10, spectrumCanvas, artwork, timing, controls, volumeSlider);
        box.setPadding(new Insets(14));
        box.getStyleClass().add("panel");
        return box;
    }

    private void chooseFolder() {
        DirectoryChooser chooser = new DirectoryChooser();
        chooser.setTitle("Select Music Folder");
        File selected = chooser.showDialog(stage);
        if (selected != null) {
            new Thread(() -> {
                List<Path> found = scanFolder(selected.toPath());
                Platform.runLater(() -> {
                    allFiles.setAll(found);
                    applyFilter();
                });
            }, "library-scanner").start();
        }
    }

    private static List<Path> scanFolder(Path root) {
        List<Path> result = new ArrayList<>();
        try (Stream<Path> stream = Files.walk(root)) {
            stream.filter(Files::isRegularFile)
                    .filter(MusicPlayerView::isSupported)
                    .sorted()
                    .forEach(result::add);
        } catch (Exception ignored) {
            // A folder may disappear while scanning.
        }
        return result;
    }

    private static boolean isSupported(Path path) {
        String name = path.getFileName().toString().toLowerCase();
        return SUPPORTED.stream().anyMatch(name::endsWith);
    }

    private void applyFilter() {
        String query = searchField.getText() == null ? "" : searchField.getText().toLowerCase();
        List<Path> filtered = new ArrayList<>();
        for (Path path : allFiles) {
            if (query.isEmpty() || path.getFileName().toString().toLowerCase().contains(query)) {
                filtered.add(path);
            }
        }
        visibleFiles.setAll(filtered);
    }

    private void playFile(Path path) {
        currentPath = path;
        nowTitle.setText(path.getFileName().toString());
        favoriteButton.setText(favorites.contains(path.toString()) ? "★" : "☆");
        loadCover(path);
        ConversionSettings settings = settingsSupplier.get();
        new Thread(() -> {
            try {
                Path wav = preparePlayable(path);
                Platform.runLater(() -> startPlayback(wav, settings));
            } catch (Exception ex) {
                Platform.runLater(() -> nowInfo.setText("Failed: " + ex.getMessage()));
            }
        }, "player-prepare").start();
    }

    private void startPlayback(Path wav, ConversionSettings settings) {
        if (settings.isAteEnabled()) {
            startLiveDsp(wav, settings);
        } else {
            startMedia(wav);
        }
    }

    private void startLiveDsp(Path wav, ConversionSettings settings) {
        stopLive();
        userStopped = false;
        try {
            AudioInputStream stream = AudioSystem.getAudioInputStream(wav.toFile());
            AudioFormat format = stream.getFormat();
            SourceDataLine line = AudioSystem.getSourceDataLine(format);
            line.open(format, 8192);
            line.start();
            liveLine = line;
            int sampleRate = (int) format.getSampleRate();
            int channelCount = Math.max(1, format.getChannels());
            var ate = settings.isAteEnabled() ? service.createAteStream(settings, sampleRate) : null;

            liveThread = new Thread(() -> {
                try {
                    int frames = 4096;
                    int sampleCount = frames * channelCount;
                    byte[] inputBytes = new byte[sampleCount * 2];
                    float[] inputFloats = new float[sampleCount];
                    float[] outputFloats = new float[sampleCount];
                    int count;
                    while ((count = stream.read(inputBytes, 0, inputBytes.length)) > 0) {
                        int samples = count / 2;
                        for (int i = 0; i < samples; i++) {
                            inputFloats[i] = littleToFloat(inputBytes, i);
                        }
                        service.processAteStream(ate, inputFloats, outputFloats);
                        int segments = levelBars.length;
                        for (int segment = 0; segment < segments; segment++) {
                            int start = segment * samples / segments;
                            int end = (segment + 1) * samples / segments;
                            double sum = 0;
                            for (int i = start; i < end; i++) {
                                sum += Math.abs(outputFloats[i]);
                            }
                            int segmentCount = end - start;
                            levelBars[segment] = segmentCount > 0
                                    ? Math.min(1.0, sum / segmentCount * 2.5)
                                    : 0;
                        }
                        byte[] outputBytes = new byte[samples * 2];
                        for (int i = 0; i < samples; i++) {
                            writeFloat(outputFloats[i], outputBytes, i);
                        }
                        line.write(outputBytes, 0, outputBytes.length);
                    }
                } catch (Exception ex) {
                    Platform.runLater(() -> nowInfo.setText("Failed: " + ex.getMessage()));
                } finally {
                    try {
                        line.drain();
                        line.stop();
                        line.close();
                    } catch (Exception ignored) {
                        // Best effort line cleanup.
                    }
                    if (ate != null) {
                        service.destroyAteStream(ate);
                    }
                    Platform.runLater(() -> {
                        liveLine = null;
                        playPauseButton.setText("\u25B6");
                    });
                    if (!userStopped) {
                        Platform.runLater(this::handleEndOfMedia);
                    }
                }
            }, "live-player");
            liveThread.setDaemon(true);
            liveThread.start();
            playPauseButton.setText("\u23F8");
        } catch (Exception ex) {
            nowInfo.setText("Failed: " + ex.getMessage());
        }
    }

    private void stopLive() {
        userStopped = true;
        if (liveThread != null) {
            liveThread.interrupt();
            liveThread = null;
        }
        if (liveLine != null) {
            try {
                liveLine.stop();
                liveLine.close();
            } catch (Exception ignored) {
                // Best effort.
            }
            liveLine = null;
        }
    }

    private static float littleToFloat(byte[] bytes, int sampleIndex) {
        int index = sampleIndex * 2;
        short value = (short) ((bytes[index] & 0xFF) | (bytes[index + 1] << 8));
        return value / 32768.0f;
    }

    private static void writeFloat(float sample, byte[] bytes, int sampleIndex) {
        int value = Math.round(Math.max(-1.0f, Math.min(1.0f, sample)) * 32767.0f);
        int index = sampleIndex * 2;
        bytes[index] = (byte) (value & 0xFF);
        bytes[index + 1] = (byte) ((value >> 8) & 0xFF);
    }

    private Path preparePlayable(Path path) throws Exception {
        Path cached = wavCache.get(path);
        if (cached != null && Files.isRegularFile(cached)) {
            return cached;
        }
        Path dir = Path.of(System.getProperty("user.home"), ".audio_engine", "playback");
        Files.createDirectories(dir);
        Path wav = dir.resolve("play_" + Math.abs(path.toString().hashCode()) + ".wav");
        AudioInfo info = service.readInfo(path.toString());
        ConversionSettings settings = new ConversionSettings();
        settings.setMode(ConversionSettings.OutputMode.PCM);
        settings.setPcmRate(info.getSampleRate());
        settings.setBitDepth(16);
        settings.setPcmFormat(ConversionSettings.PcmFormat.WAV);
        service.convert(path.toString(), wav.toString(), settings);
        wavCache.put(path, wav);
        return wav;
    }

    private void startMedia(Path wav) {
        stopLive();
        if (mediaPlayer != null) {
            mediaPlayer.stop();
        }
        Media media = new Media(wav.toUri().toString());
        mediaPlayer = new MediaPlayer(media);
        mediaDurationMs = (long) media.getDuration().toMillis();
        mediaPlayer.setOnReady(() -> {
            long duration = (long) media.getDuration().toMillis();
            mediaDurationMs = duration;
            progressSlider.setMax(Math.max(1.0, duration / 1000.0));
            positionLabel.setText("0:00 / " + format(duration));
            mediaPlayer.setVolume(volumeSlider.getValue());
            mediaPlayer.play();
            playPauseButton.setText("\u23F8");
        });
        mediaPlayer.setOnEndOfMedia(() -> handleEndOfMedia());
        mediaPlayer.currentTimeProperty().addListener((obs, old, current) -> {
            if (mediaDurationMs > 0) {
                double seconds = current.toMillis() / 1000.0;
                progressSlider.setValue(seconds);
                positionLabel.setText(format((long) current.toMillis()) + " / " + format(mediaDurationMs));
            }
        });
        mediaPlayer.play();
        playPauseButton.setText("\u23F8");
    }

    private void togglePlay() {
        if (liveLine != null) {
            stopLive();
            playPauseButton.setText("\u25B6");
            return;
        }
        if (mediaPlayer == null) {
            return;
        }
        if (mediaPlayer.getStatus() == MediaPlayer.Status.PLAYING) {
            mediaPlayer.pause();
            playPauseButton.setText("\u25B6");
        } else {
            mediaPlayer.play();
            playPauseButton.setText("\u23F8");
        }
    }

    private void previous() {
        int index = fileList.getSelectionModel().getSelectedIndex();
        if (index > 0) {
            fileList.getSelectionModel().select(index - 1);
        }
    }

    private void next() {
        if (shuffle && visibleFiles.size() > 1) {
            int current = fileList.getSelectionModel().getSelectedIndex();
            int nextIndex = current;
            while (nextIndex == current) {
                nextIndex = random.nextInt(visibleFiles.size());
            }
            fileList.getSelectionModel().select(nextIndex);
            return;
        }
        int index = fileList.getSelectionModel().getSelectedIndex();
        if (index >= 0 && index < visibleFiles.size() - 1) {
            fileList.getSelectionModel().select(index + 1);
        }
    }

    private void handleEndOfMedia() {
        if (repeat) {
            playFile(currentPath);
        } else {
            next();
        }
    }

    private void toggleFavorite() {
        if (currentPath == null) {
            return;
        }
        String key = currentPath.toString();
        if (favorites.contains(key)) {
            favorites.remove(key);
            favoriteButton.setText("☆");
        } else {
            favorites.add(key);
            favoriteButton.setText("★");
        }
        saveFavorites();
    }

    private void showInfo() {
        if (currentPath == null) {
            return;
        }
        new Thread(() -> {
            try {
                AudioInfo info = service.readInfo(currentPath.toString());
                String text = currentPath.getFileName()
                        + "\nSample rate: " + info.getSampleRate() + " Hz"
                        + "\nChannels: " + info.getChannels()
                        + "\nDuration: " + String.format("%.3f s", info.getDuration())
                        + "\nBits: " + info.getBits()
                        + "\n\nMetadata:\n" + info.getMetadata();
                Platform.runLater(() -> new Alert(
                        Alert.AlertType.INFORMATION, text, ButtonType.OK).showAndWait());
            } catch (Exception ex) {
                Platform.runLater(() -> nowInfo.setText("Failed: " + ex.getMessage()));
            }
        }, "file-info").start();
    }

    private Path favoriteFile() {
        return Path.of(System.getProperty("user.home"), ".audio_engine", "favorites.txt");
    }

    private void loadFavorites() {
        try {
            Path file = favoriteFile();
            if (Files.isRegularFile(file)) {
                favorites.clear();
                for (String line : Files.readAllLines(file, StandardCharsets.UTF_8)) {
                    if (!line.isBlank()) {
                        favorites.add(line);
                    }
                }
            }
        } catch (IOException ignored) {
            // First launch or unreadable favorites file.
        }
    }

    private void saveFavorites() {
        try {
            Path file = favoriteFile();
            Files.createDirectories(file.getParent());
            Files.write(file, favorites, StandardCharsets.UTF_8);
        } catch (IOException ignored) {
            // Best effort persistence.
        }
    }

    private void loadCover(Path path) {
        Path parent = path.getParent();
        if (parent == null) {
            coverImage.setImage(null);
            return;
        }
        String[] names = {"cover.jpg", "cover.png", "folder.jpg", "folder.png",
                "front.jpg", "front.png", "AlbumArt.jpg", "AlbumArt.png"};
        for (String name : names) {
            Path candidate = parent.resolve(name);
            if (Files.isRegularFile(candidate)) {
                coverImage.setImage(new Image(candidate.toUri().toString()));
                return;
            }
        }
        coverImage.setImage(null);
    }

    private void savePlaylist() {
        FileChooser chooser = new FileChooser();
        chooser.setTitle("Save Playlist");
        chooser.getExtensionFilters().add(new FileChooser.ExtensionFilter("M3U Playlist", "*.m3u"));
        File selected = chooser.showSaveDialog(stage);
        if (selected == null) {
            return;
        }
        try {
            StringBuilder text = new StringBuilder("#EXTM3U\n");
            for (Path path : visibleFiles) {
                text.append(path.toAbsolutePath()).append('\n');
            }
            Files.writeString(selected.toPath(), text.toString(), StandardCharsets.UTF_8);
        } catch (IOException ex) {
            nowInfo.setText("Failed: " + ex.getMessage());
        }
    }

    private void loadPlaylist() {
        FileChooser chooser = new FileChooser();
        chooser.setTitle("Load Playlist");
        chooser.getExtensionFilters().add(new FileChooser.ExtensionFilter("M3U Playlist", "*.m3u"));
        File selected = chooser.showOpenDialog(stage);
        if (selected == null) {
            return;
        }
        try {
            List<Path> loaded = new ArrayList<>();
            for (String line : Files.readAllLines(selected.toPath(), StandardCharsets.UTF_8)) {
                if (line.isBlank() || line.startsWith("#")) {
                    continue;
                }
                Path path = Path.of(line);
                if (Files.isRegularFile(path) && isSupported(path)) {
                    loaded.add(path);
                }
            }
            allFiles.setAll(loaded);
            applyFilter();
        } catch (IOException ex) {
            nowInfo.setText("Failed: " + ex.getMessage());
        }
    }

    private static String format(long millis) {
        long totalSeconds = millis / 1000;
        return String.format("%d:%02d", totalSeconds / 60, totalSeconds % 60);
    }

    private void drawVisualizer() {
        GraphicsContext g = spectrumCanvas.getGraphicsContext2D();
        double width = spectrumCanvas.getWidth();
        double height = spectrumCanvas.getHeight();
        g.clearRect(0, 0, width, height);
        g.setFill(Color.rgb(18, 19, 26));
        g.fillRect(0, 0, width, height);
        g.setStroke(Color.rgb(255, 255, 255, 0.10));
        g.strokeLine(0, height / 2, width, height / 2);
        double slot = width / levelBars.length;
        double barWidth = slot * 0.62;
        g.setFill(Color.rgb(77, 193, 255, 0.88));
        for (int i = 0; i < levelBars.length; i++) {
            double barHeight = Math.max(1.0, levelBars[i] * (height - 8));
            double x = i * slot + (slot - barWidth) / 2;
            g.fillRect(x, height - barHeight, barWidth, barHeight);
        }
    }

    private final class FileCell extends ListCell<Path> {
        @Override
        protected void updateItem(Path path, boolean empty) {
            super.updateItem(path, empty);
            if (empty || path == null) {
                setText(null);
            } else {
                setText(path.getFileName().toString());
            }
        }
    }
}
