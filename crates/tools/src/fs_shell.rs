use super::*;

pub(crate) fn run_bash(input: BashCommandInput) -> Result<String, String> {
    if let Some(output) = workspace_test_branch_preflight(&input.command) {
        return serde_json::to_string_pretty(&output).map_err(|error| error.to_string());
    }
    serde_json::to_string_pretty(&execute_bash(input).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

pub(crate) fn workspace_test_branch_preflight(command: &str) -> Option<BashCommandOutput> {
    if !is_workspace_test_command(command) {
        return None;
    }

    let branch = git_stdout(&["branch", "--show-current"])?;
    let main_ref = resolve_main_ref(&branch)?;
    let freshness = check_freshness(&branch, &main_ref);
    match freshness {
        BranchFreshness::Fresh => None,
        BranchFreshness::Stale {
            commits_behind,
            missing_fixes,
        } => Some(branch_divergence_output(
            command,
            &branch,
            &main_ref,
            commits_behind,
            None,
            &missing_fixes,
        )),
        BranchFreshness::Diverged {
            ahead,
            behind,
            missing_fixes,
        } => Some(branch_divergence_output(
            command,
            &branch,
            &main_ref,
            behind,
            Some(ahead),
            &missing_fixes,
        )),
    }
}

fn is_workspace_test_command(command: &str) -> bool {
    let normalized = normalize_shell_command(command);
    [
        "cargo test --workspace",
        "cargo test --all",
        "cargo nextest run --workspace",
        "cargo nextest run --all",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn normalize_shell_command(command: &str) -> String {
    command
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

fn resolve_main_ref(branch: &str) -> Option<String> {
    let has_local_main = git_ref_exists("main");
    let has_remote_main = git_ref_exists("origin/main");

    if branch == "main" && has_remote_main {
        Some("origin/main".to_string())
    } else if has_local_main {
        Some("main".to_string())
    } else if has_remote_main {
        Some("origin/main".to_string())
    } else {
        None
    }
}

fn git_ref_exists(reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn git_stdout(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!stdout.is_empty()).then_some(stdout)
}

fn branch_divergence_output(
    command: &str,
    branch: &str,
    main_ref: &str,
    commits_behind: usize,
    commits_ahead: Option<usize>,
    missing_fixes: &[String],
) -> BashCommandOutput {
    let relation = commits_ahead.map_or_else(
        || format!("is {commits_behind} commit(s) behind"),
        |ahead| format!("has diverged ({ahead} ahead, {commits_behind} behind)"),
    );
    let missing_summary = if missing_fixes.is_empty() {
        "(none surfaced)".to_string()
    } else {
        missing_fixes.join("; ")
    };
    let stderr = format!(
        "branch divergence detected before workspace tests: `{branch}` {relation} `{main_ref}`. Missing commits: {missing_summary}. Merge or rebase `{main_ref}` before re-running `{command}`."
    );

    BashCommandOutput {
        stdout: String::new(),
        stderr: stderr.clone(),
        raw_output_path: None,
        interrupted: false,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox: None,
        return_code_interpretation: Some("preflight_blocked:branch_divergence".to_string()),
        no_output_expected: Some(false),
        structured_content: Some(vec![serde_json::to_value(
            LaneEvent::new(
                LaneEventName::BranchStaleAgainstMain,
                LaneEventStatus::Blocked,
                iso8601_now(),
            )
            .with_failure_class(LaneFailureClass::BranchDivergence)
            .with_detail(stderr.clone())
            .with_data(json!({
                "branch": branch,
                "mainRef": main_ref,
                "commitsBehind": commits_behind,
                "commitsAhead": commits_ahead,
                "missingCommits": missing_fixes,
                "blockedCommand": command,
                "recommendedAction": format!("merge or rebase {main_ref} before workspace tests")
            })),
        )
        .expect("lane event should serialize")]),
        persisted_output_path: None,
        persisted_output_size: None,
        sandbox_status: None,
    }
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_read_file(input: ReadFileInput) -> Result<String, String> {
    to_pretty_json(read_file(&input.path, input.offset, input.limit).map_err(io_to_string)?)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_write_file(input: WriteFileInput) -> Result<String, String> {
    to_pretty_json(write_file(&input.path, &input.content).map_err(io_to_string)?)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_edit_file(input: EditFileInput) -> Result<String, String> {
    to_pretty_json(
        edit_file(
            &input.path,
            &input.old_string,
            &input.new_string,
            input.replace_all.unwrap_or(false),
        )
        .map_err(io_to_string)?,
    )
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_glob_search(input: GlobSearchInputValue) -> Result<String, String> {
    to_pretty_json(glob_search(&input.pattern, input.path.as_deref()).map_err(io_to_string)?)
}

#[allow(clippy::needless_pass_by_value)]
pub(crate) fn run_grep_search(input: GrepSearchInput) -> Result<String, String> {
    to_pretty_json(grep_search(&input).map_err(io_to_string)?)
}

pub(crate) fn run_notebook_edit(input: NotebookEditInput) -> Result<String, String> {
    to_pretty_json(execute_notebook_edit(input)?)
}

pub(crate) fn run_repl(input: ReplInput) -> Result<String, String> {
    to_pretty_json(execute_repl(input)?)
}

pub(crate) fn run_powershell(input: PowerShellInput) -> Result<String, String> {
    to_pretty_json(execute_powershell(input).map_err(|error| error.to_string())?)
}

#[allow(clippy::needless_pass_by_value)]
fn io_to_string(error: std::io::Error) -> String {
    error.to_string()
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReadFileInput {
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WriteFileInput {
    path: String,
    content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditFileInput {
    path: String,
    old_string: String,
    new_string: String,
    replace_all: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GlobSearchInputValue {
    pattern: String,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NotebookEditInput {
    notebook_path: String,
    cell_id: Option<String>,
    new_source: Option<String>,
    cell_type: Option<NotebookCellType>,
    edit_mode: Option<NotebookEditMode>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum NotebookCellType {
    Code,
    Markdown,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum NotebookEditMode {
    Replace,
    Insert,
    Delete,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReplInput {
    code: String,
    language: String,
    timeout_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PowerShellInput {
    command: String,
    timeout: Option<u64>,
    description: Option<String>,
    run_in_background: Option<bool>,
}

#[derive(Debug, Serialize)]
struct NotebookEditOutput {
    new_source: String,
    cell_id: Option<String>,
    cell_type: Option<NotebookCellType>,
    language: String,
    edit_mode: String,
    error: Option<String>,
    notebook_path: String,
    original_file: String,
    updated_file: String,
}

#[derive(Debug, Serialize)]
struct ReplOutput {
    language: String,
    stdout: String,
    stderr: String,
    #[serde(rename = "exitCode")]
    exit_code: i32,
    #[serde(rename = "durationMs")]
    duration_ms: u128,
}

#[allow(clippy::too_many_lines)]
fn execute_notebook_edit(input: NotebookEditInput) -> Result<NotebookEditOutput, String> {
    let path = std::path::PathBuf::from(&input.notebook_path);
    if path.extension().and_then(|ext| ext.to_str()) != Some("ipynb") {
        return Err(String::from(
            "File must be a Jupyter notebook (.ipynb file).",
        ));
    }

    let original_file = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let mut notebook: serde_json::Value =
        serde_json::from_str(&original_file).map_err(|error| error.to_string())?;
    let language = notebook
        .get("metadata")
        .and_then(|metadata| metadata.get("kernelspec"))
        .and_then(|kernelspec| kernelspec.get("language"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("python")
        .to_string();
    let cells = notebook
        .get_mut("cells")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| String::from("Notebook cells array not found"))?;

    let edit_mode = input.edit_mode.unwrap_or(NotebookEditMode::Replace);
    let target_index = match input.cell_id.as_deref() {
        Some(cell_id) => Some(resolve_cell_index(cells, Some(cell_id), edit_mode)?),
        None if matches!(
            edit_mode,
            NotebookEditMode::Replace | NotebookEditMode::Delete
        ) =>
        {
            Some(resolve_cell_index(cells, None, edit_mode)?)
        }
        None => None,
    };
    let resolved_cell_type = match edit_mode {
        NotebookEditMode::Delete => None,
        NotebookEditMode::Insert => Some(input.cell_type.unwrap_or(NotebookCellType::Code)),
        NotebookEditMode::Replace => Some(input.cell_type.unwrap_or_else(|| {
            target_index
                .and_then(|index| cells.get(index))
                .and_then(cell_kind)
                .unwrap_or(NotebookCellType::Code)
        })),
    };
    let new_source = require_notebook_source(input.new_source, edit_mode)?;

    let cell_id = match edit_mode {
        NotebookEditMode::Insert => {
            let resolved_cell_type = resolved_cell_type
                .ok_or_else(|| String::from("insert mode requires a cell type"))?;
            let new_id = make_cell_id(cells.len());
            let new_cell = build_notebook_cell(&new_id, resolved_cell_type, &new_source);
            let insert_at = target_index.map_or(cells.len(), |index| index + 1);
            cells.insert(insert_at, new_cell);
            cells
                .get(insert_at)
                .and_then(|cell| cell.get("id"))
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        }
        NotebookEditMode::Delete => {
            let idx = target_index
                .ok_or_else(|| String::from("delete mode requires a target cell index"))?;
            let removed = cells.remove(idx);
            removed
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        }
        NotebookEditMode::Replace => {
            let resolved_cell_type = resolved_cell_type
                .ok_or_else(|| String::from("replace mode requires a cell type"))?;
            let idx = target_index
                .ok_or_else(|| String::from("replace mode requires a target cell index"))?;
            let cell = cells
                .get_mut(idx)
                .ok_or_else(|| String::from("Cell index out of range"))?;
            cell["source"] = serde_json::Value::Array(source_lines(&new_source));
            cell["cell_type"] = serde_json::Value::String(match resolved_cell_type {
                NotebookCellType::Code => String::from("code"),
                NotebookCellType::Markdown => String::from("markdown"),
            });
            match resolved_cell_type {
                NotebookCellType::Code => {
                    if !cell.get("outputs").is_some_and(serde_json::Value::is_array) {
                        cell["outputs"] = json!([]);
                    }
                    if cell.get("execution_count").is_none() {
                        cell["execution_count"] = serde_json::Value::Null;
                    }
                }
                NotebookCellType::Markdown => {
                    if let Some(object) = cell.as_object_mut() {
                        object.remove("outputs");
                        object.remove("execution_count");
                    }
                }
            }
            cell.get("id")
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string)
        }
    };

    let updated_file =
        serde_json::to_string_pretty(&notebook).map_err(|error| error.to_string())?;
    std::fs::write(&path, &updated_file).map_err(|error| error.to_string())?;

    Ok(NotebookEditOutput {
        new_source,
        cell_id,
        cell_type: resolved_cell_type,
        language,
        edit_mode: format_notebook_edit_mode(edit_mode),
        error: None,
        notebook_path: path.display().to_string(),
        original_file,
        updated_file,
    })
}

fn require_notebook_source(
    source: Option<String>,
    edit_mode: NotebookEditMode,
) -> Result<String, String> {
    match edit_mode {
        NotebookEditMode::Delete => Ok(source.unwrap_or_default()),
        NotebookEditMode::Insert | NotebookEditMode::Replace => source
            .ok_or_else(|| String::from("new_source is required for insert and replace edits")),
    }
}

fn build_notebook_cell(cell_id: &str, cell_type: NotebookCellType, source: &str) -> Value {
    let mut cell = json!({
        "cell_type": match cell_type {
            NotebookCellType::Code => "code",
            NotebookCellType::Markdown => "markdown",
        },
        "id": cell_id,
        "metadata": {},
        "source": source_lines(source),
    });
    if let Some(object) = cell.as_object_mut() {
        match cell_type {
            NotebookCellType::Code => {
                object.insert(String::from("outputs"), json!([]));
                object.insert(String::from("execution_count"), Value::Null);
            }
            NotebookCellType::Markdown => {}
        }
    }
    cell
}

fn cell_kind(cell: &serde_json::Value) -> Option<NotebookCellType> {
    cell.get("cell_type")
        .and_then(serde_json::Value::as_str)
        .map(|kind| {
            if kind == "markdown" {
                NotebookCellType::Markdown
            } else {
                NotebookCellType::Code
            }
        })
}

fn execute_repl(input: ReplInput) -> Result<ReplOutput, String> {
    if input.code.trim().is_empty() {
        return Err(String::from("code must not be empty"));
    }
    let runtime = resolve_repl_runtime(&input.language)?;
    let started = Instant::now();
    let mut process = Command::new(runtime.program);
    process
        .args(runtime.args)
        .arg(&input.code)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let output = if let Some(timeout_ms) = input.timeout_ms {
        let mut child = process.spawn().map_err(|error| error.to_string())?;
        loop {
            if child
                .try_wait()
                .map_err(|error| error.to_string())?
                .is_some()
            {
                break child
                    .wait_with_output()
                    .map_err(|error| error.to_string())?;
            }
            if started.elapsed() >= Duration::from_millis(timeout_ms) {
                child.kill().map_err(|error| error.to_string())?;
                child
                    .wait_with_output()
                    .map_err(|error| error.to_string())?;
                return Err(format!(
                    "REPL execution exceeded timeout of {timeout_ms} ms"
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    } else {
        process
            .spawn()
            .map_err(|error| error.to_string())?
            .wait_with_output()
            .map_err(|error| error.to_string())?
    };

    Ok(ReplOutput {
        language: input.language,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(1),
        duration_ms: started.elapsed().as_millis(),
    })
}

struct ReplRuntime {
    program: &'static str,
    args: &'static [&'static str],
}

fn resolve_repl_runtime(language: &str) -> Result<ReplRuntime, String> {
    match language.trim().to_ascii_lowercase().as_str() {
        "python" | "py" => Ok(ReplRuntime {
            program: detect_first_command(&["python3", "python"])
                .ok_or_else(|| String::from("python runtime not found"))?,
            args: &["-c"],
        }),
        "javascript" | "js" | "node" => Ok(ReplRuntime {
            program: detect_first_command(&["node"])
                .ok_or_else(|| String::from("node runtime not found"))?,
            args: &["-e"],
        }),
        "sh" | "shell" | "bash" => Ok(ReplRuntime {
            program: detect_first_command(&["bash", "sh"])
                .ok_or_else(|| String::from("shell runtime not found"))?,
            args: &["-lc"],
        }),
        other => Err(format!("unsupported REPL language: {other}")),
    }
}

fn detect_first_command(commands: &[&'static str]) -> Option<&'static str> {
    commands
        .iter()
        .copied()
        .find(|command| command_exists(command))
}

#[allow(clippy::needless_pass_by_value)]
fn execute_powershell(input: PowerShellInput) -> std::io::Result<runtime::BashCommandOutput> {
    let _ = &input.description;
    if let Some(output) = workspace_test_branch_preflight(&input.command) {
        return Ok(output);
    }
    let shell = detect_powershell_shell()?;
    execute_shell_command(
        shell,
        &input.command,
        input.timeout,
        input.run_in_background,
    )
}

fn detect_powershell_shell() -> std::io::Result<&'static str> {
    if command_exists("pwsh") {
        Ok("pwsh")
    } else if command_exists("powershell") {
        Ok("powershell")
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "PowerShell executable not found (expected `pwsh` or `powershell` in PATH)",
        ))
    }
}

fn command_exists(command: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {command} >/dev/null 2>&1"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[allow(clippy::too_many_lines)]
fn execute_shell_command(
    shell: &str,
    command: &str,
    timeout: Option<u64>,
    run_in_background: Option<bool>,
) -> std::io::Result<runtime::BashCommandOutput> {
    if run_in_background.unwrap_or(false) {
        let child = std::process::Command::new(shell)
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(command)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        return Ok(runtime::BashCommandOutput {
            stdout: String::new(),
            stderr: String::new(),
            raw_output_path: None,
            interrupted: false,
            is_image: None,
            background_task_id: Some(child.id().to_string()),
            backgrounded_by_user: Some(true),
            assistant_auto_backgrounded: Some(false),
            dangerously_disable_sandbox: None,
            return_code_interpretation: None,
            no_output_expected: Some(true),
            structured_content: None,
            persisted_output_path: None,
            persisted_output_size: None,
            sandbox_status: None,
        });
    }

    let mut process = std::process::Command::new(shell);
    process
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-Command")
        .arg(command);
    process
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if let Some(timeout_ms) = timeout {
        let mut child = process.spawn()?;
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait()? {
                let output = child.wait_with_output()?;
                return Ok(runtime::BashCommandOutput {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                    raw_output_path: None,
                    interrupted: false,
                    is_image: None,
                    background_task_id: None,
                    backgrounded_by_user: None,
                    assistant_auto_backgrounded: None,
                    dangerously_disable_sandbox: None,
                    return_code_interpretation: status
                        .code()
                        .filter(|code| *code != 0)
                        .map(|code| format!("exit_code:{code}")),
                    no_output_expected: Some(output.stdout.is_empty() && output.stderr.is_empty()),
                    structured_content: None,
                    persisted_output_path: None,
                    persisted_output_size: None,
                    sandbox_status: None,
                });
            }
            if started.elapsed() >= Duration::from_millis(timeout_ms) {
                let _ = child.kill();
                let output = child.wait_with_output()?;
                let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
                let stderr = if stderr.trim().is_empty() {
                    format!("Command exceeded timeout of {timeout_ms} ms")
                } else {
                    format!(
                        "{}
Command exceeded timeout of {timeout_ms} ms",
                        stderr.trim_end()
                    )
                };
                return Ok(runtime::BashCommandOutput {
                    stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                    stderr,
                    raw_output_path: None,
                    interrupted: true,
                    is_image: None,
                    background_task_id: None,
                    backgrounded_by_user: None,
                    assistant_auto_backgrounded: None,
                    dangerously_disable_sandbox: None,
                    return_code_interpretation: Some(String::from("timeout")),
                    no_output_expected: Some(false),
                    structured_content: None,
                    persisted_output_path: None,
                    persisted_output_size: None,
                    sandbox_status: None,
                });
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    let output = process.output()?;
    Ok(runtime::BashCommandOutput {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        raw_output_path: None,
        interrupted: false,
        is_image: None,
        background_task_id: None,
        backgrounded_by_user: None,
        assistant_auto_backgrounded: None,
        dangerously_disable_sandbox: None,
        return_code_interpretation: output
            .status
            .code()
            .filter(|code| *code != 0)
            .map(|code| format!("exit_code:{code}")),
        no_output_expected: Some(output.stdout.is_empty() && output.stderr.is_empty()),
        structured_content: None,
        persisted_output_path: None,
        persisted_output_size: None,
        sandbox_status: None,
    })
}

fn resolve_cell_index(
    cells: &[serde_json::Value],
    cell_id: Option<&str>,
    edit_mode: NotebookEditMode,
) -> Result<usize, String> {
    if cells.is_empty()
        && matches!(
            edit_mode,
            NotebookEditMode::Replace | NotebookEditMode::Delete
        )
    {
        return Err(String::from("Notebook has no cells to edit"));
    }
    if let Some(cell_id) = cell_id {
        cells
            .iter()
            .position(|cell| cell.get("id").and_then(serde_json::Value::as_str) == Some(cell_id))
            .ok_or_else(|| format!("Cell id not found: {cell_id}"))
    } else {
        Ok(cells.len().saturating_sub(1))
    }
}

fn source_lines(source: &str) -> Vec<serde_json::Value> {
    if source.is_empty() {
        return vec![serde_json::Value::String(String::new())];
    }
    source
        .split_inclusive('\n')
        .map(|line| serde_json::Value::String(line.to_string()))
        .collect()
}

fn format_notebook_edit_mode(mode: NotebookEditMode) -> String {
    match mode {
        NotebookEditMode::Replace => String::from("replace"),
        NotebookEditMode::Insert => String::from("insert"),
        NotebookEditMode::Delete => String::from("delete"),
    }
}

fn make_cell_id(index: usize) -> String {
    format!("cell-{}", index + 1)
}
