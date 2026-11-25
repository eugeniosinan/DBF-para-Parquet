use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Instant;

use eframe::egui::{self, Color32, RichText, ScrollArea, Stroke, Vec2};
use tempfile::TempDir;

use crate::archive::{ArchiveExtractionResult, ArchiveManager};
use crate::dbf::{ConversionMode, DbfEncoding};
use crate::detector::{DbfAnalysis, DbfDetector, HealthSystem, PrivacyLevel};
use crate::parquet_writer::{ParquetConverter, ProgressUpdate};
use crate::report::{PostProcessingReport, PreProcessingReport};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum EncodingChoice {
    #[default]
    AutomaticStandard, // Padronizado por tipo de base (SIM, SINASC, SIVEP, DENGON -> ISO-8859-1)
    Iso8859_1,
    Cp850,
    Utf8,
}

impl EncodingChoice {
    pub fn resolve(&self, dbf: &DbfAnalysis) -> DbfEncoding {
        match self {
            EncodingChoice::AutomaticStandard => dbf.standard_encoding,
            EncodingChoice::Iso8859_1 => DbfEncoding::Iso8859_1,
            EncodingChoice::Cp850 => DbfEncoding::Cp850,
            EncodingChoice::Utf8 => DbfEncoding::Utf8,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum OutputMode {
    Individual,
    Merged,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppTab {
    Files,
    PreReport,
    Merge,
    Settings,
    PostReport,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InfoModalType {
    HealthPattern,
    StrictDbf,
    AboutAuthor,
}

pub enum WorkerMessage {
    Progress(ProgressUpdate),
    Completed(PostProcessingReport),
    Error(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    DatasusBlue,
    Light,
}

impl ThemeMode {
    pub fn display_name(&self) -> &str {
        match self {
            ThemeMode::Dark => "Escuro Moderno",
            ThemeMode::DatasusBlue => "Azul Saúde",
            ThemeMode::Light => "Claro Minimalista",
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();

        // Espaçamentos modernos e confortáveis (elimina a rigidez de WinForms anos 90)
        style.spacing.item_spacing = Vec2::new(10.0, 9.0);
        style.spacing.button_padding = Vec2::new(14.0, 7.5);
        style.spacing.indent = 16.0;

        match self {
            ThemeMode::Dark => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = Color32::from_rgb(11, 15, 25); // #0B0F19 canvas escuro
                visuals.window_fill = Color32::from_rgb(22, 31, 48); // #161F30 superfície de cards
                visuals.extreme_bg_color = Color32::from_rgb(15, 23, 42); // #0F172A inputs/text_edit

                visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);

                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(22, 31, 48);
                visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(37, 49, 74));
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(30, 41, 64);
                visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(45, 60, 90));
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(45, 60, 92);
                visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(70, 95, 140));
                visuals.widgets.active.bg_fill = Color32::from_rgb(37, 99, 235);
                visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250));
                visuals.selection.bg_fill = Color32::from_rgb(37, 99, 235);

                style.visuals = visuals;
            }
            ThemeMode::DatasusBlue => {
                let mut visuals = egui::Visuals::dark();
                visuals.panel_fill = Color32::from_rgb(8, 18, 33);
                visuals.window_fill = Color32::from_rgb(14, 32, 58);
                visuals.extreme_bg_color = Color32::from_rgb(10, 24, 44);

                visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);

                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(14, 32, 58);
                visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(24, 52, 92));
                visuals.widgets.inactive.bg_fill = Color32::from_rgb(20, 46, 80);
                visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(32, 70, 122));
                visuals.widgets.hovered.bg_fill = Color32::from_rgb(28, 65, 112);
                visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(56, 189, 248));
                visuals.widgets.active.bg_fill = Color32::from_rgb(14, 116, 144);
                visuals.selection.bg_fill = Color32::from_rgb(2, 132, 199);

                style.visuals = visuals;
            }
            ThemeMode::Light => {
                let mut visuals = egui::Visuals::light();
                visuals.panel_fill = Color32::from_rgb(248, 250, 252);
                visuals.window_fill = Color32::from_rgb(255, 255, 255);
                visuals.extreme_bg_color = Color32::from_rgb(241, 245, 249);

                visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
                visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);

                visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(255, 255, 255);
                visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225));
                visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(15, 23, 42));

                visuals.widgets.inactive.bg_fill = Color32::from_rgb(241, 245, 249);
                visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(203, 213, 225));
                visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(15, 23, 42));

                visuals.widgets.hovered.bg_fill = Color32::from_rgb(226, 232, 240);
                visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(148, 163, 184));
                visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(15, 23, 42));

                visuals.widgets.active.bg_fill = Color32::from_rgb(37, 99, 235);
                visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
                visuals.selection.bg_fill = Color32::from_rgb(191, 219, 254);
                visuals.selection.stroke = Stroke::new(1.0_f32, Color32::from_rgb(37, 99, 235));

                style.visuals = visuals;
            }
        }

        ctx.set_style(style);
    }
}

/// Helper para renderizar um card moderno com cantos arredondados e borda sutil
fn ui_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let frame = egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(egui::CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(18, 16));
    frame.show(ui, add_contents).inner
}

pub struct DbfConverterApp {
    // Arquivos brutos adicionados pelo usuário
    input_paths: Vec<PathBuf>,

    // Arquivos DBF identificados e prontos para conversão (podem ser extraídos temporários)
    inspected_dbfs: Vec<DbfAnalysis>,
    invalid_archives: Vec<String>,
    _active_temp_dirs: Vec<TempDir>,

    // Opções
    output_mode: OutputMode,
    conversion_mode: ConversionMode,
    encoding_choice: EncodingChoice,
    custom_output_dir: Option<PathBuf>,
    custom_output_filename: String,

    // Tema e Imagem
    theme_mode: ThemeMode,
    theme_initialized: bool,
    logo_texture: Option<egui::TextureHandle>,

    // Modal de Informações
    info_modal: Option<InfoModalType>,

    // Relatórios
    pre_report: Option<PreProcessingReport>,
    post_report: Option<PostProcessingReport>,

    // Estado da conversão
    is_converting: bool,
    current_progress: Option<ProgressUpdate>,
    status_message: String,
    active_tab: AppTab,

    // Comunicação entre thread de conversão e UI
    worker_rx: Option<Receiver<WorkerMessage>>,
}

impl Default for DbfConverterApp {
    fn default() -> Self {
        Self {
            input_paths: Vec::new(),
            inspected_dbfs: Vec::new(),
            invalid_archives: Vec::new(),
            _active_temp_dirs: Vec::new(),
            output_mode: OutputMode::Individual,
            conversion_mode: ConversionMode::EpidemiologicalDates,
            encoding_choice: EncodingChoice::AutomaticStandard,
            custom_output_dir: None,
            custom_output_filename: "base_vigilancia_consolidada.parquet".to_string(),
            theme_mode: ThemeMode::Dark,
            theme_initialized: false,
            logo_texture: None,
            info_modal: None,
            pre_report: None,
            post_report: None,
            is_converting: false,
            current_progress: None,
            status_message: "Arraste arquivos DBF, DBC, ZIP, RAR ou 7Z para iniciar.".to_string(),
            active_tab: AppTab::Files,
            worker_rx: None,
        }
    }
}

impl DbfConverterApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }

    pub fn muted_text_color(&self) -> Color32 {
        if self.theme_mode == ThemeMode::Light {
            Color32::from_rgb(71, 85, 105) // Slate 600
        } else {
            Color32::from_rgb(148, 163, 184) // Slate 400
        }
    }

    pub fn strong_text_color(&self) -> Color32 {
        if self.theme_mode == ThemeMode::Light {
            Color32::from_rgb(15, 23, 42) // Slate 900
        } else {
            Color32::from_rgb(248, 250, 252) // Slate 50
        }
    }

    /// Processa arquivos adicionados pelo usuário (analisando arquivos DBF ou descompactando ZIP/RAR/7Z)
    pub fn add_paths(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if !self.input_paths.contains(&path) {
                self.input_paths.push(path);
            }
        }
        self.reanalyze_inputs();
    }

    /// Remove um arquivo específico da lista pelo índice
    pub fn remove_path(&mut self, index: usize) {
        if index < self.input_paths.len() {
            self.input_paths.remove(index);
            if self.input_paths.is_empty() {
                self.clear_inputs();
            } else {
                self.reanalyze_inputs();
            }
        }
    }

    pub fn clear_inputs(&mut self) {
        self.input_paths.clear();
        self.inspected_dbfs.clear();
        self.invalid_archives.clear();
        self._active_temp_dirs.clear();
        self.pre_report = None;
        self.post_report = None;
        self.current_progress = None;
        self.status_message = "Pronto para receber novos arquivos.".to_string();
    }

    pub fn reanalyze_inputs(&mut self) {
        self.inspected_dbfs.clear();
        self.invalid_archives.clear();
        self._active_temp_dirs.clear();

        let mut pre_report = PreProcessingReport::new();
        pre_report.total_inputs = self.input_paths.len();

        let paths_to_process = self.input_paths.clone();
        for path in &paths_to_process {
            if !path.exists() {
                continue;
            }

            if path.is_dir() {
                // Se for diretório, varre procurando DBF e arquivos compactados
                for entry in walkdir::WalkDir::new(path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    if entry.file_type().is_file() {
                        let sub_p = entry.path().to_path_buf();
                        self.process_single_input(&sub_p, &mut pre_report);
                    }
                }
            } else {
                self.process_single_input(path, &mut pre_report);
            }
        }

        self.pre_report = Some(pre_report);
        let count = self.inspected_dbfs.len();
        let invalid = self.invalid_archives.len();

        if count > 0 {
            self.status_message = format!(
                "Análise concluída: {} base(s) DBF identificada(s){}.",
                count,
                if invalid > 0 {
                    format!(" e {} arquivo(s) compactado(s) sem DBF ignorado(s)", invalid)
                } else {
                    "".to_string()
                }
            );
        } else if invalid > 0 {
            self.status_message = format!(
                "Aviso: nenhum arquivo DBF encontrado nos {} arquivo(s) compactado(s).",
                invalid
            );
        }
    }

    fn process_single_input(&mut self, path: &Path, pre_report: &mut PreProcessingReport) {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let parent_dir = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .to_string();

        if ext == "dbf" {
            match DbfDetector::analyze_header(path, None, parent_dir, false) {
                Ok(analysis) => {
                    self.inspected_dbfs.push(analysis.clone());
                    pre_report.valid_dbfs.push(analysis);
                }
                Err(e) => {
                    pre_report.general_warnings.push(format!(
                        "Erro ao analisar DBF {:?}: {}",
                        path.file_name().unwrap_or_default(),
                        e
                    ));
                }
            }
        } else if ArchiveManager::is_archive(path) {
            match ArchiveManager::process_archive(path) {
                ArchiveExtractionResult::Success {
                    archive_name,
                    dbf_files,
                    _temp_dir,
                } => {
                    for dbf in &dbf_files {
                        if let Ok(analysis) = DbfDetector::analyze_header(
                            dbf,
                            Some(archive_name.clone()),
                            parent_dir.clone(),
                            true,
                        ) {
                            self.inspected_dbfs.push(analysis.clone());
                            pre_report.valid_dbfs.push(analysis);
                        }
                    }
                    self._active_temp_dirs.push(_temp_dir);
                }
                ArchiveExtractionResult::NoDbfFound {
                    archive_name,
                    internal_files_sample,
                } => {
                    let msg = format!(
                        "{} (Arquivos internos encontrados: {})",
                        archive_name,
                        if internal_files_sample.is_empty() {
                            "nenhum".to_string()
                        } else {
                            internal_files_sample.join(", ")
                        }
                    );
                    self.invalid_archives.push(msg.clone());
                    pre_report.invalid_archives_without_dbf.push(msg);
                }
                ArchiveExtractionResult::Error {
                    archive_name,
                    message,
                } => {
                    pre_report
                        .general_warnings
                        .push(format!("Falha no arquivo compactado {}: {}", archive_name, message));
                }
            }
        }
    }

    pub fn start_conversion(&mut self) {
        if self.inspected_dbfs.is_empty() {
            self.status_message = "Nenhum arquivo DBF válido para conversão.".to_string();
            return;
        }

        self.is_converting = true;
        self.status_message = "Iniciando processo de conversão...".to_string();

        let (tx, rx) = channel::<WorkerMessage>();
        self.worker_rx = Some(rx);

        let inspected = self.inspected_dbfs.clone();
        let invalid_archives = self.invalid_archives.clone();
        let mode = self.conversion_mode;
        let encoding_choice = self.encoding_choice;
        let output_mode = self.output_mode;
        let custom_dir = self.custom_output_dir.clone();
        let custom_filename = self.custom_output_filename.clone();

        thread::spawn(move || {
            let start_time = Instant::now();
            let mut post_report = PostProcessingReport::new(Instant::now().elapsed());
            post_report.ignored_archives_without_dbf = invalid_archives;

            let (prog_tx, prog_rx) = channel::<ProgressUpdate>();
            let worker_tx = tx.clone();

            // Thread auxiliar para repassar progresso à UI
            let bridge_handle = thread::spawn(move || {
                while let Ok(update) = prog_rx.recv() {
                    let _ = worker_tx.send(WorkerMessage::Progress(update));
                }
            });

            match output_mode {
                OutputMode::Individual => {
                    let total = inspected.len();
                    let mut total_records = 0u64;

                    for (i, dbf) in inspected.iter().enumerate() {
                        let input_path = PathBuf::from(&dbf.file_path);
                        let parent_dir = match &custom_dir {
                            Some(d) => d.clone(),
                            None => PathBuf::from(&dbf.original_dir),
                        };

                        let file_stem = Path::new(&dbf.display_name)
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_else(|| format!("saida_{}", i));

                        let out_name = if total == 1 && !custom_filename.trim().is_empty() {
                            let mut name = custom_filename.clone();
                            if !name.to_lowercase().ends_with(".parquet") {
                                name.push_str(".parquet");
                            }
                            name
                        } else {
                            // Se a pessoa não escolheu o nome, o parquet recebe exatamente o mesmo nome do DBF convertido
                            format!("{}.parquet", file_stem)
                        };

                        let output_parquet = parent_dir.join(out_name);
                        let file_encoding = encoding_choice.resolve(dbf);

                        let _ = tx.send(WorkerMessage::Progress(ProgressUpdate {
                            current_file: dbf.display_name.clone(),
                            file_index: i + 1,
                            total_files: total,
                            records_processed: 0,
                            total_records_expected: dbf.num_records as u64,
                            message: format!("Processando arquivo {} de {}: {}", i + 1, total, dbf.display_name),
                        }));

                        match ParquetConverter::convert_single(
                            &input_path,
                            &output_parquet,
                            mode,
                            file_encoding,
                            Some(&prog_tx),
                        ) {
                            Ok(summary) => {
                                total_records += summary.records;
                                post_report.successful_files.push(summary);

                                // Se veio de arquivo temporário, limpa o DBF de /tmp mantendo apenas o Parquet
                                if dbf.is_from_temp_archive {
                                    let _ = std::fs::remove_file(&input_path);
                                }
                            }
                            Err(e) => {
                                post_report.failed_files.push(format!(
                                    "{} -> Erro: {}",
                                    dbf.display_name, e
                                ));
                                if dbf.is_from_temp_archive {
                                    let _ = std::fs::remove_file(&input_path);
                                }
                            }
                        }
                    }

                    post_report.total_records_written = total_records;
                }
                OutputMode::Merged => {
                    let mut dbf_paths = Vec::new();
                    for d in &inspected {
                        dbf_paths.push(PathBuf::from(&d.file_path));
                    }

                    let parent_dir = match &custom_dir {
                        Some(d) => d.clone(),
                        None => PathBuf::from(&inspected[0].original_dir),
                    };

                    let mut out_name = custom_filename.clone();
                    if out_name.trim().is_empty() {
                        out_name = "base_vigilancia_consolidada.parquet".to_string();
                    }
                    if !out_name.to_lowercase().ends_with(".parquet") {
                        out_name.push_str(".parquet");
                    }

                    let output_parquet = parent_dir.join(out_name);
                    let merged_encoding = inspected
                        .first()
                        .map(|d| encoding_choice.resolve(d))
                        .unwrap_or(DbfEncoding::Iso8859_1);

                    match ParquetConverter::convert_merged(
                        &dbf_paths,
                        &output_parquet,
                        mode,
                        merged_encoding,
                        Some(&prog_tx),
                    ) {
                        Ok(summary) => {
                            post_report.total_records_written = summary.records;
                            post_report.successful_files.push(summary);
                        }
                        Err(e) => {
                            post_report.failed_files.push(format!(
                                "Falha na unificação dos arquivos: {}",
                                e
                            ));
                        }
                    }

                    // Limpa os DBFs temporários de /tmp mantendo apenas o Parquet gerado
                    for d in &inspected {
                        if d.is_from_temp_archive {
                            let _ = std::fs::remove_file(&d.file_path);
                        }
                    }
                }
            }

            drop(prog_tx);
            let _ = bridge_handle.join();

            let total_dur = start_time.elapsed();
            post_report.completed_at = chrono::Local::now().format("%d/%m/%Y %H:%M:%S").to_string();
            let secs = total_dur.as_secs_f64();
            post_report.total_duration = if secs < 60.0 {
                format!("{:.2} segundos", secs)
            } else {
                format!("{:.1} minutos", secs / 60.0)
            };

            let _ = tx.send(WorkerMessage::Completed(post_report));
        });
    }

    fn check_worker_messages(&mut self) {
        if let Some(rx) = &self.worker_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    WorkerMessage::Progress(update) => {
                        self.status_message = update.message.clone();
                        self.current_progress = Some(update);
                    }
                    WorkerMessage::Completed(report) => {
                        self.is_converting = false;
                        self.status_message = format!(
                            "Conversão concluída! {} registro(s) gravados em {}.",
                            report.total_records_written, report.total_duration
                        );
                        self.post_report = Some(report);
                        self.active_tab = AppTab::PostReport;
                    }
                    WorkerMessage::Error(err) => {
                        self.is_converting = false;
                        self.status_message = format!("Erro: {}", err);
                    }
                }
            }
        }
    }
}

impl eframe::App for DbfConverterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_worker_messages();

        if !self.theme_initialized {
            self.theme_mode.apply(ctx);
            self.theme_initialized = true;
        }

        if self.logo_texture.is_none() {
            let img_bytes = include_bytes!("../../docs/hex_sticker.png");
            if let Ok(dynamic_img) = image::load_from_memory(img_bytes) {
                let rgba = dynamic_img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                self.logo_texture = Some(ctx.load_texture("hex_logo", color_image, Default::default()));
            }
        }

        // Modal de Informações sobre os Padrões de Conversão e Autoria
        if let Some(modal) = self.info_modal {
            let mut is_open = true;

            if modal == InfoModalType::AboutAuthor {
                egui::Window::new("ℹ Sobre o Aplicativo e Autoria")
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut is_open)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.set_max_width(560.0);
                        ui.horizontal(|ui| {
                            if let Some(tex) = &self.logo_texture {
                                ui.add(egui::Image::new(tex).max_size(Vec2::new(60.0, 69.3)));
                            }
                            ui.vertical(|ui| {
                                ui.heading("DBF / DBC para Parquet");
                                ui.label(RichText::new("Conversor Oficial de Bases Epidemiológicas do DATASUS").italics());
                                ui.label(RichText::new("Autor: Eugênio Araújo").strong().color(Color32::from_rgb(130, 185, 255)));
                                ui.label("Centro de Inteligência Epidemiológica da SMS Rio (CIE / SVS / SMS-Rio)");
                                ui.hyperlink_to("🔗 Repositório no GitHub (Código Aberto)", "https://github.com/eugeniosinan/DBF-para-Parquet");
                            });
                        });
                        ui.separator();
                        ui.label(
                            "Objetivos e Funcionalidades:\n\n\
                            1. Conversão ultrarrápida de bases epidemiológicas legadas (DBF e DBC) para o padrão analítico Apache Parquet com compressão ZSTD.\n\
                            2. Reconhecimento automático de schemas oficiais: SIM, SINASC, SIVEP SRAG, SIVEP SG, DENGON e SINAN NET.\n\
                            3. Padronização automática de encoding conforme diretrizes oficiais do DATASUS e DuckDB (ISO-8859-1 / Windows-1252).\n\
                            4. Auditoria de dados pessoais sensíveis e anonimização de acordo com a LGPD.\n\
                            5. Descompactação direta de lotes ZIP, RAR e 7Z e descompressão transparente de arquivos DBC."
                        );
                        ui.add_space(10.0);
                        if ui.button("Fechar").clicked() {
                            self.info_modal = None;
                        }
                    });
            } else {
                let (title, content) = match modal {
                    InfoModalType::HealthPattern => (
                        "ℹ Informações — Padrão Saúde / DuckDB",
                        "Como funciona o Padrão Saúde / DuckDB:\n\n\
                        1. Conversão Inteligente de Datas:\n\
                           • Colunas de data declaradas como texto no DBF (tipo 'C'), como DTOBITO, DTNASC, DT_NOTIFIC, DT_SIN_PRI, DT_PRISINT, etc., são convertidas no formato nativo DATE (YYYY-MM-DD) do Parquet.\n\
                           • Extrai dígitos numéricos e reconhece tanto datas brasileiras (DDMMYYYY ou DD/MM/YYYY) quanto formato internacional (YYYYMMDD ou YYYY-MM-DD).\n\n\
                        2. Limpeza de Campos Textuais:\n\
                           • Todos os demais campos de texto permanecem como String UTF-8 limpa.\n\
                           • Remove espaços excedentes nas extremidades (TRIM).\n\
                           • Converte valores vazios, compostos apenas por espaços ou preenchidos com underscores ('_________') em NULL, evitando ruídos estatísticos.",
                    ),
                    InfoModalType::StrictDbf => (
                        "ℹ Informações — Padrão DBF Estrito",
                        "Como funciona o Padrão DBF Estrito:\n\n\
                        1. Fidelidade Rígida aos Tipos do Cabeçalho:\n\
                           • Respeita estritamente o tipo de cada campo conforme definido no cabeçalho binário do arquivo DBF original.\n\
                           • Campos declarados como Character ('C') são gravados como Utf8 / String sem nenhuma conversão para Date, mesmo que contenham datas.\n\
                           • Campos marcados como Date ('D') seguem como Date32.\n\
                           • Campos numéricos ('N' e 'F') seguem como inteiros (Int32/Int64) ou ponto flutuante (Float64) de acordo com o número de casas decimais informado no DBF.",
                    ),
                    InfoModalType::AboutAuthor => unreachable!(),
                };

                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut is_open)
                    .anchor(egui::Align2::CENTER_CENTER, Vec2::new(0.0, 0.0))
                    .show(ctx, |ui| {
                        ui.set_max_width(540.0);
                        ui.label(content);
                        ui.add_space(12.0);
                        if ui.button("Fechar").clicked() {
                            self.info_modal = None;
                        }
                    });
            }

            if !is_open {
                self.info_modal = None;
            }
        }

        // Detecta arrastar e soltar (drag & drop) de arquivos na janela
        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped_files.is_empty() {
            let mut paths = Vec::new();
            for f in dropped_files {
                if let Some(p) = f.path {
                    paths.push(p);
                }
            }
            if !paths.is_empty() {
                self.add_paths(paths);
            }
        }

        // Top Panel: Cabeçalho com Selo Maior à Esquerda e Título, Subtítulo e Menu ao Lado
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // Selo em tamanho generoso ocupando a altura vertical do cabeçalho
                if let Some(tex) = &self.logo_texture {
                    ui.add(egui::Image::new(tex).max_size(Vec2::new(76.0, 88.0)));
                }

                ui.add_space(8.0);

                // Coluna ao lado do selo contendo Linha 1 (Título e Controles), Linha 2 (Subtítulo) e Linha 3 (Menu em Pílula)
                ui.vertical(|ui| {
                    // Linha 1: Título principal + Seletor de Tema + Botão Sobre
                    ui.horizontal(|ui| {
                        ui.heading(RichText::new("Conversor de Bases da Vigilância — DBF/DBC para Parquet").strong().size(18.0));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("ℹ Sobre").on_hover_text("Informações sobre o sistema e autoria").clicked() {
                                self.info_modal = Some(InfoModalType::AboutAuthor);
                            }

                            // Seletor de Tema
                            let prev_theme = self.theme_mode;
                            ui.menu_button(format!("🎨 {}", self.theme_mode.display_name()), |ui| {
                                if ui.selectable_value(&mut self.theme_mode, ThemeMode::Dark, "🌙 Escuro Moderno").clicked() {
                                    ui.close();
                                }
                                if ui.selectable_value(&mut self.theme_mode, ThemeMode::DatasusBlue, "🏥 Azul Saúde").clicked() {
                                    ui.close();
                                }
                                if ui.selectable_value(&mut self.theme_mode, ThemeMode::Light, "☀ Claro Minimalista").clicked() {
                                    ui.close();
                                }
                            });
                            if self.theme_mode != prev_theme {
                                self.theme_mode.apply(ctx);
                            }
                        });
                    });

                    // Linha 2: Subtítulo oficial
                    ui.label(RichText::new("Padronização Oficial DATASUS • SIM, SINASC, SIVEP, DENGON, SINAN NET").color(self.muted_text_color()).size(12.0));
                    ui.add_space(4.0);

                    // Linha 3: Menu em Pílula (Segmented Control)
                    ui.horizontal(|ui| {
                        egui::Frame::new()
                            .fill(ui.visuals().widgets.inactive.bg_fill)
                            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                            .corner_radius(egui::CornerRadius::same(8))
                            .inner_margin(egui::Margin::symmetric(4, 3))
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

                                let b1 = ui.selectable_label(self.active_tab == AppTab::Files, "📁 1. Arquivos");
                                if b1.clicked() { self.active_tab = AppTab::Files; }

                                let pre_label = format!("🔍 2. Auditoria Prévia ({})", self.inspected_dbfs.len());
                                let b2 = ui.selectable_label(self.active_tab == AppTab::PreReport, pre_label);
                                if b2.clicked() { self.active_tab = AppTab::PreReport; }

                                let b_merge = ui.selectable_label(self.active_tab == AppTab::Merge, "🧩 3. Juntar Arquivos");
                                if b_merge.clicked() { self.active_tab = AppTab::Merge; }

                                let b_cfg = ui.selectable_label(self.active_tab == AppTab::Settings, "⚙ 4. Configurações");
                                if b_cfg.clicked() { self.active_tab = AppTab::Settings; }

                                if self.post_report.is_some() {
                                    let b_post = ui.selectable_label(self.active_tab == AppTab::PostReport, "✅ 5. Relatório");
                                    if b_post.clicked() { self.active_tab = AppTab::PostReport; }
                                }
                            });
                    });
                });
            });
            ui.add_space(6.0);
        });

        // Bottom Panel: Status, Progresso, Botão Fixo de Conversão e Autoria Exclusiva no Rodapé
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            if self.is_converting {
                if let Some(prog) = &self.current_progress {
                    let fraction = if prog.total_records_expected > 0 {
                        (prog.records_processed as f32 / prog.total_records_expected as f32).clamp(0.0, 1.0)
                    } else {
                        0.0
                    };
                    ui.add(egui::ProgressBar::new(fraction).show_percentage().animate(true));
                } else {
                    ui.add(egui::ProgressBar::new(0.0).animate(true));
                }
                ui.add_space(4.0);
            }

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Status:").strong().color(self.muted_text_color()));
                        ui.label(RichText::new(&self.status_message).color(self.strong_text_color()));
                    });
                    ui.label(RichText::new("Eugênio Araújo • Centro de Inteligência Epidemiológica da SMS Rio (CIE / SVS / SMS-Rio)").small().strong().color(self.muted_text_color()));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_convert = !self.is_converting && !self.input_paths.is_empty();
                    let btn_text = if self.is_converting {
                        "⏳ Convertendo...".to_string()
                    } else if self.input_paths.is_empty() {
                        "🚀 Iniciar Conversão (0 arquivos)".to_string()
                    } else if self.output_mode == OutputMode::Merged {
                        format!("🧩 Juntar e Converter {} Base(s)", self.inspected_dbfs.len())
                    } else {
                        format!("🚀 Converter {} Base(s) em Parquet", self.inspected_dbfs.len())
                    };

                    let convert_btn = ui.add_enabled(
                        can_convert,
                        egui::Button::new(RichText::new(btn_text).strong().size(14.5).color(Color32::WHITE))
                            .fill(if can_convert {
                                Color32::from_rgb(16, 185, 129) // Emerald 500
                            } else {
                                Color32::from_rgb(100, 116, 139) // Slate 500
                            })
                            .min_size(Vec2::new(280.0, 40.0))
                            .corner_radius(egui::CornerRadius::same(8)),
                    );

                    if convert_btn.clicked() {
                        self.start_conversion();
                    }
                });
            });
            ui.add_space(6.0);
        });

        // Central Panel: Conteúdo da Aba ativa
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.active_tab {
                AppTab::Files => self.ui_files(ui),
                AppTab::PreReport => self.ui_pre_report(ui),
                AppTab::Merge => self.ui_merge(ui),
                AppTab::Settings => self.ui_settings(ui),
                AppTab::PostReport => self.ui_post_report(ui),
            }
        });

        if self.is_converting {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }
}

impl DbfConverterApp {
    fn ui_files(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);

            // ==========================================
            // ETAPA 1: SELEÇÃO E DROPZONE DE ARQUIVOS
            // ==========================================
            ui_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(" 1 ").strong().color(Color32::WHITE).background_color(Color32::from_rgb(37, 99, 235)));
                    ui.label(RichText::new("Seleção de Arquivos e Pastas da Vigilância").strong().size(15.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !self.input_paths.is_empty() {
                            if ui.button(RichText::new("🗑 Limpar Tudo").color(Color32::from_rgb(239, 68, 68))).on_hover_text("Limpa todos os arquivos da lista").clicked() {
                                self.clear_inputs();
                            }
                        }
                        if ui.button("📂 Adicionar Pasta...").on_hover_text("Varre recursivamente a pasta selecionada, localizando e descompactando todos os DBF, DBC e compactados (ZIP/RAR/7Z).").clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                self.add_paths(vec![dir]);
                            }
                        }
                        if ui.button("📄 Adicionar Arquivos...").clicked() {
                            if let Some(files) = rfd::FileDialog::new()
                                .add_filter(
                                    "Bases e Compactados (*.dbf, *.dbc, *.zip, *.7z, *.rar)",
                                    &[
                                        "dbf", "DBF", "Dbf",
                                        "dbc", "DBC", "Dbc",
                                        "zip", "ZIP", "Zip",
                                        "7z", "7Z",
                                        "rar", "RAR", "Rar",
                                    ],
                                )
                                .add_filter(
                                    "DBF / DBC (DATASUS) (*.dbf, *.dbc)",
                                    &["dbf", "DBF", "Dbf", "dbc", "DBC", "Dbc"],
                                )
                                .add_filter(
                                    "Compactados (*.zip, *.7z, *.rar, *.dbc)",
                                    &[
                                        "zip", "ZIP", "Zip",
                                        "7z", "7Z",
                                        "rar", "RAR", "Rar",
                                        "dbc", "DBC", "Dbc",
                                    ],
                                )
                                .add_filter("Todos os Arquivos (*.*)", &["*"])
                                .pick_files()
                            {
                                self.add_paths(files);
                            }
                        }
                    });
                });

                ui.add_space(8.0);

                // Dropzone interativa com alto contraste para temas Claro e Escuro
                let drop_height = 84.0;
                let (drop_rect, drop_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), drop_height), egui::Sense::hover());
                let is_hovered = drop_resp.hovered();

                let is_light = self.theme_mode == ThemeMode::Light;
                let (border_color, bg_color, text_color, subtext_color) = if is_light {
                    if is_hovered {
                        (
                            Color32::from_rgb(37, 99, 235), // Blue 600
                            Color32::from_rgb(239, 246, 255), // Blue 50
                            Color32::from_rgb(30, 58, 138), // Blue 900
                            Color32::from_rgb(71, 85, 105), // Slate 600
                        )
                    } else {
                        (
                            Color32::from_rgb(147, 197, 253), // Blue 300
                            Color32::from_rgb(248, 250, 252), // Slate 50
                            Color32::from_rgb(15, 23, 42), // Slate 900 (preto carvão nítido)
                            Color32::from_rgb(71, 85, 105), // Slate 600
                        )
                    }
                } else {
                    if is_hovered {
                        (
                            Color32::from_rgb(96, 165, 250),
                            Color32::from_rgba_premultiplied(37, 99, 235, 40),
                            Color32::WHITE,
                            Color32::from_rgb(186, 230, 253),
                        )
                    } else {
                        (
                            Color32::from_rgb(51, 65, 85),
                            Color32::from_rgba_premultiplied(15, 23, 42, 120),
                            Color32::from_rgb(226, 232, 240),
                            Color32::from_rgb(148, 163, 184),
                        )
                    }
                };

                let painter = ui.painter();
                painter.rect(drop_rect, 8.0, bg_color, Stroke::new(1.5_f32, border_color), egui::StrokeKind::Inside);

                let label_text = if self.input_paths.is_empty() {
                    "⬇ Arraste e solte arquivos aqui (.dbf, .dbc, .zip, .rar, .7z) ou pastas completas"
                } else {
                    "⬇ Arraste mais arquivos ou pastas para adicionar à lista"
                };
                painter.text(
                    drop_rect.center() - Vec2::new(0.0, 11.0),
                    egui::Align2::CENTER_CENTER,
                    label_text,
                    egui::FontId::proportional(14.0),
                    text_color,
                );
                painter.text(
                    drop_rect.center() + Vec2::new(0.0, 13.0),
                    egui::Align2::CENTER_CENTER,
                    "Descompressão transparente de DBC e extração direta de pacotes compactados",
                    egui::FontId::proportional(11.5),
                    subtext_color,
                );
            });

            ui.add_space(8.0);

            // ==========================================
            // LISTA DE ARQUIVOS ADICIONADOS COM BOTÃO DE REMOVER
            // ==========================================
            ui_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Arquivos Selecionados").strong().size(14.5));
                    ui.label(
                        RichText::new(format!(
                            "({} item(s) • {} base(s) DBF identificada(s))",
                            self.input_paths.len(),
                            self.inspected_dbfs.len()
                        ))
                        .color(self.muted_text_color()),
                    );

                    if !self.inspected_dbfs.is_empty() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🔍 Ver Auditoria Prévia Detalhada ➔").clicked() {
                                self.active_tab = AppTab::PreReport;
                            }
                        });
                    }
                });

                ui.add_space(6.0);

                if self.input_paths.is_empty() {
                    ui.label(
                        RichText::new("Nenhum arquivo adicionado ainda. Arraste arquivos na caixa acima ou use os botões de adicionar.")
                            .italics()
                            .color(self.muted_text_color()),
                    );
                } else {
                    let mut path_to_remove = None;

                    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                        egui::Grid::new("files_list_grid").striped(true).spacing(Vec2::new(12.0, 6.0)).show(ui, |ui| {
                            ui.label(RichText::new("Item / Origem").strong().color(self.muted_text_color()));
                            ui.label(RichText::new("Tamanho").strong().color(self.muted_text_color()));
                            ui.label(RichText::new("Ação").strong().color(self.muted_text_color()));
                            ui.end_row();

                            for (idx, p) in self.input_paths.iter().enumerate() {
                                let fname = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| p.to_string_lossy().to_string());
                                let fmeta = std::fs::metadata(p);
                                let size_str = if let Ok(m) = fmeta {
                                    if m.is_dir() {
                                        "Pasta".to_string()
                                    } else {
                                        format!("{:.2} MB", m.len() as f64 / (1024.0 * 1024.0))
                                    }
                                } else {
                                    "-".to_string()
                                };

                                ui.label(RichText::new(&fname).strong());
                                ui.label(size_str);

                                if ui.button(RichText::new("✕ Remover").color(Color32::from_rgb(239, 68, 68)).strong()).on_hover_text("Remover este item da lista").clicked() {
                                    path_to_remove = Some(idx);
                                }
                                ui.end_row();
                            }
                        });
                    });

                    if let Some(idx) = path_to_remove {
                        self.remove_path(idx);
                    }
                }
            });

            ui.add_space(8.0);

            // ==========================================
            // PASTA DE DESTINO DOS ARQUIVOS PARQUET
            // ==========================================
            ui_card(ui, |ui| {
                ui.label(RichText::new("Pasta de Destino dos Arquivos Parquet").strong().size(14.5));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    let dir_display = self
                        .custom_output_dir
                        .as_ref()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Mesma pasta de origem de cada arquivo (Padrão)".to_string());
                    ui.label(RichText::new(dir_display).monospace());

                    if ui.button("📂 Alterar Pasta...").clicked() {
                        if let Some(d) = rfd::FileDialog::new().pick_folder() {
                            self.custom_output_dir = Some(d);
                        }
                    }
                    if self.custom_output_dir.is_some() && ui.button("Restaurar Padrão").clicked() {
                        self.custom_output_dir = None;
                    }
                });

                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "💡 Modo de conversão padrão: Individual (cada base gera seu respectivo .parquet com compressão ZSTD).
                        Para mesclar múltiplos arquivos em uma única tabela consolidada, acesse a aba '🧩 3. Juntar Arquivos'."
                    )
                    .small()
                    .color(self.muted_text_color()),
                );
            });

            ui.add_space(8.0);
        });
    }

    fn ui_merge(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);

            ui_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(" 3 ").strong().color(Color32::WHITE).background_color(Color32::from_rgb(37, 99, 235)));
                    ui.label(RichText::new("Mesclagem Consolidada de Arquivos (Merge)").strong().size(15.5));
                });

                ui.add_space(8.0);

                // Card educativo em destaque sobre o requisito obrigatório de mesmo schema
                let is_light = self.theme_mode == ThemeMode::Light;
                let warn_bg = if is_light {
                    Color32::from_rgb(254, 243, 199) // Amber 100
                } else {
                    Color32::from_rgb(45, 34, 15)
                };
                let warn_border = Color32::from_rgb(245, 158, 11); // Amber 500
                let warn_text = if is_light {
                    Color32::from_rgb(120, 53, 15) // Amber 900
                } else {
                    Color32::from_rgb(253, 230, 138)
                };

                egui::Frame::new()
                    .fill(warn_bg)
                    .stroke(Stroke::new(1.0_f32, warn_border))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(RichText::new("⚠️ REQUISITO OBRIGATÓRIO PARA A MESCLAGEM:").strong().color(warn_border));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                "Para juntar múltiplos arquivos DBF/DBC em um único arquivo Parquet consolidado, todas as bases selecionadas                                 DEVEM possuir exatamente a mesma estrutura de colunas (mesmo schema) e pertencer ao mesmo                                 sistema de vigilância epidemiológica (exemplo: apenas arquivos de SIM Óbitos, ou apenas SINASC Nascidos, ou apenas SIVEP-Gripe).

                                Bases de sistemas diferentes ou com colunas divergentes devem ser convertidas individualmente na aba '📁 1. Arquivos'."
                            )
                            .size(12.5)
                            .color(warn_text),
                        );
                    });

                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Modo de Saída da Conversão:").strong());
                    ui.radio_value(&mut self.output_mode, OutputMode::Individual, "Individual (1 Parquet por arquivo)");
                    ui.radio_value(&mut self.output_mode, OutputMode::Merged, "🧩 Juntar Arquivos em 1 Parquet Consolidado");
                });

                if self.output_mode == OutputMode::Merged {
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Nome do Arquivo Consolidado:").strong());
                        ui.text_edit_singleline(&mut self.custom_output_filename);
                    });
                }

                ui.add_space(12.0);
                ui.label(RichText::new(format!("Bases que farão parte da mesclagem ({}):", self.inspected_dbfs.len())).strong());
                ui.add_space(4.0);

                if self.inspected_dbfs.is_empty() {
                    ui.label(
                        RichText::new("Nenhuma base identificada ainda. Adicione arquivos na aba '📁 1. Arquivos'.")
                            .italics()
                            .color(self.muted_text_color()),
                    );
                } else {
                    for (i, dbf) in self.inspected_dbfs.iter().enumerate() {
                        ui.label(format!("{}. {} (Sistema: {}, {} colunas, {} registros)", i + 1, dbf.display_name, dbf.system.code(), dbf.num_columns, dbf.num_records));
                    }
                }
            });

            ui.add_space(8.0);
        });
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);

            // CARD 1: TRATAMENTO DE DATAS DA VIGILÂNCIA
            ui_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(" ⚙ ").strong().color(Color32::WHITE).background_color(Color32::from_rgb(37, 99, 235)));
                    ui.label(RichText::new("Tratamento de Datas da Vigilância Epidemiológica").strong().size(15.0));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("ℹ Padrão Saúde vs Estrito").clicked() {
                            self.info_modal = Some(InfoModalType::HealthPattern);
                        }
                    });
                });

                ui.add_space(8.0);
                ui.radio_value(
                    &mut self.conversion_mode,
                    ConversionMode::EpidemiologicalDates,
                    "Padrão Saúde / DuckDB (Recomendado): Normaliza campos de datas D8 (DTOBITO, DTNASC, DTNOTIF...) para o tipo Date32 ISO do Apache Arrow e enriquece com Ano e Semana Epidemiológica.",
                );
                ui.add_space(4.0);
                ui.radio_value(
                    &mut self.conversion_mode,
                    ConversionMode::StrictDbfTypes,
                    "Padrão DBF Estrito: Mantém exatamente a representação de tipos original do xBase sem conversão para Date32 do Arrow.",
                );
            });

            ui.add_space(8.0);

            // CARD 2: CODIFICAÇÃO DE CARACTERES (ENCODING)
            ui_card(ui, |ui| {
                ui.label(RichText::new("Codificação de Caracteres (Encoding)").strong().size(15.0));
                ui.add_space(6.0);

                ui.radio_value(
                    &mut self.encoding_choice,
                    EncodingChoice::AutomaticStandard,
                    "Automático por Agravo (Recomendado): Aplica o padrão oficial de cada sistema DATASUS reconhecido (CP850 / ISO-8859-1 para SIM/SINASC/Dengue e UTF-8 para SIVEP).",
                );
                ui.add_space(4.0);
                ui.radio_value(&mut self.encoding_choice, EncodingChoice::Cp850, "Forçar CP850 (IBM 850 / DOS Latino-1 clássico DATASUS)");
                ui.add_space(4.0);
                ui.radio_value(&mut self.encoding_choice, EncodingChoice::Iso8859_1, "Forçar ISO-8859-1 (Latin-1)");
                ui.add_space(4.0);
                ui.radio_value(&mut self.encoding_choice, EncodingChoice::Utf8, "Forçar UTF-8 (Padrão moderno / SIVEP Web)");
            });

            ui.add_space(8.0);

            // CARD 3: FORMATO PARQUET E COMPRESSÃO
            ui_card(ui, |ui| {
                ui.label(RichText::new("Compressão e Otimização do Apache Parquet").strong().size(15.0));
                ui.add_space(6.0);
                ui.label("• Algoritmo de Compressão: ZSTD Nível 3 (Padrão ouro de performance: descompressão em gigabytes por segundo com taxa de compressão superior ao GZIP).");
                ui.label("• Sanitização de Strings: Remoção automática de espaços em branco à direita (trailing spaces) característicos do formato DBF.");
            });

            ui.add_space(8.0);
        });
    }

    fn ui_pre_report(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("🔍 Relatório Pré-Processamento (Análise das Bases da Vigilância)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📋 Voltar às Configurações").clicked() {
                        self.active_tab = AppTab::Files;
                    }
                });
            });

            ui.separator();

            if let Some(report) = &self.pre_report {
                // Alertas de arquivos compactados sem DBF
                if !report.invalid_archives_without_dbf.is_empty() {
                    ui_card(ui, |ui| {
                        ui.label(
                            RichText::new("⚠ Arquivos Compactados Sem Base DBF Detectada:")
                                .color(Color32::from_rgb(255, 160, 50))
                                .strong(),
                        );
                        for inv in &report.invalid_archives_without_dbf {
                            ui.label(format!("• {}", inv));
                        }
                    });
                    ui.add_space(8.0);
                }

                if report.valid_dbfs.is_empty() {
                    ui.label("Nenhum arquivo DBF pronto para análise.");
                    return;
                }

                ui.label(RichText::new(format!("Total de bases DBF prontas para conversão: {}", report.valid_dbfs.len())).strong());
                ui.add_space(6.0);

                ui_card(ui, |ui| {
                    egui::Grid::new("pre_report_grid").striped(true).spacing(Vec2::new(16.0, 8.0)).show(ui, |ui| {
                        ui.label(RichText::new("Arquivo").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Base Identificada").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Correspondência").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Encoding Padronizado").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Privacidade LGPD").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Registros").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Tamanho").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.end_row();

                        for item in &report.valid_dbfs {
                            let fname = if let Some(a) = &item.source_archive {
                                format!("{} (em {})", item.display_name, a)
                            } else {
                                item.display_name.clone()
                            };
                            ui.label(fname);

                            let sys_color = match item.system {
                                HealthSystem::SIM => Color32::from_rgb(70, 160, 240),
                                HealthSystem::SINASC => Color32::from_rgb(100, 200, 120),
                                HealthSystem::SivepSrag => Color32::from_rgb(230, 160, 60),
                                HealthSystem::SivepSg => Color32::from_rgb(200, 130, 220),
                                HealthSystem::Dengon => Color32::from_rgb(80, 210, 210),
                                _ => Color32::LIGHT_GRAY,
                            };
                            ui.label(RichText::new(item.system.code()).color(sys_color).strong());
                            ui.label(format!("{:.1}% ({}/{})", item.match_percentage, item.matched_columns_count, item.num_columns));
                            ui.label(RichText::new(item.standard_encoding.display_name()).color(Color32::from_rgb(120, 200, 140)));

                            let priv_color = match item.privacy {
                                PrivacyLevel::WithPersonalData => Color32::from_rgb(240, 120, 80),
                                PrivacyLevel::WithoutPersonalData => Color32::from_rgb(100, 200, 120),
                            };
                            ui.label(RichText::new(item.privacy.display_name()).color(priv_color));

                            ui.label(item.num_records.to_string());
                            ui.label(format!("{:.2} MB", item.file_size_bytes as f64 / (1024.0 * 1024.0)));
                            ui.end_row();
                        }
                    });
                });

                ui.add_space(12.0);
                ui.collapsing("Exibir Detalhes de Colunas e Auditoria", |ui| {
                    for item in &report.valid_dbfs {
                        ui.add_space(6.0);
                        ui_card(ui, |ui| {
                            ui.label(RichText::new(&item.display_name).strong());
                            ui.label(format!("Sistema: {}", item.system.display_name()));
                            ui.label(format!(
                                "Correspondência com o Schema Oficial: {:.1}% ({} de {} colunas pertencem ao dicionário de {} colunas do sistema)",
                                item.match_percentage, item.matched_columns_count, item.num_columns, item.total_schema_columns
                            ));
                            ui.label(format!(
                                "Encoding Padronizado: {} ({})",
                                item.standard_encoding.display_name(), item.encoding_rationale
                            ));
                            if !item.key_columns_found.is_empty() {
                                ui.label(format!("Colunas de assinatura encontradas: {}", item.key_columns_found.join(", ")));
                            }
                            if !item.personal_columns_found.is_empty() {
                                ui.label(RichText::new(format!(
                                    "Dados pessoais do paciente/cidadão detectados: {}",
                                    item.personal_columns_found.join(", ")
                                )).color(Color32::from_rgb(240, 120, 80)));
                            } else {
                                ui.label("Dados pessoais: Nenhum campo nominal ou de endereço do paciente detectado (Base Anonimizada / Disseminação Pública).");
                            }
                            ui.label(format!("Colunas presentes no DBF: {}", item.column_names.join(", ")));
                        });
                    }
                });
            } else {
                ui.label("Adicione arquivos para gerar a análise prévia.");
            }
        });
    }

    fn ui_post_report(&mut self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("✅ Relatório Pós-Processamento");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Nova Conversão").clicked() {
                        self.active_tab = AppTab::Files;
                    }
                });
            });

            ui.separator();

            if let Some(rep) = &self.post_report {
                ui_card(ui, |ui| {
                    ui.label(RichText::new(format!("Finalizado em: {}", rep.completed_at)).italics());
                    ui.label(RichText::new(format!("Duração total: {}", rep.total_duration)).strong());
                    ui.label(RichText::new(format!("Total de registros gravados: {}", rep.total_records_written)).size(16.0).color(Color32::from_rgb(52, 211, 153)));
                });

                ui.add_space(8.0);

                if !rep.ignored_archives_without_dbf.is_empty() {
                    ui_card(ui, |ui| {
                        ui.label(RichText::new("⚠ Arquivos compactados sem DBF que foram ignorados:").color(Color32::from_rgb(240, 160, 50)));
                        for f in &rep.ignored_archives_without_dbf {
                            ui.label(format!("• {}", f));
                        }
                    });
                    ui.add_space(8.0);
                }

                if !rep.failed_files.is_empty() {
                    ui_card(ui, |ui| {
                        ui.label(RichText::new("❌ Erros encontrados:").color(Color32::from_rgb(240, 80, 80)));
                        for err in &rep.failed_files {
                            ui.label(err);
                        }
                    });
                    ui.add_space(8.0);
                }

                ui.label(RichText::new("Arquivos Parquet Gerados:").strong());
                ui.add_space(6.0);
                ui_card(ui, |ui| {
                    egui::Grid::new("post_report_grid").striped(true).spacing(Vec2::new(16.0, 8.0)).show(ui, |ui| {
                        ui.label(RichText::new("Arquivo de Saída").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Registros").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Tamanho").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.label(RichText::new("Tempo").strong().color(Color32::from_rgb(148, 163, 184)));
                        ui.end_row();

                        for item in &rep.successful_files {
                            ui.label(&item.output_path);
                            ui.label(item.records.to_string());
                            ui.label(format!("{:.2} MB", item.file_size_bytes as f64 / (1024.0 * 1024.0)));
                            ui.label(format!("{:.2}s", item.duration_ms as f64 / 1000.0));
                            ui.end_row();
                        }
                    });
                });

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if let Some(first) = rep.successful_files.first() {
                        if let Some(parent) = Path::new(&first.output_path).parent() {
                            let p_str = parent.to_string_lossy().to_string();
                            if ui.button("📂 Abrir Pasta dos Arquivos").clicked() {
                                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                            }
                            ui.label(format!("Local: {}", p_str));
                        }
                    }
                });
            }
        });
    }
}
