use biome_analyze::AnalyzerConfiguration;
use biome_analyze::AnalyzerOptions;
use biome_formatter::IndentStyle;
use biome_formatter::IndentWidth;
use biome_formatter::LineEnding;
use biome_formatter::LineWidth;
use biome_formatter::Printed;
use biome_fs::BiomePath;
use biome_parser::AnyParse;
use biome_r_formatter::context::RFormatOptions;
use biome_r_formatter::format_node;
use biome_r_parser::RParserOptions;
use biome_r_syntax::RLanguage;
use biome_rowan::NodeCache;

use crate::file_handlers::AnalyzerCapabilities;
use crate::file_handlers::DebugCapabilities;
use crate::file_handlers::ExtensionHandler;
use crate::file_handlers::FormatterCapabilities;
use crate::file_handlers::ParseResult;
use crate::file_handlers::ParserCapabilities;
use crate::file_handlers::SearchCapabilities;
use crate::settings::FormatSettings;
use crate::settings::LanguageListSettings;
use crate::settings::LanguageSettings;
use crate::settings::LinterSettings;
use crate::settings::OverrideSettings;
use crate::settings::ServiceLanguage;
use crate::settings::Settings;
use crate::settings::WorkspaceSettingsHandle;
use crate::workspace::DocumentFileSource;
use crate::WorkspaceError;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RFormatterSettings {
    pub line_ending: Option<LineEnding>,
    pub line_width: Option<LineWidth>,
    pub indent_width: Option<IndentWidth>,
    pub indent_style: Option<IndentStyle>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RParserSettings {}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RLinterSettings {
    pub enabled: Option<bool>,
}

impl ServiceLanguage for RLanguage {
    type FormatterSettings = RFormatterSettings;
    type LinterSettings = RLinterSettings;
    type OrganizeImportsSettings = ();
    type FormatOptions = RFormatOptions;
    type ParserSettings = RParserSettings;
    type EnvironmentSettings = ();

    fn lookup_settings(language: &LanguageListSettings) -> &LanguageSettings<Self> {
        &language.r
    }

    fn resolve_format_options(
        global: Option<&FormatSettings>,
        overrides: Option<&OverrideSettings>,
        language: Option<&RFormatterSettings>,
        path: &BiomePath,
        _document_file_source: &DocumentFileSource,
    ) -> Self::FormatOptions {
        let indent_style = language
            .and_then(|l| l.indent_style)
            .or(global.and_then(|g| g.indent_style))
            .unwrap_or_default();
        let line_width = language
            .and_then(|l| l.line_width)
            .or(global.and_then(|g| g.line_width))
            .unwrap_or_default();
        let indent_width = language
            .and_then(|l| l.indent_width)
            .or(global.and_then(|g| g.indent_width))
            .unwrap_or_default();

        let line_ending = language
            .and_then(|l| l.line_ending)
            .or(global.and_then(|g| g.line_ending))
            .unwrap_or_default();

        let options = RFormatOptions::new()
            .with_line_ending(line_ending)
            .with_indent_style(indent_style)
            .with_indent_width(indent_width)
            .with_line_width(line_width);

        if let Some(overrides) = overrides {
            overrides.to_override_r_format_options(path, options)
        } else {
            options
        }
    }

    fn resolve_analyzer_options(
        _global: Option<&Settings>,
        _linter: Option<&LinterSettings>,
        _overrides: Option<&OverrideSettings>,
        _language: Option<&Self::LinterSettings>,
        path: &BiomePath,
        _file_source: &DocumentFileSource,
    ) -> AnalyzerOptions {
        AnalyzerOptions {
            configuration: AnalyzerConfiguration::default(),
            file_path: path.to_path_buf(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RFileHandler;

impl ExtensionHandler for RFileHandler {
    fn capabilities(&self) -> super::Capabilities {
        super::Capabilities {
            parser: ParserCapabilities { parse: Some(parse) },
            debug: DebugCapabilities {
                debug_syntax_tree: None,
                debug_control_flow: None,
                debug_formatter_ir: None,
            },
            analyzer: AnalyzerCapabilities {
                lint: None,
                code_actions: None,
                fix_all: None,
                rename: None,
                organize_imports: None,
            },
            formatter: FormatterCapabilities {
                format: Some(format),
                format_range: None,
                format_on_type: None,
            },
            search: SearchCapabilities { search: None },
        }
    }
}

fn parse(
    biome_path: &BiomePath,
    _file_source: DocumentFileSource,
    text: &str,
    settings: Option<&Settings>,
    cache: &mut NodeCache,
) -> ParseResult {
    let mut options = RParserOptions {};
    if let Some(settings) = settings {
        options = settings
            .override_settings
            .to_override_r_parser_options(biome_path, options);
    }

    let parse = biome_r_parser::parse_r_with_cache(text, options, cache);
    ParseResult {
        any_parse: parse.into(),
        language: None,
    }
}

#[tracing::instrument(level = "trace", skip(parse, settings))]
pub(crate) fn format(
    biome_path: &BiomePath,
    document_file_source: &DocumentFileSource,
    parse: AnyParse,
    settings: WorkspaceSettingsHandle,
) -> Result<Printed, WorkspaceError> {
    let options = settings.format_options::<RLanguage>(biome_path, document_file_source);

    tracing::debug!("Options used for format: \n{}", options);

    let tree = parse.syntax();
    tracing::info!("Format file {}", biome_path.display());
    let formatted = format_node(options, &tree)?;
    match formatted.print() {
        Ok(printed) => Ok(printed),
        Err(error) => {
            tracing::error!("The file {} couldn't be formatted", biome_path.display());
            Err(WorkspaceError::FormatError(error.into()))
        }
    }
}
