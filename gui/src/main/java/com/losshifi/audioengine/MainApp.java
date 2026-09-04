package com.losshifi.audioengine;

import javafx.application.Application;
import javafx.application.Platform;
import javafx.animation.AnimationTimer;
import javafx.beans.binding.Bindings;
import javafx.beans.binding.BooleanBinding;
import javafx.beans.property.SimpleStringProperty;
import javafx.collections.FXCollections;
import javafx.collections.ObservableList;
import javafx.concurrent.Task;
import javafx.geometry.Insets;
import javafx.geometry.Pos;
import javafx.scene.Node;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.scene.canvas.Canvas;
import javafx.scene.canvas.GraphicsContext;
import javafx.scene.chart.LineChart;
import javafx.scene.chart.NumberAxis;
import javafx.scene.chart.XYChart;
import javafx.scene.control.Alert;
import javafx.scene.control.Button;
import javafx.scene.control.ButtonType;
import javafx.scene.control.CheckBox;
import javafx.scene.control.ComboBox;
import javafx.scene.control.Label;
import javafx.scene.control.ListCell;
import javafx.scene.control.ListView;
import javafx.scene.control.Labeled;
import javafx.scene.control.ProgressBar;
import javafx.scene.control.RadioButton;
import javafx.scene.control.ScrollPane;
import javafx.scene.control.Slider;
import javafx.scene.control.SplitPane;
import javafx.scene.control.Tab;
import javafx.scene.control.TabPane;
import javafx.scene.control.TextArea;
import javafx.scene.control.ToggleGroup;
import javafx.scene.input.Dragboard;
import javafx.scene.input.ClipboardContent;
import javafx.scene.input.TransferMode;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.GridPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Pane;
import javafx.scene.layout.Priority;
import javafx.scene.layout.Region;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;
import javafx.scene.paint.Color;
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
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ConcurrentHashMap;
import java.util.stream.Collectors;

public final class MainApp extends Application {
    private final ObservableList<BatchItem> items = FXCollections.observableArrayList();
    private final ListView<BatchItem> fileList = new ListView<>();
    private final Label queueStatsLabel = new Label("0 个文件");
    private final Label outputDirLabel = new Label("-");
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
    private final Slider ateNoiseSlider = new Slider(-140, 0, 0);
    private final Label ateNoiseLabel = new Label("AUTO");
    private final Slider ateJitterSlider = new Slider(0, 100, 0);
    private final Label ateJitterLabel = new Label("0 ps");
    private final Slider atePhaseSlider = new Slider(0, 2, 0);
    private final Label atePhaseLabel = new Label("0.00°");
    private final Slider ateCrossoverSlider = new Slider(0, 1, 0);
    private final Label ateCrossoverLabel = new Label("0.00");
    private final Slider ateEvenHarmonicSlider = new Slider(0.2, 2, 1);
    private final Label ateEvenHarmonicLabel = new Label("1.00x");
    private final Slider ateOddHarmonicSlider = new Slider(0.2, 2, 1);
    private final Label ateOddHarmonicLabel = new Label("1.00x");
    private final Button resetAteCustomButton = new Button("重置自定义");

    private final Label sampleRateLabel = new Label("-");
    private final Label bitDepthLabel = new Label("-");
    private final Label channelsLabel = new Label("-");
    private final Label durationLabel = new Label("-");
    private final Label familyLabel = new Label("-");
    private final Label metadataLabel = new Label("无");

    private final Button addButton = new Button("添加文件");
    private final Button outputButton = new Button("输出目录");
    private final Button clearButton = new Button("清空队列");
    private final Button removeButton = new Button("移除选中");
    private final Button skipSelectedButton = new Button("跳过选中");
    private final Button applyToSelectedButton = new Button("应用到选中");
    private final Button responseCompareButton = new Button("响应对比");
    private final Button matchReferenceButton = new Button("音色匹配");
    private final Button ateSelectButton = new Button("选择音频");
    private final Button startButton = new Button("开始转换");
    private final Button cancelButton = new Button("取消");
    private final Button openOutputButton = new Button("打开输出");
    private final Button dashboardLanguageButton = new Button("LANG: ENG");
    private final ComboBox<String> languageBox = new ComboBox<>();
    private final Label memoryLabel = new Label("HEAP 0 MB");
    private final Label cacheLabel = new Label("CACHE 0");
    private final Button converterOpenButton = new Button("OPEN CONVERTER");
    private final Button ateOpenButton = new Button("OPEN ATE LAB");
    private final Button dashboardOutputButton = new Button("OPEN OUTPUT");
    private final Label dashboardQueueLabel = new Label("QUEUE 0");
    private final Label outputStatusLabel = new Label("OUTPUT -");
    private final Label ateCurrentFileLabel = new Label("未选择文件");

    private AudioEngineService service;
    private AppConfig config;
    private Path outputDir;
    private Stage stage;
    private BorderPane root;
    private BorderPane dashboardRoot;
    private Scene converterScene;
    private Scene dashboardScene;
    private TabPane settingsTabs;
    private TechField techField;
    private String language = "English";
    private final SimpleStringProperty languageProperty = new SimpleStringProperty("English");
    private final Map<Object, String> textSources = new LinkedHashMap<>();
    private boolean batchRunning;
    private boolean selectingProgrammatically;
    private boolean applyingLanguageProgrammatically;
    private AudioInfo currentInfo;
    private BatchConversionTask batchTask;
    private BatchItem draggedItem;
    private final Set<Integer> recommendedRates = new HashSet<>();
    private final Map<String, AudioInfo> infoCache = new ConcurrentHashMap<>();
    private final Map<String, ResponseCurve> responseCache = new LinkedHashMap<>(64, 0.75f, true) {
        @Override
        protected boolean removeEldestEntry(Map.Entry<String, ResponseCurve> eldest) {
            return size() > 48;
        }
    };
    private static final Map<String, String> EN_TEXT = Map.ofEntries(
            Map.entry("主题", "Theme"),
            Map.entry("添加文件", "Add Files"),
            Map.entry("输出目录", "Output Dir"),
            Map.entry("清空队列", "Clear Queue"),
            Map.entry("进度", "Progress"),
            Map.entry("开始转换", "Start"),
            Map.entry("取消", "Cancel"),
            Map.entry("打开输出", "Open Output"),
            Map.entry("移除选中", "Remove"),
            Map.entry("队列", "Queue"),
            Map.entry("日志", "Logs"),
            Map.entry("响应对比", "Response Compare"),
            Map.entry("转换", "Convert"),
            Map.entry("文件", "File"),
            Map.entry("音色处理", "Tone Processing"),
            Map.entry("启用 ATE", "Enable ATE"),
            Map.entry("风格", "Style"),
            Map.entry("强度", "Intensity"),
            Map.entry("当前文件", "Current File"),
            Map.entry("ATE 操作", "ATE Actions"),
            Map.entry("选择音频", "Select Audio"),
            Map.entry("未选择文件", "No file selected"),
            Map.entry("采样率", "Sample Rate"),
            Map.entry("位深", "Bit Depth"),
            Map.entry("格式", "Format"),
            Map.entry("输出模式", "Output Mode"),
            Map.entry("格式参数", "Format Parameters"),
            Map.entry("元数据", "Metadata"),
            Map.entry("声道", "Channels"),
            Map.entry("时长", "Duration"),
            Map.entry("采样族", "Family"),
            Map.entry("选择文件", "Choose Files"),
            Map.entry("拖放音频文件", "Drop Audio Files"),
            Map.entry("读取中", "Reading"),
            Map.entry("就绪", "Ready"),
            Map.entry("未知", "Unknown"),
            Map.entry("无", "None"),
            Map.entry("分析中...", "Analyzing..."),
            Map.entry("原曲 / ATE 处理后频谱对比", "Original / ATE Response"),
            Map.entry("原曲", "Original"),
            Map.entry("处理后", "Processed"),
            Map.entry("频率 (Hz)", "Frequency (Hz)"),
            Map.entry("电平 (dB)", "Level (dB)"),
            Map.entry("ATE 响应曲线对比", "ATE Response Comparison"),
            Map.entry("读取失败", "Read Failed"),
            Map.entry("待处理", "Pending"),
            Map.entry("处理中", "Processing"),
            Map.entry("写入中", "Writing"),
            Map.entry("完成", "Done"),
            Map.entry("失败", "Failed"),
            Map.entry("已取消", "Cancelled"),
            Map.entry("跳过选中", "Skip Selected"),
            Map.entry("应用到选中", "Apply to Selected"),
            Map.entry("已应用设置", "Applied"),
            Map.entry("已跳过", "Skipped"),
            Map.entry("0 个文件", "0 files"),
            Map.entry("个文件", " files"),
            Map.entry("二次元", "Anime"),
            Map.entry("黑色", "Black"),
            Map.entry("DSD 模式", "DSD Mode"),
            Map.entry("打开输出目录失败: ", "Failed to open output dir: "),
            Map.entry("自定义 Lab", "Custom Lab"),
            Map.entry("底噪", "Noise Floor"),
            Map.entry("抖动", "Jitter"),
            Map.entry("声道相位", "Phase"),
            Map.entry("交越深度", "Crossover"),
            Map.entry("偶次谐波", "Even Harmonics"),
            Map.entry("奇次谐波", "Odd Harmonics"),
            Map.entry("重置自定义", "Reset Custom"),
            Map.entry("音色匹配", "Match Reference"),
            Map.entry("匹配中...", "Matching..."),
            Map.entry("AUTO", "Auto")
    );
    private static final Map<String, String> HANT_TEXT = Map.ofEntries(
            Map.entry("主题", "主題"),
            Map.entry("添加文件", "新增檔案"),
            Map.entry("输出目录", "輸出目錄"),
            Map.entry("清空队列", "清空佇列"),
            Map.entry("进度", "進度"),
            Map.entry("开始转换", "開始轉換"),
            Map.entry("打开输出", "開啟輸出"),
            Map.entry("移除选中", "移除選取"),
            Map.entry("队列", "佇列"),
            Map.entry("日志", "日誌"),
            Map.entry("响应对比", "回應對比"),
            Map.entry("转换", "轉換"),
            Map.entry("音色处理", "音色處理"),
            Map.entry("启用 ATE", "啟用 ATE"),
            Map.entry("强度", "強度"),
            Map.entry("当前文件", "目前檔案"),
            Map.entry("ATE 操作", "ATE 操作"),
            Map.entry("选择音频", "選擇音訊"),
            Map.entry("未选择文件", "尚未選擇檔案"),
            Map.entry("采样率", "取樣率"),
            Map.entry("位深", "位元深度"),
            Map.entry("输出模式", "輸出模式"),
            Map.entry("格式参数", "格式參數"),
            Map.entry("元数据", "中繼資料"),
            Map.entry("声道", "聲道"),
            Map.entry("时长", "時長"),
            Map.entry("采样族", "取樣家族"),
            Map.entry("选择文件", "選擇檔案"),
            Map.entry("拖放音频文件", "拖曳音訊檔案"),
            Map.entry("读取中", "讀取中"),
            Map.entry("就绪", "就緒"),
            Map.entry("未知", "未知"),
            Map.entry("无", "無"),
            Map.entry("原曲 / ATE 处理后频谱对比", "原曲 / ATE 處理後頻譜對比"),
            Map.entry("原曲", "原曲"),
            Map.entry("处理后", "處理後"),
            Map.entry("频率 (Hz)", "頻率 (Hz)"),
            Map.entry("电平 (dB)", "電平 (dB)"),
            Map.entry("ATE 响应曲线对比", "ATE 回應曲線對比"),
            Map.entry("读取失败", "讀取失敗"),
            Map.entry("待处理", "待處理"),
            Map.entry("处理中", "處理中"),
            Map.entry("写入中", "寫入中"),
            Map.entry("完成", "完成"),
            Map.entry("失败", "失敗"),
            Map.entry("已取消", "已取消"),
            Map.entry("跳过选中", "跳過選取"),
            Map.entry("应用到选中", "套用到選取"),
            Map.entry("已应用设置", "已套用設定"),
            Map.entry("已跳过", "已跳過"),
            Map.entry("0 个文件", "0 個檔案"),
            Map.entry("个文件", " 個檔案"),
            Map.entry("DSD 模式", "DSD 模式"),
            Map.entry("打开输出目录失败: ", "開啟輸出目錄失敗: "),
            Map.entry("自定义 Lab", "自訂 Lab"),
            Map.entry("底噪", "底噪"),
            Map.entry("抖动", "抖動"),
            Map.entry("声道相位", "聲道相位"),
            Map.entry("交越深度", "交越深度"),
            Map.entry("偶次谐波", "偶次諧波"),
            Map.entry("奇次谐波", "奇次諧波"),
            Map.entry("重置自定义", "重設自訂"),
            Map.entry("音色匹配", "音色匹配"),
            Map.entry("匹配中...", "匹配中..."),
            Map.entry("AUTO", "自動")
    );
    private static final Map<String, String> HANT_EN_TEXT = Map.ofEntries(
            Map.entry("AUDIO ENGINE", "音訊引擎"),
            Map.entry("DSP CONSOLE", "DSP 主控台"),
            Map.entry("HI-RES AUDIO PROCESSING", "高解析音訊處理"),
            Map.entry("PCM / DSD / ANALOG TONE LAB", "PCM / DSD / 類比音色實驗室"),
            Map.entry("CONVERTER", "轉換器"),
            Map.entry("Batch PCM / DSD conversion with memory-cached file metadata.",
                    "批次 PCM / DSD 轉換，並使用記憶體快取檔案中繼資料。"),
            Map.entry("ATE TONE LAB", "ATE 音色實驗室"),
            Map.entry("Analog-style response comparison and tone curve inspection.",
                    "類比風格回應對比與音色曲線檢視。"),
            Map.entry("OUTPUT", "輸出"),
            Map.entry("Open the current output folder without blocking the UI.",
                    "開啟目前輸出資料夾，且不會阻擋介面操作。"),
            Map.entry("OPEN CONVERTER", "開啟轉換器"),
            Map.entry("OPEN ATE LAB", "開啟 ATE 實驗室"),
            Map.entry("OPEN OUTPUT", "開啟輸出"),
            Map.entry("CACHED ANALYSIS", "快取分析"),
            Map.entry("HEAP ", "堆積 "),
            Map.entry("CACHE ", "快取 "),
            Map.entry("QUEUE ", "佇列 "),
            Map.entry("OUTPUT ", "輸出 "),
            Map.entry("THEME: BLACK", "主題：黑色"),
            Map.entry("THEME: ANIME", "主題：二次元"),
            Map.entry("LANG: ENG", "語言：EN"),
            Map.entry("LANG: 繁", "語言：繁"),
            Map.entry("LANG: 简", "語言：簡"),
            Map.entry("BACK", "返回"),
            Map.entry("Audio Engine", "音訊引擎")
    );
    private static final Map<String, String> HANS_EN_TEXT = Map.ofEntries(
            Map.entry("AUDIO ENGINE", "音频引擎"),
            Map.entry("DSP CONSOLE", "DSP 控制台"),
            Map.entry("HI-RES AUDIO PROCESSING", "高解析音频处理"),
            Map.entry("PCM / DSD / ANALOG TONE LAB", "PCM / DSD / 模拟音色实验室"),
            Map.entry("CONVERTER", "转换器"),
            Map.entry("Batch PCM / DSD conversion with memory-cached file metadata.",
                    "批量 PCM / DSD 转换，并使用内存缓存文件元数据。"),
            Map.entry("ATE TONE LAB", "ATE 音色实验室"),
            Map.entry("Analog-style response comparison and tone curve inspection.",
                    "模拟风格响应对比与音色曲线检查。"),
            Map.entry("OUTPUT", "输出"),
            Map.entry("Open the current output folder without blocking the UI.",
                    "打开当前输出目录，不阻塞界面操作。"),
            Map.entry("OPEN CONVERTER", "打开转换器"),
            Map.entry("OPEN ATE LAB", "打开 ATE 实验室"),
            Map.entry("OPEN OUTPUT", "打开输出"),
            Map.entry("CACHED ANALYSIS", "缓存分析"),
            Map.entry("HEAP ", "堆 "),
            Map.entry("CACHE ", "缓存 "),
            Map.entry("QUEUE ", "队列 "),
            Map.entry("OUTPUT ", "输出 "),
            Map.entry("THEME: BLACK", "主题：黑色"),
            Map.entry("THEME: ANIME", "主题：二次元"),
            Map.entry("LANG: ENG", "语言：EN"),
            Map.entry("LANG: 繁", "语言：繁"),
            Map.entry("LANG: 简", "语言：简"),
            Map.entry("BACK", "返回"),
            Map.entry("AUTO", "自动"),
            Map.entry("Audio Engine", "音频引擎")
    );
    private final ExecutorService infoExecutor = Executors.newSingleThreadExecutor(r -> {
        Thread thread = new Thread(r);
        thread.setDaemon(true);
        return thread;
    });

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

        root = new BorderPane();
        root.setTop(buildTopBar());
        root.setCenter(buildCenter());
        root.setBottom(buildBottomBar());
        configureDropTarget(root);

        converterScene = new Scene(root, 1240, 820);
        converterScene.getStylesheets().add(getClass().getResource("themes.css").toExternalForm());

        dashboardRoot = buildDashboard();
        dashboardScene = new Scene(dashboardRoot, 1240, 820);
        dashboardScene.getStylesheets().add(getClass().getResource("themes.css").toExternalForm());
        stage.setTitle("Audio Engine");

        fileList.setCellFactory(list -> new BatchCell());
        fileList.setItems(items);
        fileList.setOnDragDetected(event -> {
            draggedItem = fileList.getSelectionModel().getSelectedItem();
            if (draggedItem == null) {
                return;
            }
            ClipboardContent content = new ClipboardContent();
            content.putString("audio-engine-batch-item");
            Dragboard board = fileList.startDragAndDrop(TransferMode.MOVE);
            board.setDragView(fileList.snapshot(null, null));
            board.setContent(content);
            event.consume();
        });
        fileList.setOnDragOver(event -> {
            if (draggedItem != null) {
                event.acceptTransferModes(TransferMode.MOVE);
            }
            event.consume();
        });
        fileList.setOnDragDropped(event -> {
            if (draggedItem != null) {
                int from = items.indexOf(draggedItem);
                BatchItem target = fileList.getSelectionModel().getSelectedItem();
                int to = target == null ? items.size() - 1 : items.indexOf(target);
                if (from >= 0) {
                    items.remove(from);
                    to = Math.max(0, Math.min(to, items.size()));
                    items.add(to, draggedItem);
                    fileList.getSelectionModel().select(draggedItem);
                }
                draggedItem = null;
                event.setDropCompleted(true);
            }
            event.consume();
        });
        fileList.getSelectionModel().selectedItemProperty().addListener((obs, old, item) -> {
            if (item == null) {
                clearInfo();
            } else {
                loadInfo(item);
            }
            ateCurrentFileLabel.setText(item == null ? noFileText() : item.getFileName());
            updateCommandState();
        });

        loadSettingsIntoUi();
        applyBlackTheme();
        updateCommandState();
        recordTextSources(dashboardScene.getRoot());
        recordTextSources(converterScene.getRoot());
        setLanguage(config.get("language", "English"));

        List<File> initialFiles = getParameters().getRaw().stream()
                .map(File::new)
                .filter(File::isFile)
                .collect(Collectors.toList());
        if (!initialFiles.isEmpty()) {
            addFiles(initialFiles);
            showConverter();
        } else {
            showDashboard();
        }
        stage.show();
        if (techField != null && stage.getScene() == dashboardScene) {
            techField.start();
        }
    }

    private void showDashboard() {
        if (techField != null) {
            techField.stop();
            if (stage.isShowing()) {
                techField.start();
            }
        }
        updateDashboardStats();
        stage.setScene(dashboardScene);
        stage.setTitle("Audio Engine");
    }

    private void showConverter() {
        if (techField != null) {
            techField.stop();
        }
        stage.setScene(converterScene);
        stage.setTitle("Audio Engine - Converter");
    }

    private BorderPane buildDashboard() {
        BorderPane dashboard = new BorderPane();
        dashboard.getStyleClass().add("dashboard-root");

        dashboardLanguageButton.getStyleClass().add("ghost-button");
        dashboardLanguageButton.setOnAction(event -> {
            String next = switch (language) {
                case "English" -> "繁體中文";
                case "繁體中文" -> "简体中文";
                default -> "English";
            };
            setLanguage(next);
        });

        Label primaryTitle = dashboardTitle("AUDIO ENGINE");
        Label secondaryTitle = dashboardTitle("DSP CONSOLE");
        HBox header = new HBox(14,
                primaryTitle,
                secondaryTitle,
                memoryLabel,
                cacheLabel,
                dashboardLanguageButton);
        header.setAlignment(Pos.CENTER_LEFT);
        header.setPadding(new Insets(18, 24, 14, 24));
        HBox.setHgrow(secondaryTitle, Priority.ALWAYS);

        TechField field = new TechField();
        techField = field;

        Label kicker = new Label("HI-RES AUDIO PROCESSING");
        kicker.getStyleClass().add("dashboard-kicker");
        Label headline = new Label("AUDIO ENGINE");
        headline.getStyleClass().add("dashboard-headline");
        Label subline = new Label("PCM / DSD / ANALOG TONE LAB");
        subline.getStyleClass().add("dashboard-subline");
        VBox copy = new VBox(6, kicker, headline, subline);
        copy.setAlignment(Pos.CENTER_LEFT);
        copy.setPadding(new Insets(28, 42, 0, 42));
        StackPane.setAlignment(copy, Pos.TOP_LEFT);
        StackPane hero = new StackPane(field, copy);
        hero.setMinHeight(220);
        hero.setPrefHeight(250);

        converterOpenButton.setOnAction(event -> {
            if (settingsTabs != null) {
                settingsTabs.getSelectionModel().select(0);
            }
            showConverter();
        });
        ateOpenButton.setOnAction(event -> {
            showConverter();
            if (settingsTabs != null) {
                settingsTabs.getSelectionModel().select(1);
            }
        });
        dashboardOutputButton.setOnAction(event -> openOutputFolder());

        Node converterCard = dashboardModule(
                "01", "CONVERTER",
                "Batch PCM / DSD conversion with memory-cached file metadata.",
                converterOpenButton);
        Node ateCard = dashboardModule(
                "02", "ATE TONE LAB",
                "Analog-style response comparison and tone curve inspection.",
                ateOpenButton);
        Node outputCard = dashboardModule(
                "03", "OUTPUT",
                "Open the current output folder without blocking the UI.",
                dashboardOutputButton);
        HBox modules = new HBox(14, converterCard, ateCard, outputCard);
        modules.setAlignment(Pos.CENTER);

        dashboardQueueLabel.getStyleClass().add("dashboard-stat");
        outputStatusLabel.getStyleClass().add("dashboard-stat");
        outputStatusLabel.setMaxWidth(520);
        outputStatusLabel.setTextOverrun(javafx.scene.control.OverrunStyle.LEADING_ELLIPSIS);
        Region spacer = new Region();
        HBox.setHgrow(spacer, Priority.ALWAYS);
        HBox footer = new HBox(18,
                dashboardQueueLabel,
                outputStatusLabel,
                spacer,
                new Label("CACHED ANALYSIS"));
        footer.setAlignment(Pos.CENTER_LEFT);
        footer.setPadding(new Insets(16, 24, 20, 24));
        footer.getStyleClass().add("dashboard-status");

        VBox content = new VBox(16, hero, modules, footer);
        content.setPadding(new Insets(0, 24, 12, 24));
        dashboard.setTop(header);
        dashboard.setCenter(content);
        return dashboard;
    }

    private Node dashboardModule(String number, String title, String copy, Button action) {
        Label index = new Label(number);
        index.getStyleClass().add("module-index");
        Label titleLabel = new Label(title);
        titleLabel.getStyleClass().add("module-title");
        Label copyLabel = new Label(copy);
        copyLabel.getStyleClass().add("module-copy");
        copyLabel.setWrapText(true);
        action.setMaxWidth(Double.MAX_VALUE);
        action.getStyleClass().add("dashboard-action");
        VBox card = new VBox(12, index, titleLabel, copyLabel, action);
        card.setPrefWidth(330);
        card.setMinHeight(210);
        card.setPadding(new Insets(18));
        card.getStyleClass().add("dashboard-module");
        VBox.setVgrow(action, Priority.ALWAYS);
        return card;
    }

    private Label dashboardTitle(String text) {
        Label label = new Label(text);
        label.getStyleClass().add("dashboard-title");
        return label;
    }

    private void updateDashboardStats() {
        dashboardQueueLabel.setText(translateText("QUEUE ") + items.size());
        outputStatusLabel.setText(translateText("OUTPUT ")
                + (outputDir == null ? "-" : outputDir));
        Runtime runtime = Runtime.getRuntime();
        long used = runtime.totalMemory() - runtime.freeMemory();
        long max = runtime.maxMemory();
        memoryLabel.setText(String.format(translateText("HEAP ") + "%.0f / %.0f MB",
                used / 1024.0 / 1024.0, max / 1024.0 / 1024.0));
        cacheLabel.setText(translateText("CACHE ") + (infoCache.size() + responseCache.size()));
    }

    private void updateCacheStats() {
        cacheLabel.setText(translateText("CACHE ") + (infoCache.size() + responseCache.size()));
    }

    private void setLanguage(String value) {
        if (!List.of("English", "简体中文", "繁體中文").contains(value)) {
            return;
        }
        applyingLanguageProgrammatically = true;
        languageBox.setValue(value);
        applyingLanguageProgrammatically = false;
        language = value;
        languageProperty.set(value);
        config.set("language", value);
        saveConfig();
        applyLanguageToTextNodes();
        stage.setTitle(translateText("Audio Engine"));
        updateAteNoiseLabel();
        if (currentInfo != null) {
            showInfo(currentInfo);
        } else {
            clearInfo();
        }
        outputDirLabel.setText(outputDir == null ? "-" : outputDir.toString());
        BatchItem selected = fileList.getSelectionModel().getSelectedItem();
        ateCurrentFileLabel.setText(selected == null ? noFileText() : selected.getFileName());
        dashboardLanguageButton.setText(switch (value) {
            case "English" -> "LANG: ENG";
            case "繁體中文" -> "LANG: 繁";
            default -> "LANG: 简";
        });
        updateDashboardStats();
        updateQueueStats();
    }

    private void recordTextSources(Node node) {
        if (node instanceof Labeled labeled && !isDynamicTextNode(labeled)) {
            textSources.put(labeled, labeled.getText());
        }
        if (node instanceof TabPane tabs) {
            for (Tab tab : tabs.getTabs()) {
                textSources.put(tab, tab.getText());
            }
        }
        if (node instanceof Parent parent) {
            for (Node child : parent.getChildrenUnmodifiable()) {
                recordTextSources(child);
            }
        }
    }

    private boolean isDynamicTextNode(Node node) {
        return node == queueStatsLabel
                || node == outputStatusLabel
                || node == dashboardQueueLabel
                || node == memoryLabel
                || node == cacheLabel
                || node == sampleRateLabel
                || node == bitDepthLabel
                || node == channelsLabel
                || node == durationLabel
                || node == familyLabel
                || node == metadataLabel
                || node == ateCurrentFileLabel
                || node == outputDirLabel;
    }

    private void applyLanguageToTextNodes() {
        for (Map.Entry<Object, String> entry : textSources.entrySet()) {
            String translated = translateText(entry.getValue());
            if (entry.getKey() instanceof Labeled labeled) {
                labeled.setText(translated);
            } else if (entry.getKey() instanceof Tab tab) {
                tab.setText(translated);
            }
        }
    }

    private String translateText(String source) {
        return switch (language) {
            case "English" -> EN_TEXT.getOrDefault(source, source);
            case "繁體中文" -> HANT_TEXT.getOrDefault(
                    source, HANT_EN_TEXT.getOrDefault(source, source));
            default -> HANS_EN_TEXT.getOrDefault(source, source);
        };
    }

    private String noFileText() {
        return switch (language) {
            case "English" -> "No file selected";
            case "繁體中文" -> "尚未選擇檔案";
            default -> "未选择文件";
        };
    }

    private static String fileCacheKey(Path path) {
        File file = path.toAbsolutePath().toFile();
        return path.toAbsolutePath().normalize() + "|"
                + file.lastModified() + "|" + file.length();
    }

    private static String responseCacheKey(Path path, ConversionSettings settings) {
        return fileCacheKey(path)
                + "|" + settings.getMode()
                + "|" + settings.getPcmRate()
                + "|" + settings.getBitDepth()
                + "|" + settings.getPcmFormat()
                + "|" + settings.getDsdMode()
                + "|" + settings.getDsdFormat()
                + "|" + settings.isAteEnabled()
                + "|" + settings.getAteStyle()
                + "|" + String.format(Locale.ROOT, "%.6f", settings.getAteIntensity())
                + "|" + String.format(Locale.ROOT, "%.4f", settings.getAteNoiseDb())
                + "|" + String.format(Locale.ROOT, "%.4f", settings.getAteJitterPs())
                + "|" + String.format(Locale.ROOT, "%.4f", settings.getAtePhaseDeg())
                + "|" + String.format(Locale.ROOT, "%.4f", settings.getAteCrossoverDepth())
                + "|" + String.format(Locale.ROOT, "%.4f", settings.getAteEvenHarmonics())
                + "|" + String.format(Locale.ROOT, "%.4f", settings.getAteOddHarmonics());
    }

    private Node buildTopBar() {
        Label brand = new Label("Audio Engine");
        brand.getStyleClass().add("brand");
        Button backButton = new Button("BACK");
        backButton.getStyleClass().add("back-button");
        backButton.setOnAction(event -> showDashboard());

        languageBox.setItems(FXCollections.observableArrayList(
                "English", "简体中文", "繁體中文"));
        languageBox.valueProperty().addListener((obs, oldValue, newValue) -> {
            if (!applyingLanguageProgrammatically && newValue != null && !newValue.equals(oldValue)) {
                setLanguage(newValue);
            }
        });

        addButton.setOnAction(event -> chooseFiles());
        outputButton.setOnAction(event -> chooseOutputDir());
        clearButton.setOnAction(event -> clearQueue());

        Region spacer = new Region();
        HBox.setHgrow(spacer, Priority.ALWAYS);
        HBox bar = new HBox(10, backButton, brand, spacer, languageBox,
                addButton, outputButton, clearButton);
        bar.setAlignment(Pos.CENTER_LEFT);
        bar.setPadding(new Insets(12));
        bar.getStyleClass().add("panel");
        return bar;
    }

    private Node buildBottomBar() {
        progressBar.setMaxWidth(Double.MAX_VALUE);
        startButton.getStyleClass().add("primary-button");
        startButton.setOnAction(event -> startBatch());
        cancelButton.setOnAction(event -> cancelBatch());
        openOutputButton.setOnAction(event -> openOutputFolder());

        HBox bar = new HBox(10, new Label("进度"), progressBar,
                startButton, cancelButton, openOutputButton);
        bar.setAlignment(Pos.CENTER_LEFT);
        bar.setPadding(new Insets(12));
        bar.getStyleClass().add("panel");
        HBox.setHgrow(progressBar, Priority.ALWAYS);
        return bar;
    }

    private Node buildCenter() {
        return buildWorkspace();
    }

    private Node buildEmptyState() {
        Label title = new Label("拖放音频文件");
        title.getStyleClass().add("empty-title");
        Button browse = new Button("选择文件");
        browse.getStyleClass().add("primary-button");
        browse.setOnAction(event -> chooseFiles());

        VBox content = new VBox(18, title, browse);
        content.setAlignment(Pos.CENTER);
        content.setMaxSize(520, 300);
        content.getStyleClass().add("empty-drop");
        return content;
    }

    private Node buildWorkspace() {
        Label queueTitle = sectionTitle("队列");
        queueStatsLabel.getStyleClass().add("muted-label");
        Region queueHeaderSpacer = new Region();
        HBox.setHgrow(queueHeaderSpacer, Priority.ALWAYS);
        HBox queueHeader = new HBox(10, queueTitle, queueHeaderSpacer, queueStatsLabel);

        removeButton.setOnAction(event -> removeSelected());
        skipSelectedButton.setOnAction(event -> {
            BatchItem selected = fileList.getSelectionModel().getSelectedItem();
            if (selected != null) {
                selected.setSkipped(true);
            }
        });
        applyToSelectedButton.setOnAction(event -> {
            BatchItem selected = fileList.getSelectionModel().getSelectedItem();
            if (selected != null) {
                selected.setAteOverride(collectSettings());
                selected.setStatus("已应用设置");
            }
        });
        outputDirLabel.getStyleClass().add("muted-label");
        outputDirLabel.setMaxWidth(220);
        outputDirLabel.setPrefWidth(220);
        outputDirLabel.setTextOverrun(javafx.scene.control.OverrunStyle.LEADING_ELLIPSIS);
        Region queueActionSpacer = new Region();
        HBox.setHgrow(queueActionSpacer, Priority.ALWAYS);
        HBox queueActions = new HBox(8, removeButton, skipSelectedButton,
                applyToSelectedButton, queueActionSpacer, outputDirLabel);

        VBox left = new VBox(10,
                queueHeader,
                fileList,
                queueActions,
                sectionTitle("日志"),
                logArea);
        left.setPadding(new Insets(14));
        left.getStyleClass().add("panel");
        VBox.setVgrow(fileList, Priority.ALWAYS);
        VBox.setVgrow(logArea, Priority.ALWAYS);

        Node right = buildSettingsTabs();
        SplitPane split = new SplitPane(left, right);
        split.setDividerPositions(0.58);
        SplitPane.setResizableWithParent(right, false);
        return split;
    }

    private Node buildSettingsTabs() {
        Tab conversionTab = new Tab("转换", wrapScroll(buildConversionPanel()));
        Tab ateTab = new Tab("ATE", wrapScroll(buildAtePanel()));
        Tab detailsTab = new Tab("文件", wrapScroll(buildDetailsPanel()));

        settingsTabs = new TabPane(conversionTab, ateTab, detailsTab);
        TabPane tabs = settingsTabs;
        tabs.setTabClosingPolicy(TabPane.TabClosingPolicy.UNAVAILABLE);
        VBox box = new VBox(tabs);
        box.setPrefWidth(400);
        box.getStyleClass().add("panel");
        VBox.setVgrow(tabs, Priority.ALWAYS);
        return box;
    }

    private Node wrapScroll(Node content) {
        ScrollPane scroll = new ScrollPane(content);
        scroll.setFitToWidth(true);
        scroll.setFitToHeight(true);
        scroll.getStyleClass().add("settings-scroll");
        return scroll;
    }

    private Node buildConversionPanel() {
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
                        Alert.AlertType.WARNING,
                        "目标 " + value + " Hz 不是当前输入 " + currentInfo.getSampleRate()
                                + " Hz 的整数倍升频，可能引入不自然的高频成像。继续使用该值？",
                        ButtonType.OK, ButtonType.CANCEL);
                alert.setHeaderText("非整数倍升频");
                alert.showAndWait();
            }
        });
        configurePcmRateCell();

        bitDepthBox.setItems(FXCollections.observableArrayList(16, 24));
        pcmFormatBox.setItems(FXCollections.observableArrayList(ConversionSettings.PcmFormat.values()));
        dsdModeBox.setItems(FXCollections.observableArrayList(ConversionSettings.DsdMode.values()));
        dsdFormatBox.setItems(FXCollections.observableArrayList(ConversionSettings.DsdFormat.values()));

        VBox pcmPanel = formGrid(
                new Label("采样率"), pcmRateBox,
                new Label("位深"), bitDepthBox,
                new Label("格式"), pcmFormatBox);
        VBox dsdPanel = formGrid(
                new Label("DSD 模式"), dsdModeBox,
                new Label("格式"), dsdFormatBox);
        pcmPanel.visibleProperty().bind(pcmRadio.selectedProperty());
        pcmPanel.managedProperty().bind(pcmRadio.selectedProperty());
        dsdPanel.visibleProperty().bind(dsdRadio.selectedProperty());
        dsdPanel.managedProperty().bind(dsdRadio.selectedProperty());

        VBox panel = new VBox(12,
                sectionTitle("输出模式"),
                new HBox(18, pcmRadio, dsdRadio),
                sectionTitle("格式参数"),
                pcmPanel, dsdPanel);
        panel.setPadding(new Insets(12));
        return panel;
    }

    private Node buildAtePanel() {
        ateStyleBox.setItems(FXCollections.observableArrayList(ConversionSettings.AteStyle.values()));
        ateIntensitySlider.valueProperty().addListener((obs, old, value) ->
                ateIntensityLabel.setText(String.format("%.2f", value.doubleValue())));
        ateIntensityLabel.setText("0.50");
        ateNoiseSlider.valueProperty().addListener((obs, old, value) -> updateAteNoiseLabel());
        ateJitterSlider.valueProperty().addListener((obs, old, value) ->
                ateJitterLabel.setText(String.format("%.0f ps", value.doubleValue())));
        atePhaseSlider.valueProperty().addListener((obs, old, value) ->
                atePhaseLabel.setText(String.format("%.2f°", value.doubleValue())));
        ateCrossoverSlider.valueProperty().addListener((obs, old, value) ->
                ateCrossoverLabel.setText(String.format("%.2f", value.doubleValue())));
        ateEvenHarmonicSlider.valueProperty().addListener((obs, old, value) ->
                ateEvenHarmonicLabel.setText(String.format("%.2fx", value.doubleValue())));
        ateOddHarmonicSlider.valueProperty().addListener((obs, old, value) ->
                ateOddHarmonicLabel.setText(String.format("%.2fx", value.doubleValue())));
        resetAteCustomButton.setOnAction(event -> {
            ateNoiseSlider.setValue(0);
            ateJitterSlider.setValue(0);
            atePhaseSlider.setValue(0);
            ateCrossoverSlider.setValue(0);
            ateEvenHarmonicSlider.setValue(1.0);
            ateOddHarmonicSlider.setValue(1.0);
        });
        ateCurrentFileLabel.getStyleClass().add("muted-label");
        ateCurrentFileLabel.setWrapText(true);
        ateSelectButton.setMaxWidth(Double.MAX_VALUE);
        ateSelectButton.setOnAction(event -> chooseFiles());
        responseCompareButton.getStyleClass().add("primary-button");
        responseCompareButton.setMaxWidth(Double.MAX_VALUE);
        responseCompareButton.setOnAction(event -> analyzeResponseCurve());
        matchReferenceButton.setMaxWidth(Double.MAX_VALUE);
        matchReferenceButton.setOnAction(event -> matchReferenceTone());
        configureAteStyleCells();

        VBox panel = new VBox(12,
                sectionTitle("ATE 操作"),
                ateSelectButton,
                new Label("当前文件"), ateCurrentFileLabel,
                sectionTitle("音色处理"),
                ateCheck,
                new Label("风格"), ateStyleBox,
                new Label("强度"), ateIntensitySlider, ateIntensityLabel,
                sectionTitle("自定义 Lab"),
                new Label("底噪"), ateNoiseSlider, ateNoiseLabel,
                new Label("抖动"), ateJitterSlider, ateJitterLabel,
                new Label("声道相位"), atePhaseSlider, atePhaseLabel,
                new Label("交越深度"), ateCrossoverSlider, ateCrossoverLabel,
                new Label("偶次谐波"), ateEvenHarmonicSlider, ateEvenHarmonicLabel,
                new Label("奇次谐波"), ateOddHarmonicSlider, ateOddHarmonicLabel,
                resetAteCustomButton,
                matchReferenceButton,
                responseCompareButton);
        panel.setPadding(new Insets(12));
        return panel;
    }

    private void configureAteStyleCells() {
        ateStyleBox.setCellFactory(list -> new ListCell<>() {
            {
                languageProperty.addListener((obs, old, value) -> updateItem(getItem(), isEmpty()));
            }

            @Override
            protected void updateItem(ConversionSettings.AteStyle item, boolean empty) {
                super.updateItem(item, empty);
                setText(empty || item == null ? null : item.display(language));
            }
        });
        ateStyleBox.setButtonCell(new ListCell<>() {
            {
                languageProperty.addListener((obs, old, value) -> updateItem(getItem(), isEmpty()));
            }

            @Override
            protected void updateItem(ConversionSettings.AteStyle item, boolean empty) {
                super.updateItem(item, empty);
                setText(empty || item == null ? null : item.display(language));
            }
        });
    }

    private void updateAteNoiseLabel() {
        double noise = ateNoiseSlider.getValue();
        ateNoiseLabel.setText(noise >= -0.01 ? translateText("AUTO") : String.format("%.0f dB", noise));
    }

    private Node buildDetailsPanel() {
        GridPane grid = new GridPane();
        grid.setHgap(12);
        grid.setVgap(8);
        grid.addRow(0, new Label("采样率"), sampleRateLabel);
        grid.addRow(1, new Label("位深"), bitDepthLabel);
        grid.addRow(2, new Label("声道"), channelsLabel);
        grid.addRow(3, new Label("时长"), durationLabel);
        grid.addRow(4, new Label("采样族"), familyLabel);
        grid.addRow(5, new Label("元数据"), metadataLabel);
        metadataLabel.setWrapText(true);

        VBox panel = new VBox(12, sectionTitle("当前文件"), grid);
        panel.setPadding(new Insets(12));
        return panel;
    }

    private VBox formGrid(Label firstLabel, Node first, Label secondLabel, Node second,
                          Label thirdLabel, Node third) {
        GridPane grid = new GridPane();
        grid.setHgap(10);
        grid.setVgap(8);
        grid.addRow(0, firstLabel, first);
        grid.addRow(1, secondLabel, second);
        grid.addRow(2, thirdLabel, third);
        VBox box = new VBox(grid);
        box.setSpacing(6);
        return box;
    }

    private VBox formGrid(Label firstLabel, Node first, Label secondLabel, Node second) {
        GridPane grid = new GridPane();
        grid.setHgap(10);
        grid.setVgap(8);
        grid.addRow(0, firstLabel, first);
        grid.addRow(1, secondLabel, second);
        VBox box = new VBox(grid);
        box.setSpacing(6);
        return box;
    }

    private void configurePcmRateCell() {
        pcmRateBox.setCellFactory(list -> new ListCell<>() {
            @Override
            protected void updateItem(Integer rate, boolean empty) {
                super.updateItem(rate, empty);
                if (empty || rate == null) {
                    setText(null);
                    setStyle("");
                } else {
                    setText(rate + " Hz");
                    setStyle(currentInfo != null && !recommendedRates.contains(rate)
                            ? "-fx-text-fill: #999999;"
                            : "");
                }
            }
        });
        pcmRateBox.setButtonCell(new ListCell<>() {
            @Override
            protected void updateItem(Integer rate, boolean empty) {
                super.updateItem(rate, empty);
                setText(empty || rate == null ? null : rate + " Hz");
            }
        });
    }

    private void chooseFiles() {
        FileChooser chooser = new FileChooser();
        chooser.setTitle("选择音频文件");
        chooser.getExtensionFilters().add(new FileChooser.ExtensionFilter(
                "支持的音频",
                "*.wav", "*.flac", "*.mp3", "*.ogg", "*.opus", "*.m4a", "*.aac", "*.mp4",
                "*.aiff", "*.aif", "*.dsf", "*.dff"));
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
            outputDirLabel.setText(outputDir.toString());
            config.set("output_dir", outputDir.toString());
            saveConfig();
            try {
                Files.createDirectories(outputDir);
            } catch (IOException ex) {
                appendLog("输出目录创建失败: " + ex.getMessage());
            }
        }
    }

    private void addFiles(List<File> files) {
        boolean added = false;
        for (File file : files) {
            if (file == null || !file.isFile()) {
                continue;
            }
            Path path = file.toPath();
            boolean exists = items.stream().anyMatch(item -> item.getInput().equals(path));
            if (!exists) {
                items.add(new BatchItem(path));
                added = true;
            }
        }
        if (!added) {
            return;
        }
        if (fileList.getSelectionModel().getSelectedItem() == null) {
            fileList.getSelectionModel().select(0);
        }
        updateQueueStats();
        updateCommandState();
    }

    private void removeSelected() {
        BatchItem selected = fileList.getSelectionModel().getSelectedItem();
        if (selected != null) {
            items.remove(selected);
            updateQueueStats();
            updateCommandState();
        }
    }

    private void clearQueue() {
        if (batchRunning) {
            return;
        }
        items.clear();
        clearInfo();
        ateCurrentFileLabel.setText(noFileText());
        updateQueueStats();
        updateCommandState();
    }

    private void loadInfo(BatchItem item) {
        if (item.getInfo() != null) {
            showInfo(item.getInfo());
            return;
        }
        String cacheKey = fileCacheKey(item.getInput());
        AudioInfo cached = infoCache.get(cacheKey);
        if (cached != null) {
            item.setInfo(cached);
            item.setStatus("就绪");
            if (fileList.getSelectionModel().getSelectedItem() == item) {
                showInfo(cached);
            }
            updateQueueStats();
            updateCacheStats();
            return;
        }
        item.setStatus("读取中");
        Task<AudioInfo> infoTask = new Task<>() {
            @Override
            protected AudioInfo call() throws Exception {
                return service.readInfo(item.getInput().toString());
            }
        };
        infoTask.setOnSucceeded(event -> {
            item.setInfo(infoTask.getValue());
            infoCache.put(cacheKey, infoTask.getValue());
            if (!batchRunning) {
                item.setStatus("就绪");
            }
            if (fileList.getSelectionModel().getSelectedItem() == item) {
                showInfo(infoTask.getValue());
            }
            updateQueueStats();
            updateCacheStats();
        });
        infoTask.setOnFailed(event -> {
            item.setStatus("读取失败");
            if (fileList.getSelectionModel().getSelectedItem() == item) {
                appendLog("读取信息失败: " + item.getFileName() + " - "
                        + infoTask.getException().getMessage());
            }
        });
        infoExecutor.submit(infoTask);
    }

    private void showInfo(AudioInfo info) {
        currentInfo = info;
        sampleRateLabel.setText(info.getSampleRate() + " Hz");
        bitDepthLabel.setText(info.getBits() == 0
                ? translateText("未知")
                : info.getBits() + " bit");
        channelsLabel.setText(info.getChannels() + " ch");
        durationLabel.setText(String.format("%.3f s", info.getDuration()));
        familyLabel.setText(familyLabel(info));
        metadataLabel.setText(formatMetadata(info.getMetadata()));
        refreshRecommendations(info);
    }

    private void clearInfo() {
        currentInfo = null;
        sampleRateLabel.setText("-");
        bitDepthLabel.setText("-");
        channelsLabel.setText("-");
        durationLabel.setText("-");
        familyLabel.setText("-");
        metadataLabel.setText(translateText("无"));
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
        settings.setAteNoiseDb(ateNoiseSlider.getValue());
        settings.setAteJitterPs(ateJitterSlider.getValue());
        settings.setAtePhaseDeg(atePhaseSlider.getValue());
        settings.setAteCrossoverDepth(ateCrossoverSlider.getValue());
        settings.setAteEvenHarmonics(ateEvenHarmonicSlider.getValue());
        settings.setAteOddHarmonics(ateOddHarmonicSlider.getValue());
        return settings;
    }

    private void loadSettingsIntoUi() {
        boolean pcm = "PCM".equals(config.get("mode", "PCM"));
        pcmRadio.setSelected(pcm);
        dsdRadio.setSelected(!pcm);
        pcmRateBox.setValue(config.getInt("pcm_rate", 176400));
        bitDepthBox.setValue(config.getInt("bit_depth", 24));
        try {
            pcmFormatBox.setValue(ConversionSettings.PcmFormat.valueOf(
                    config.get("pcm_format", "WAV")));
        } catch (IllegalArgumentException ex) {
            pcmFormatBox.setValue(ConversionSettings.PcmFormat.WAV);
        }
        try {
            dsdModeBox.setValue(ConversionSettings.DsdMode.valueOf(
                    config.get("dsd_mode", "DSD256")));
        } catch (IllegalArgumentException ex) {
            dsdModeBox.setValue(ConversionSettings.DsdMode.DSD256);
        }
        try {
            dsdFormatBox.setValue(ConversionSettings.DsdFormat.valueOf(
                    config.get("dsd_format", "DSF")));
        } catch (IllegalArgumentException ex) {
            dsdFormatBox.setValue(ConversionSettings.DsdFormat.DSF);
        }
        ateCheck.setSelected(config.getBoolean("ate_enabled", false));
        ateIntensitySlider.setValue(config.getDouble("ate_intensity", 0.5));
        ateNoiseSlider.setValue(config.getDouble("ate_noise_db", 0));
        ateJitterSlider.setValue(config.getDouble("ate_jitter_ps", 0));
        atePhaseSlider.setValue(config.getDouble("ate_phase_deg", 0));
        ateCrossoverSlider.setValue(config.getDouble("ate_crossover_depth", 0));
        ateEvenHarmonicSlider.setValue(config.getDouble("ate_even_harmonics", 1.0));
        ateOddHarmonicSlider.setValue(config.getDouble("ate_odd_harmonics", 1.0));
        try {
            ateStyleBox.setValue(ConversionSettings.AteStyle.valueOf(
                    config.get("ate_style", "TUBE")));
        } catch (IllegalArgumentException ex) {
            ateStyleBox.setValue(ConversionSettings.AteStyle.TUBE);
        }
        outputDirLabel.setText(outputDir.toString());
    }

    private void saveSettings(ConversionSettings settings) {
        config.set("mode", settings.getMode().name());
        config.set("pcm_rate", String.valueOf(settings.getPcmRate()));
        config.set("bit_depth", String.valueOf(settings.getBitDepth()));
        config.set("pcm_format", settings.getPcmFormat().name());
        config.set("dsd_mode", settings.getDsdMode().name());
        config.set("dsd_format", settings.getDsdFormat().name());
        config.set("ate_enabled", String.valueOf(settings.isAteEnabled()));
        config.set("ate_style", settings.getAteStyle().name());
        config.set("ate_intensity", String.valueOf(settings.getAteIntensity()));
        config.set("ate_noise_db", String.valueOf(settings.getAteNoiseDb()));
        config.set("ate_jitter_ps", String.valueOf(settings.getAteJitterPs()));
        config.set("ate_phase_deg", String.valueOf(settings.getAtePhaseDeg()));
        config.set("ate_crossover_depth", String.valueOf(settings.getAteCrossoverDepth()));
        config.set("ate_even_harmonics", String.valueOf(settings.getAteEvenHarmonics()));
        config.set("ate_odd_harmonics", String.valueOf(settings.getAteOddHarmonics()));
        saveConfig();
    }

    private void saveConfig() {
        try {
            config.save();
        } catch (IOException ex) {
            appendLog("配置保存失败: " + ex.getMessage());
        }
    }

    private void startBatch() {
        if (batchRunning || items.isEmpty()) {
            return;
        }

        ConversionSettings settings = collectSettings();
        saveSettings(settings);
        List<BatchItem> snapshot = new ArrayList<>(items);
        batchRunning = true;
        updateCommandState();

        progressBar.progressProperty().unbind();
        progressBar.setProgress(0);
        logArea.clear();
        appendLog("开始批量转换");
        batchTask = new BatchConversionTask(
                snapshot,
                service,
                settings,
                outputDir,
                text -> Platform.runLater(() -> logArea.appendText(text + "\n")));
        progressBar.progressProperty().bind(batchTask.progressProperty());
        batchTask.setOnSucceeded(event -> finishBatch("全部完成", true));
        batchTask.setOnFailed(event -> finishBatch(
                "批量任务失败: " + batchTask.getException().getMessage(), false));
        batchTask.setOnCancelled(event -> finishBatch("批量任务已取消", false));

        Thread thread = new Thread(batchTask);
        thread.setDaemon(true);
        thread.start();
    }

    private void finishBatch(String message, boolean openOutput) {
        progressBar.progressProperty().unbind();
        batchRunning = false;
        updateCommandState();
        appendLog(message);
        if (openOutput) {
            openOutputFolder();
        }
    }

    private void cancelBatch() {
        if (batchRunning && batchTask != null) {
            batchTask.cancel(true);
        }
    }

    private void openOutputFolder() {
        if (outputDir == null) {
            return;
        }
        String os = System.getProperty("os.name", "").toLowerCase();
        if (os.contains("linux")) {
            try {
                new ProcessBuilder("xdg-open", outputDir.toString()).start();
            } catch (IOException ex) {
                appendLog("打开输出目录失败: " + ex.getMessage());
            }
            return;
        }
        if (!Desktop.isDesktopSupported()) {
            return;
        }
        Thread opener = new Thread(() -> {
            try {
                Desktop.getDesktop().open(outputDir.toFile());
            } catch (IOException ex) {
                Platform.runLater(() ->
                        appendLog("打开输出目录失败: " + ex.getMessage()));
            }
        });
        opener.setDaemon(true);
        opener.start();
    }

    private void updateQueueStats() {
        double totalSeconds = 0.0;
        for (BatchItem item : items) {
            if (item.getInfo() != null) {
                totalSeconds += item.getInfo().getDuration();
            }
        }
        String files = switch (language) {
            case "English" -> " files";
            case "繁體中文" -> " 個檔案";
            default -> " 个文件";
        };
        String duration = switch (language) {
            case "English" -> " min";
            case "繁體中文" -> " 分鐘";
            default -> " 分钟";
        };
        queueStatsLabel.setText(items.size() + files + (totalSeconds > 0
                ? String.format(Locale.ROOT, " / %.0f%s", totalSeconds / 60.0, duration)
                : ""));
    }

    private void updateCommandState() {
        boolean hasItems = !items.isEmpty();
        boolean hasSelection = fileList.getSelectionModel().getSelectedItem() != null;
        setConversionControlsEnabled(!batchRunning);
        startButton.setDisable(!hasItems || batchRunning);
        cancelButton.setDisable(!batchRunning);
        clearButton.setDisable(!hasItems || batchRunning);
        removeButton.setDisable(!hasSelection || batchRunning);
        skipSelectedButton.setDisable(!hasSelection || batchRunning);
        applyToSelectedButton.setDisable(!hasSelection || batchRunning);
        responseCompareButton.setDisable(!hasSelection || batchRunning);
    }

    private void setConversionControlsEnabled(boolean enabled) {
        pcmRadio.setDisable(!enabled);
        dsdRadio.setDisable(!enabled);
        pcmRateBox.setDisable(!enabled);
        bitDepthBox.setDisable(!enabled);
        pcmFormatBox.setDisable(!enabled);
        dsdModeBox.setDisable(!enabled);
        dsdFormatBox.setDisable(!enabled);
        ateCheck.setDisable(!enabled);
        ateStyleBox.setDisable(!enabled);
        ateIntensitySlider.setDisable(!enabled);
        ateNoiseSlider.setDisable(!enabled);
        ateJitterSlider.setDisable(!enabled);
        atePhaseSlider.setDisable(!enabled);
        ateCrossoverSlider.setDisable(!enabled);
        ateEvenHarmonicSlider.setDisable(!enabled);
        ateOddHarmonicSlider.setDisable(!enabled);
        resetAteCustomButton.setDisable(!enabled);
        matchReferenceButton.setDisable(!enabled);
    }

    private void applyBlackTheme() {
        if (root != null) {
            root.getStyleClass().removeAll("theme-anime", "theme-black");
            root.getStyleClass().add("theme-black");
            root.setStyle("-fx-background-color: #17181c;");
        }
        if (dashboardRoot != null) {
            dashboardRoot.getStyleClass().removeAll("theme-anime", "theme-black");
            dashboardRoot.getStyleClass().add("theme-black");
            dashboardRoot.setStyle("-fx-background-color: #17181c;");
        }
    }

    private void configureDropTarget(Node target) {
        target.setOnDragOver(event -> {
            Dragboard board = event.getDragboard();
            if (board.hasFiles()) {
                event.acceptTransferModes(TransferMode.COPY);
            }
            event.consume();
        });
        target.setOnDragDropped(event -> {
            Dragboard board = event.getDragboard();
            boolean completed = false;
            if (board.hasFiles()) {
                addFiles(board.getFiles());
                completed = true;
            }
            event.setDropCompleted(completed);
            event.consume();
        });
    }

    private void matchReferenceTone() {
        if (batchRunning) {
            return;
        }
        FileChooser chooser = new FileChooser();
        chooser.setTitle("选择参考音色");
        chooser.getExtensionFilters().add(new FileChooser.ExtensionFilter(
                "支持的音频",
                "*.wav", "*.flac", "*.mp3", "*.ogg", "*.opus", "*.m4a", "*.aac", "*.mp4",
                "*.aiff", "*.aif", "*.dsf", "*.dff"));
        File selected = chooser.showOpenDialog(stage);
        if (selected == null) {
            return;
        }

        matchReferenceButton.setDisable(true);
        matchReferenceButton.setText(translateText("匹配中..."));
        appendLog("开始分析参考音色: " + selected.getName());
        Task<AudioEngineService.ReferenceProfile> task = new Task<>() {
            @Override
            protected AudioEngineService.ReferenceProfile call() throws Exception {
                return service.analyzeReference(selected.getAbsolutePath());
            }
        };
        task.setOnSucceeded(event -> {
            matchReferenceButton.setDisable(false);
            matchReferenceButton.setText(translateText("音色匹配"));
            AudioEngineService.ReferenceProfile profile = task.getValue();
            applyReferenceProfile(profile);
            appendLog(String.format(
                    "匹配完成: even=%.1f dB, odd=%.1f dB, noise=%.1f dB, THD=%.3f%%",
                    profile.evenDb, profile.oddDb, profile.noiseFloorDb, profile.thdPercent));
        });
        task.setOnFailed(event -> {
            matchReferenceButton.setDisable(false);
            matchReferenceButton.setText(translateText("音色匹配"));
            Throwable error = task.getException();
            appendLog("参考音色分析失败: "
                    + (error == null ? "未知错误" : error.getMessage()));
        });
        Thread thread = new Thread(task);
        thread.setDaemon(true);
        thread.start();
    }

    private void applyReferenceProfile(AudioEngineService.ReferenceProfile profile) {
        double evenScale = Math.pow(10, (profile.evenDb + 60.0) / 40.0);
        double oddScale = Math.pow(10, (profile.oddDb + 60.0) / 40.0);
        ateEvenHarmonicSlider.setValue(clamp(0.2, 2.0, evenScale));
        ateOddHarmonicSlider.setValue(clamp(0.2, 2.0, oddScale));
        ateCheck.setSelected(true);
    }

    private static double clamp(double min, double max, double value) {
        return Math.max(min, Math.min(max, value));
    }

    private void analyzeResponseCurve() {
        BatchItem selected = fileList.getSelectionModel().getSelectedItem();
        if (selected == null || batchRunning) {
            return;
        }

        ConversionSettings settings = collectSettings();
        Path selectedPath = selected.getInput();
        String input = selectedPath.toString();
        String cacheKey = responseCacheKey(selectedPath, settings);
        ResponseCurve cached = responseCache.get(cacheKey);
        if (cached != null) {
            appendLog("响应曲线命中内存缓存: " + selected.getFileName());
            showResponseCurveWindow(cached);
            return;
        }
        responseCompareButton.setDisable(true);
        responseCompareButton.setText(translateText("分析中..."));
        appendLog("开始分析响应曲线: " + selected.getFileName());

        Task<ResponseCurve> task = new Task<>() {
            @Override
            protected ResponseCurve call() throws Exception {
                return service.readAteResponseCurve(input, settings);
            }
        };
        task.setOnSucceeded(event -> {
            responseCompareButton.setDisable(false);
            responseCompareButton.setText(translateText("响应对比"));
            updateCommandState();
            appendLog("响应曲线分析完成");
            ResponseCurve curve = task.getValue();
            responseCache.put(cacheKey, curve);
            updateCacheStats();
            showResponseCurveWindow(curve);
        });
        task.setOnFailed(event -> {
            responseCompareButton.setDisable(false);
            responseCompareButton.setText(translateText("响应对比"));
            updateCommandState();
            Throwable error = task.getException();
            appendLog("响应曲线分析失败: "
                    + (error == null ? "未知错误" : error.getMessage()));
        });

        Thread thread = new Thread(task);
        thread.setDaemon(true);
        thread.start();
    }

    private void showResponseCurveWindow(ResponseCurve curve) {
        if (curve.points().isEmpty()) {
            appendLog("响应曲线数据为空");
            return;
        }

        NumberAxis xAxis = new NumberAxis();
        xAxis.setLabel("频率 (Hz)");
        NumberAxis yAxis = new NumberAxis();
        yAxis.setLabel("电平 (dB)");
        double min = curve.points().get(0).beforeDb();
        double max = min;
        for (ResponseCurve.Point point : curve.points()) {
            min = Math.min(min, Math.min(point.beforeDb(), point.afterDb()));
            max = Math.max(max, Math.max(point.beforeDb(), point.afterDb()));
        }
        yAxis.setLowerBound(Math.floor(min) - 5);
        yAxis.setUpperBound(Math.ceil(max) + 5);
        yAxis.setAutoRanging(false);

        LineChart<Number, Number> chart = new LineChart<>(xAxis, yAxis);
        chart.setTitle("原曲 / ATE 处理后频谱对比");
        chart.setAnimated(false);
        chart.setCreateSymbols(false);
        chart.setLegendVisible(true);

        XYChart.Series<Number, Number> beforeSeries = new XYChart.Series<>();
        beforeSeries.setName("原曲");
        XYChart.Series<Number, Number> afterSeries = new XYChart.Series<>();
        afterSeries.setName("处理后");
        for (ResponseCurve.Point point : curve.points()) {
            beforeSeries.getData().add(new XYChart.Data<>(point.frequency(), point.beforeDb()));
            afterSeries.getData().add(new XYChart.Data<>(point.frequency(), point.afterDb()));
        }
        chart.getData().add(beforeSeries);
        chart.getData().add(afterSeries);

        BorderPane graphRoot = new BorderPane(chart);
        graphRoot.setPadding(new Insets(12));
        graphRoot.getStyleClass().add("theme-black");
        chart.getStyleClass().add("panel");
        Scene graphScene = new Scene(graphRoot, 980, 540);
        graphScene.getStylesheets().add(getClass().getResource("themes.css").toExternalForm());

        Stage graphStage = new Stage();
        graphStage.initOwner(stage);
        graphStage.setTitle("ATE 响应曲线对比");
        graphStage.setScene(graphScene);
        graphStage.show();
    }

    private void appendLog(String text) {
        logArea.appendText(text + "\n");
    }

    private void showFatal(String header, Throwable ex) {
        Alert alert = new Alert(Alert.AlertType.ERROR,
                header + "\n" + ex.getMessage(), ButtonType.OK);
        alert.setHeaderText(header);
        alert.showAndWait();
    }

    private Label sectionTitle(String text) {
        Label label = new Label(text);
        label.getStyleClass().add("section-title");
        return label;
    }

    private String familyLabel(AudioInfo info) {
        if (info == null) {
            return "-";
        }
        String suffix = switch (language) {
            case "English" -> " family";
            case "繁體中文" -> " 家族";
            default -> " 家族";
        };
        if (info.is44100Family()) {
            return "44.1k" + suffix;
        }
        if (info.is48000Family()) {
            return "48k" + suffix;
        }
        return info.getSampleRate() + " Hz";
    }

    private String formatMetadata(Map<String, String> metadata) {
        if (metadata.isEmpty()) {
            return translateText("无");
        }
        return metadata.entrySet().stream()
                .map(entry -> entry.getKey() + "=" + entry.getValue())
                .collect(Collectors.joining(", "));
    }

    private static final class TechField extends Pane {
        private final Canvas canvas = new Canvas();
        private final double[] x;
        private final double[] y;
        private final double[] phase;
        private final double[] speed;
        private boolean running;
        private long lastDraw;
        private final AnimationTimer timer = new AnimationTimer() {
            @Override
            public void handle(long now) {
                if (now - lastDraw < 80_000_000) {
                    return;
                }
                lastDraw = now;
                draw(now);
            }
        };

        TechField() {
            int points = 28;
            x = new double[points];
            y = new double[points];
            phase = new double[points];
            speed = new double[points];
            java.util.Random random = new java.util.Random(0x4154_455f_4956L);
            for (int i = 0; i < points; i++) {
                x[i] = random.nextDouble();
                y[i] = random.nextDouble();
                phase[i] = random.nextDouble() * Math.PI * 2.0;
                speed[i] = 0.2 + random.nextDouble() * 0.6;
            }
            getChildren().add(canvas);
            canvas.widthProperty().bind(widthProperty());
            canvas.heightProperty().bind(heightProperty());
        }

        void start() {
            if (running) {
                return;
            }
            running = true;
            lastDraw = 0;
            timer.start();
        }

        void stop() {
            running = false;
            timer.stop();
        }

        private void draw(long now) {
            double width = canvas.getWidth();
            double height = canvas.getHeight();
            if (width < 10 || height < 10) {
                return;
            }
            GraphicsContext g = canvas.getGraphicsContext2D();
            g.clearRect(0, 0, width, height);
            double t = now / 1_000_000_000.0;

            g.setLineWidth(1);
            for (int i = 0; i < 9; i++) {
                double yy = height * (i + 0.5) / 9.0;
                g.setStroke(Color.rgb(77, 193, 255, 0.10 + 0.03 * Math.sin(t * 0.7 + i)));
                g.strokeLine(0, yy, width, yy);
            }

            int bars = 54;
            double barWidth = width / (bars * 1.7);
            for (int i = 0; i < bars; i++) {
                double f = i / (double) bars;
                double wave = 0.42
                        + 0.26 * Math.sin(t * 1.6 + f * 18.0)
                        + 0.18 * Math.sin(t * 2.7 - f * 7.0)
                        + 0.08 * Math.sin(f * 53.0);
                double barHeight = Math.max(8, height * Math.max(0.05, wave) * (0.18 + 0.72 * f));
                double x = 24 + (width - 48) * f;
                double y = height - 36 - barHeight;
                if (i % 5 == 0) {
                    g.setStroke(Color.rgb(255, 184, 107, 0.75));
                } else if (i % 3 == 0) {
                    g.setStroke(Color.rgb(83, 227, 161, 0.65));
                } else {
                    g.setStroke(Color.rgb(77, 193, 255, 0.58));
                }
                g.strokeLine(x, height - 36, x, y);
            }

            for (int i = 0; i < x.length; i++) {
                double px = width * (0.5 + 0.5 * Math.sin(t * speed[i] * 0.35 + phase[i]));
                double py = height * (0.18 + 0.64 * (0.5 + 0.5 * Math.sin(t * speed[i] * 0.22 + i)));
                double pulse = 0.35 + 0.2 * Math.sin(t * 3.0 + i);
                g.setFill(Color.rgb(77, 193, 255, 0.28 * pulse));
                g.fillOval(px - 3, py - 3, 6, 6);
                g.setStroke(Color.rgb(77, 193, 255, 0.18));
                g.strokeLine(px, py, px + (x[i] - 0.5) * width * 0.12,
                        py + (y[i] - 0.5) * height * 0.12);
            }
        }
    }

    private final class BatchCell extends ListCell<BatchItem> {
        private final Label status = new Label();
        private final ProgressBar progress = new ProgressBar();

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

            Label name = new Label(item.getFileName());
            name.setWrapText(false);
            name.getStyleClass().add("file-name");
            progress.prefWidthProperty().bind(widthProperty().multiply(0.35));
            progress.progressProperty().bind(item.progressProperty());
            status.textProperty().bind(Bindings.createStringBinding(
                    () -> translateText(item.getStatus()),
                    item.statusProperty(), languageProperty));

            HBox row = new HBox(10, name, status, progress);
            row.setAlignment(Pos.CENTER_LEFT);
            HBox.setHgrow(name, Priority.ALWAYS);
            setGraphic(row);
        }
    }
}
