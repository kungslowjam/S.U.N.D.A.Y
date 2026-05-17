//! Built-in tool implementations.

pub mod academic_search;
pub mod apply_patch;
pub mod browser_tools;
pub mod calculator;
pub mod code_analyzer;
pub mod crypto;
pub mod directory_tools;
pub mod file_tools;
pub mod git_tools;
pub mod grep;
pub mod hermes_tools;
pub mod http_tools;
pub mod memory_tools;
pub mod office_tools;
pub mod planner;
pub mod repo_map;
pub mod system_health;
pub mod web_fetch;
pub mod web_search;
pub mod workspace_tools;
pub mod shell;
pub mod think;
pub mod scan_chunks;

pub use academic_search::{SemanticScholarSearchTool, ArxivSearchTool, OpenAlexSearchTool};
pub use apply_patch::ApplyPatchTool;
pub use browser_tools::{BrowserNavigateTool, BrowserScreenshotTool, BrowserClickTool, BrowserTypeTool, BrowserViewTreeTool};
pub use code_analyzer::CodeAnalyzerTool;
pub use hermes_tools::{DelegateBrowserTool, DelegateResearchTool};
pub use calculator::CalculatorTool;
pub use crypto::CryptoPriceTool;
pub use directory_tools::ListDirectoryTool;
pub use file_tools::{FileEditTool, FileReadTool, FileWriteTool, FileReadMultipleTool};
pub use git_tools::{GitCommitTool, GitDiffTool, GitLogTool, GitStatusTool};
pub use grep::FileGrepTool;
pub use system_health::SystemHealthTool;
pub use web_fetch::WebFetchTool;
pub use http_tools::HttpRequestTool;
pub use memory_tools::{MemorySearchTool, MemoryStoreTool};
pub use office_tools::{WordWriteTool, ExcelReadTool};
pub use planner::TaskPlannerTool;
pub use repo_map::RepoMapTool;
pub use web_search::WebSearchTool;
pub use workspace_tools::ProjectWorkspaceTool;
pub use shell::ShellExecTool;
pub use think::ThinkTool;
pub use scan_chunks::ScanChunksTool;

use crate::traits::BaseTool;
use sunday_core::{ToolResult, ToolSpec};
use serde_json::Value;

pub enum BuiltinTool {
    SemanticScholarSearch(academic_search::SemanticScholarSearchTool),
    ArxivSearch(academic_search::ArxivSearchTool),
    OpenAlexSearch(academic_search::OpenAlexSearchTool),
    ApplyPatch(apply_patch::ApplyPatchTool),
    CodeAnalyzer(code_analyzer::CodeAnalyzerTool),
    BrowserNavigate(browser_tools::BrowserNavigateTool),
    BrowserScreenshot(browser_tools::BrowserScreenshotTool),
    BrowserClick(browser_tools::BrowserClickTool),
    BrowserType(browser_tools::BrowserTypeTool),
    BrowserViewTree(browser_tools::BrowserViewTreeTool),
    Calculator(calculator::CalculatorTool),
    Think(think::ThinkTool),
    FileEdit(file_tools::FileEditTool),
    FileRead(file_tools::FileReadTool),
    FileReadMultiple(file_tools::FileReadMultipleTool),
    FileWrite(file_tools::FileWriteTool),
    FileGrep(grep::FileGrepTool),
    ListDirectory(directory_tools::ListDirectoryTool),
    ShellExec(shell::ShellExecTool),
    HttpRequest(http_tools::HttpRequestTool),
    WebSearch(web_search::WebSearchTool),
    GitStatus(git_tools::GitStatusTool),
    GitDiff(git_tools::GitDiffTool),
    GitLog(git_tools::GitLogTool),
    GitCommit(git_tools::GitCommitTool),
    DelegateBrowser(hermes_tools::DelegateBrowserTool),
    DelegateResearch(hermes_tools::DelegateResearchTool),
    CryptoPrice(crypto::CryptoPriceTool),
    WordWrite(office_tools::WordWriteTool),
    ExcelRead(office_tools::ExcelReadTool),
    ProjectCreate(workspace_tools::ProjectWorkspaceTool),
    MemorySearch(memory_tools::MemorySearchTool),
    MemoryStore(memory_tools::MemoryStoreTool),
    WebFetch(web_fetch::WebFetchTool),
    TaskPlanner(planner::TaskPlannerTool),
    RepoMap(repo_map::RepoMapTool),
    SystemHealth(system_health::SystemHealthTool),
    ScanChunks(scan_chunks::ScanChunksTool),
}

macro_rules! delegate_tool {
    ($self:expr, $method:ident $(, $arg:expr)*) => {
        match $self {
            BuiltinTool::SemanticScholarSearch(t) => t.$method($($arg),*),
            BuiltinTool::ArxivSearch(t) => t.$method($($arg),*),
            BuiltinTool::OpenAlexSearch(t) => t.$method($($arg),*),
            BuiltinTool::ApplyPatch(t) => t.$method($($arg),*),
            BuiltinTool::CodeAnalyzer(t) => t.$method($($arg),*),
            BuiltinTool::BrowserNavigate(t) => t.$method($($arg),*),
            BuiltinTool::BrowserScreenshot(t) => t.$method($($arg),*),
            BuiltinTool::BrowserClick(t) => t.$method($($arg),*),
            BuiltinTool::BrowserType(t) => t.$method($($arg),*),
            BuiltinTool::BrowserViewTree(t) => t.$method($($arg),*),
            BuiltinTool::Calculator(t) => t.$method($($arg),*),
            BuiltinTool::Think(t) => t.$method($($arg),*),
            BuiltinTool::FileEdit(t) => t.$method($($arg),*),
            BuiltinTool::FileRead(t) => t.$method($($arg),*),
            BuiltinTool::FileReadMultiple(t) => t.$method($($arg),*),
            BuiltinTool::FileWrite(t) => t.$method($($arg),*),
            BuiltinTool::FileGrep(t) => t.$method($($arg),*),
            BuiltinTool::ListDirectory(t) => t.$method($($arg),*),
            BuiltinTool::ShellExec(t) => t.$method($($arg),*),
            BuiltinTool::HttpRequest(t) => t.$method($($arg),*),
            BuiltinTool::WebSearch(t) => t.$method($($arg),*),
            BuiltinTool::GitStatus(t) => t.$method($($arg),*),
            BuiltinTool::GitDiff(t) => t.$method($($arg),*),
            BuiltinTool::GitLog(t) => t.$method($($arg),*),
            BuiltinTool::GitCommit(t) => t.$method($($arg),*),
            BuiltinTool::DelegateBrowser(t) => t.$method($($arg),*),
            BuiltinTool::DelegateResearch(t) => t.$method($($arg),*),
            BuiltinTool::CryptoPrice(t) => t.$method($($arg),*),
            BuiltinTool::WordWrite(t) => t.$method($($arg),*),
            BuiltinTool::ExcelRead(t) => t.$method($($arg),*),
            BuiltinTool::ProjectCreate(t) => t.$method($($arg),*),
            BuiltinTool::MemorySearch(t) => t.$method($($arg),*),
            BuiltinTool::MemoryStore(t) => t.$method($($arg),*),
            BuiltinTool::WebFetch(t) => t.$method($($arg),*),
            BuiltinTool::TaskPlanner(t) => t.$method($($arg),*),
            BuiltinTool::RepoMap(t) => t.$method($($arg),*),
            BuiltinTool::SystemHealth(t) => t.$method($($arg),*),
            BuiltinTool::ScanChunks(t) => t.$method($($arg),*),
        }
    };
}

impl BaseTool for BuiltinTool {
    fn tool_id(&self) -> &str {
        delegate_tool!(self, tool_id)
    }
    fn spec(&self) -> &ToolSpec {
        delegate_tool!(self, spec)
    }
    fn execute(&self, params: &Value) -> Result<ToolResult, sunday_core::SUNDAYError> {
        delegate_tool!(self, execute, params)
    }
}
