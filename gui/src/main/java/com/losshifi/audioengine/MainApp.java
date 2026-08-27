package com.losshifi.audioengine;

import javafx.application.Application;
import javafx.application.Platform;
import javafx.collections.FXCollections;
import javafx.concurrent.Task;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.control.CheckBox;
import javafx.scene.control.ComboBox;
import javafx.scene.control.Label;
import javafx.scene.control.ListCell;
import javafx.scene.control.ListView;
import javafx.scene.control.ProgressBar;
import javafx.scene.control.RadioButton;
import javafx.scene.control.Separator;
import javafx.scene.control.Slider;
import javafx.scene.control.TextArea;
import javafx.scene.control.ToggleGroup;
import javafx.scene.control.Alert;
import javafx.scene.control.Alert.AlertType;
import javafx.scene.control.ButtonType;
import javafx.scene.input.Dragboard;
import javafx.scene.input.TransferMode;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.GridPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Priority;
import javafx.scene.layout.VBox;
import javafx.stage.DirectoryChooser;
import javafx.stage.FileChooser;
import javafx.stage.Stage;

import java.awt.Desktop;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Map;
import java.util.Set;
import java.util.stream.Collectors;

public final class MainApp extends Application {
    private AudioEngineService service;
    private AppConfig config;
    private Path outputDir;
    private Stage stage;

    private final ListView<BatchItem> fileList = new ListView<>();
    private final Label sampleRateLabel = new Label("-");
    private final Label bitDepthLabel = new Label("-");
    private final Label channelsLabel = new Label("-");
    private final Label durationLabel = new Label("-");
    private final Label metadataLabel = new Label("-");
    private final TextArea logArea = new TextArea();
    private final ProgressBar progressBar = new ProgressBar(0);

    private final RadioButton pcmRadio = new RadioButton("PCM");
    private final RadioButton dsdRadio = new RadioButton("DSD");
    private final ComboBox<Integer> pcmRateBox = new ComboBox<>();
    private final ComboBox<Integer> bitDepthBox = new ComboBox<>();
    private final ComboBox<ConversionSettings.PcmFormat> pcmFormatBox = new ComboBox<>();
    private final ComboBox<ConversionSettings.DsdMode> dsdModeBox = new ComboBox<>();
    private final ComboBox<ConversionSettings.DsdFormat> dsdFormatBox = new ComboBox<>();
    private final CheckBox ateCheck = new CheckBox("启用 ATE");
    private final ComboBox<ConversionSettings.AteStyle> ateStyleBox = new ComboBox<>();
    private final Slider ateIntensitySlider = new Slider(0, 1, 0.5);
    private final Label ateIntensityLabel = new Label("0.50");

    private final Set<Integer> recommendedRates = new HashSet<>();
    private boolean selectingProgrammatically;
    private AudioInfo currentInfo;
    private BatchConversionTask batchTask;

    @Override
    public void start(Stage stage) {
        this.stage = stage;
        try {
            service = new AudioEngineService();
        } catch (Throwable ex) {
            showFatal("无法加载 audio_engine 动态库", ex);
            return;
        }

        config = AppConfig.loadDefault();
        outputDir = Path.of(config.get("output_dir",
                Path.of(System.getProperty("user.home"), "audio_engine_out").toString()));
        try {
            Files.createDirectories(outputDir);
        } catch (IOException ex) {
            showFatal("无法创建输出目录", ex);
            return;
        }

        buildUi(stage);
        loadSettingsIntoUi();
        stage.show();
    }

    private void buildUi(Stage stage) {
        BorderPane root = new BorderPane();
        root.setTop(buildTopBar());
        root.setLeft(buildControls());
        root.setCenter(buildCenter());
        root.setBottom(buildBottomBar());

        Scene scene = new Scene(root, 1180, 760);
        stage.setTitle("All-in Audio Engine");
        stage.setScene(scene);

        fileList.setCellFactory(list -> new BatchCell());
        fileList.getSelectionModel().selectedItemProperty().addListener((obs, old, item) -> {
            if (item != null) {
                loadInfo(item);
            }
        });
        fileList.setOnDragOver(event -> {
            Dragboard board = event.getDragboard();
            if (board.hasFiles()) {
                event.acceptTransferModes(TransferMode.COPY);
            }
            event.consume();
        });
        fileList.setOnDragDropped(event -> {
            Dragboard board = event.getDragboard();
            if (board.hasFiles()) {
                addFiles(board.getFiles());
                event.setDropCompleted(true);
            }
            event.consume();
        });
    }

    private HBox buildTopBar() {
        Button addButton = new Button("添加文件");
        addButton.setOnAction(event -> chooseFiles());

        Button chooseOutputButton = new Button("选择输出目录");
        chooseOutputButton.setOnAction(event -> chooseOutputDir());

        Button startButton = new Button("开始处理");
        startButton.setOnAction(event -> startBatch());

        Button cancelButton = new Button("取消");
        cancelButton.setOnAction(event -> cancelBatch());

        HBox bar = new HBox(10, addButton, chooseOutputButton, startButton, cancelButton);
        bar.setAlignment(Pos.CENTER_LEFT);
        bar.setPadding(new Insets(10));
        return bar;
    }

    private VBox buildControls() {
        ToggleGroup modeGroup = new ToggleGroup();
        pcmRadio.setToggleGroup(modeGroup);
        dsdRadio.setToggleGroup(modeGroup);
        pcmRadio.setSelected(true);

        pcmRateBox.setItems(FXCollections.observableArrayList(
                88200, 96000, 176400, 192000, 352800, 384000));
        pcmRateBox.valueProperty().addListener((obs, old, value) -> {
            if (!selectingProgrammatically && value != null && currentInfo != null
                    && !recommendedRates.contains(value)) {
                Alert alert = new Alert(
                        AlertType.WARNING,
                        "目标 " + value + " Hz 不是当前输入 " + currentInfo.getSampleRate()
                                + " Hz 的整数倍升频，可能引入不自然的高频成像。继续使用该值？",
                        ButtonType.OK, ButtonType.CANCEL);
                alert.setHeaderText("非整数倍升频");
                alert.showAndWait();
            }
        });

        bitDepthBox.setItems(FXCollections.observableArrayList(16, 24));
        bitDepthBox.setValue(24);
        pcmFormatBox.setItems(FXCollections.observableArrayList(ConversionSettings.PcmFormat.values()));
        pcmFormatBox.setValue(ConversionSettings.PcmFormat.WAV);

        dsdModeBox.setItems(FXCollections.observableArrayList(ConversionSettings.DsdMode.values()));
        dsdModeBox.setValue(ConversionSettings.DsdMode.DSD256);
        dsdFormatBox.setItems(FXCollections.observableArrayList(ConversionSettings.DsdFormat.values()));
        dsdFormatBox.setValue(ConversionSettings.DsdFormat.DSF);

        GridPane pcmGrid = new GridPane();
        pcmGrid.setHgap(8);
        pcmGrid.setVgap(8);
        pcmGrid.addRow(0, new Label("采样率"), pcmRateBox);
        pcmGrid.addRow(1, new Label("位深"), bitDepthBox);
        pcmGrid.addRow(2, new Label("格式"), pcmFormatBox);

        GridPane dsdGrid = new GridPane();
        dsdGrid.setHgap(8);
        dsdGrid.setVgap(8);
        dsdGrid.addRow(0, new Label("DSD 模式"), dsdModeBox);
        dsdGrid.addRow(1, new Label("格式"), dsdFormatBox);

        VBox pcmPanel = new VBox(10, pcmGrid);
        VBox dsdPanel = new VBox(10, dsdGrid);
        pcmPanel.visibleProperty().bind(pcmRadio.selectedProperty());
        pcmPanel.managedProperty().bind(pcmRadio.selectedProperty());
        dsdPanel.visibleProperty().bind(dsdRadio.selectedProperty());
        dsdPanel.managedProperty().bind(dsdRadio.selectedProperty());

        ateStyleBox.setItems(FXCollections.observableArrayList(ConversionSettings.AteStyle.values()));
        ateStyleBox.setValue(ConversionSettings.AteStyle.TUBE);
        ateIntensitySlider.valueProperty().addListener((obs, old, value) ->
                ateIntensityLabel.setText(String.format("%.2f", value.doubleValue())));
        ateIntensityLabel.setText("0.50");

        VBox atePanel = new VBox(10, ateCheck,
                new Label("风格"), ateStyleBox,
                new Label("强度"), ateIntensitySlider, ateIntensityLabel);
        atePanel.setPadding(new Insets(8));

        GridPane infoGrid = new GridPane();
        infoGrid.setHgap(10);
        infoGrid.setVgap(6);
        infoGrid.addRow(0, new Label("采样率"), sampleRateLabel);
        infoGrid.addRow(1, new Label("位深"), bitDepthLabel);
        infoGrid.addRow(2, new Label("声道"), channelsLabel);
        infoGrid.addRow(3, new Label("时长"), durationLabel);
        infoGrid.addRow(4, new Label("元数据"), metadataLabel);
        metadataLabel.setWrapText(true);

        VBox controls = new VBox(12,
                new Label("输出模式"), pcmRadio, dsdRadio,
                pcmPanel, dsdPanel,
                new Label("ATE"), atePanel,
                new Separator(),
                new Label("输入信息"), infoGrid);
        controls.setPadding(new Insets(12));
        controls.setPrefWidth(340);
        return controls;
    }

    private VBox buildCenter() {
        fileList.setPrefHeight(320);
        logArea.setEditable(false);
        logArea.setPrefHeight(220);
        VBox center = new VBox(10, new Label("批量文件"), fileList, new Label("日志"), logArea);
        center.setPadding(new Insets(12));
        VBox.setVgrow(fileList, Priority.ALWAYS);
        VBox.setVgrow(logArea, Priority.ALWAYS);
        return center;
    }

    private HBox buildBottomBar() {
        progressBar.setMaxWidth(Double.MAX_VALUE);
        HBox bottom = new HBox(10, new Label("进度"), progressBar);
        bottom.setPadding(new Insets(10));
        HBox.setHgrow(progressBar, Priority.ALWAYS);
        return bottom;
    }

    private void chooseFiles() {
        FileChooser chooser = new FileChooser();
        chooser.setTitle("选择音频文件");
        chooser.getExtensionFilters().add(new FileChooser.ExtensionFilter(
                "支持的音频",
                "*.wav", "*.flac", "*.mp3", "*.ogg", "*.opus", "*.m4a", "*.aac", "*.mp4"));
        List<File> files = chooser.showOpenMultipleDialog(stage);
        if (files != null) {
            addFiles(files);
        }
    }

    private void chooseOutputDir() {
        DirectoryChooser chooser = new DirectoryChooser();
        chooser.setTitle("选择输出目录");
        File selected = chooser.showDialog(stage);
        if (selected != null) {
            outputDir = selected.toPath();
            config.set("output_dir", outputDir.toString());
            try {
                Files.createDirectories(outputDir);
            } catch (IOException ex) {
                appendLog("输出目录创建失败: " + ex.getMessage());
            }
        }
    }

    private void addFiles(List<File> files) {
        for (File file : files) {
            Path path = file.toPath();
            boolean exists = fileList.getItems().stream()
                    .anyMatch(item -> item.getInput().equals(path));
            if (!exists) {
                fileList.getItems().add(new BatchItem(path));
            }
        }
        if (!fileList.getItems().isEmpty()) {
            fileList.getSelectionModel().select(0);
        }
    }

    private void loadInfo(BatchItem item) {
        Task<AudioInfo> infoTask = new Task<>() {
            @Override
            protected AudioInfo call() throws Exception {
                return service.readInfo(item.getInput().toString());
            }
        };
        infoTask.setOnSucceeded(event -> {
            if (fileList.getSelectionModel().getSelectedItem() == item) {
                item.setInfo(infoTask.getValue());
                showInfo(infoTask.getValue());
            }
        });
        infoTask.setOnFailed(event -> {
            if (fileList.getSelectionModel().getSelectedItem() == item) {
                appendLog("读取信息失败: " + item.getFileName() + " - "
                        + infoTask.getException().getMessage());
            }
        });
        Thread thread = new Thread(infoTask);
        thread.setDaemon(true);
        thread.start();
    }

    private void showInfo(AudioInfo info) {
        currentInfo = info;
        sampleRateLabel.setText(info.getSampleRate() + " Hz");
        bitDepthLabel.setText(info.getBits() == 0 ? "未知" : info.getBits() + " bit");
        channelsLabel.setText(info.getChannels() + " ch");
        durationLabel.setText(String.format("%.3f s", info.getDuration()));
        metadataLabel.setText(formatMetadata(info.getMetadata()));
        refreshRecommendations(info);
    }

    private static String formatMetadata(Map<String, String> metadata) {
        if (metadata.isEmpty()) {
            return "无";
        }
        return metadata.entrySet().stream()
                .map(entry -> entry.getKey() + "=" + entry.getValue())
                .collect(Collectors.joining(", "));
    }

    private void refreshRecommendations(AudioInfo info) {
        recommendedRates.clear();
        if (info != null && info.is44100Family()) {
            recommendedRates.add(88200);
            recommendedRates.add(176400);
            recommendedRates.add(352800);
        } else if (info != null && info.is48000Family()) {
            recommendedRates.add(96000);
            recommendedRates.add(192000);
            recommendedRates.add(384000);
        }

        selectingProgrammatically = true;
        Integer current = pcmRateBox.getValue();
        if (current == null || !recommendedRates.contains(current)) {
            if (!recommendedRates.isEmpty()) {
                pcmRateBox.setValue(recommendedRates.iterator().next());
            } else if (current == null) {
                pcmRateBox.setValue(176400);
            }
        }
        selectingProgrammatically = false;

        pcmRateBox.setCellFactory(list -> new ListCell<>() {
            @Override
            protected void updateItem(Integer rate, boolean empty) {
                super.updateItem(rate, empty);
                if (empty || rate == null) {
                    setText(null);
                    setStyle("");
                } else {
                    setText(rate + " Hz");
                    setStyle(currentInfo != null && recommendedRates.contains(rate)
                            ? "-fx-text-fill: black;"
                            : "-fx-text-fill: #888888;");
                }
            }
        });
        pcmRateBox.setButtonCell(new ListCell<>() {
            @Override
            protected void updateItem(Integer rate, boolean empty) {
                super.updateItem(rate, empty);
                if (empty || rate == null) {
                    setText(null);
                } else {
                    setText(rate + " Hz");
                }
            }
        });
    }

    private ConversionSettings collectSettings() {
        ConversionSettings settings = new ConversionSettings();
        settings.setMode(pcmRadio.isSelected()
                ? ConversionSettings.OutputMode.PCM
                : ConversionSettings.OutputMode.DSD);
        settings.setPcmRate(pcmRateBox.getValue() == null ? 176400 : pcmRateBox.getValue());
        settings.setBitDepth(bitDepthBox.getValue() == null ? 24 : bitDepthBox.getValue());
        settings.setPcmFormat(pcmFormatBox.getValue() == null
                ? ConversionSettings.PcmFormat.WAV
                : pcmFormatBox.getValue());
        settings.setDsdMode(dsdModeBox.getValue() == null
                ? ConversionSettings.DsdMode.DSD256
                : dsdModeBox.getValue());
        settings.setDsdFormat(dsdFormatBox.getValue() == null
                ? ConversionSettings.DsdFormat.DSF
                : dsdFormatBox.getValue());
        settings.setAteEnabled(ateCheck.isSelected());
        settings.setAteStyle(ateStyleBox.getValue() == null
                ? ConversionSettings.AteStyle.TUBE
                : ateStyleBox.getValue());
        settings.setAteIntensity(ateIntensitySlider.getValue());
        return settings;
    }

    private void loadSettingsIntoUi() {
        pcmRateBox.setValue(config.getInt("pcm_rate", 176400));
        bitDepthBox.setValue(config.getInt("bit_depth", 24));
        try {
            dsdModeBox.setValue(ConversionSettings.DsdMode.valueOf(
                    config.get("dsd_mode", "DSD256")));
        } catch (IllegalArgumentException ex) {
            dsdModeBox.setValue(ConversionSettings.DsdMode.DSD256);
        }
        ateCheck.setSelected(config.getBoolean("ate_enabled", false));
        ateIntensitySlider.setValue(config.getDouble("ate_intensity", 0.5));
    }

    private void saveSettings(ConversionSettings settings) {
        config.set("output_dir", outputDir.toString());
        config.set("pcm_rate", String.valueOf(settings.getPcmRate()));
        config.set("bit_depth", String.valueOf(settings.getBitDepth()));
        config.set("dsd_mode", settings.getDsdMode().name());
        config.set("ate_enabled", String.valueOf(settings.isAteEnabled()));
        config.set("ate_intensity", String.valueOf(settings.getAteIntensity()));
        try {
            config.save();
        } catch (IOException ex) {
            appendLog("配置保存失败: " + ex.getMessage());
        }
    }

    private void startBatch() {
        if (fileList.getItems().isEmpty()) {
            appendLog("请先添加文件");
            return;
        }
        ConversionSettings settings = collectSettings();
        saveSettings(settings);
        List<BatchItem> snapshot = new ArrayList<>(fileList.getItems());

        progressBar.progressProperty().unbind();
        progressBar.setProgress(0);
        logArea.clear();
        batchTask = new BatchConversionTask(
                snapshot,
                service,
                settings,
                outputDir,
                text -> Platform.runLater(() -> logArea.appendText(text + "\n")));
        progressBar.progressProperty().bind(batchTask.progressProperty());
        batchTask.setOnSucceeded(event -> {
            progressBar.progressProperty().unbind();
            progressBar.setProgress(1);
            appendLog("全部完成");
            openOutputFolder();
        });
        batchTask.setOnFailed(event -> {
            progressBar.progressProperty().unbind();
            appendLog("批量任务失败: " + batchTask.getException().getMessage());
        });
        batchTask.setOnCancelled(event -> {
            progressBar.progressProperty().unbind();
            appendLog("批量任务已取消");
        });

        Thread thread = new Thread(batchTask);
        thread.setDaemon(true);
        thread.start();
    }

    private void cancelBatch() {
        if (batchTask != null) {
            batchTask.cancel(true);
        }
    }

    private void openOutputFolder() {
        if (Desktop.isDesktopSupported()) {
            try {
                Desktop.getDesktop().open(outputDir.toFile());
            } catch (IOException ex) {
                appendLog("打开输出目录失败: " + ex.getMessage());
            }
        }
    }

    private void appendLog(String text) {
        logArea.appendText(text + "\n");
    }

    private void showFatal(String header, Throwable ex) {
        Alert alert = new Alert(AlertType.ERROR,
                header + "\n" + ex.getMessage(), ButtonType.OK);
        alert.setHeaderText(header);
        alert.showAndWait();
    }

    private static final class BatchCell extends ListCell<BatchItem> {
        private final ProgressBar progress = new ProgressBar();
        private final Label status = new Label();

        @Override
        protected void updateItem(BatchItem item, boolean empty) {
            super.updateItem(item, empty);
            progress.progressProperty().unbind();
            status.textProperty().unbind();
            if (empty || item == null) {
                setText(null);
                setGraphic(null);
                return;
            }
            progress.progressProperty().bind(item.progressProperty());
            status.textProperty().bind(item.statusProperty());
            setText(item.getFileName());
            HBox box = new HBox(10, status, progress);
            box.setAlignment(Pos.CENTER_LEFT);
            setGraphic(box);
        }
    }
}
