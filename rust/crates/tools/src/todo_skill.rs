use std::path::{Path, PathBuf};

use crate::types::{
    SkillFileEntry, SkillInput, SkillOutput, SkillReferenceEntry, TodoItem, TodoStatus,
    TodoWriteInput, TodoWriteOutput,
};

pub(crate) fn execute_todo_write(input: TodoWriteInput) -> Result<TodoWriteOutput, String> {
    validate_todos(&input.todos)?;
    let store_path = todo_store_path()?;
    let old_todos = if store_path.exists() {
        serde_json::from_str::<Vec<TodoItem>>(
            &std::fs::read_to_string(&store_path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?
    } else {
        Vec::new()
    };

    let all_done = input
        .todos
        .iter()
        .all(|todo| matches!(todo.status, TodoStatus::Completed));
    let persisted = if all_done {
        Vec::new()
    } else {
        input.todos.clone()
    };

    if let Some(parent) = store_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(
        &store_path,
        serde_json::to_string_pretty(&persisted).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;

    let verification_nudge_needed = (all_done
        && input.todos.len() >= 3
        && !input
            .todos
            .iter()
            .any(|todo| todo.content.to_lowercase().contains("verif")))
    .then_some(true);

    Ok(TodoWriteOutput {
        old_todos,
        new_todos: input.todos,
        verification_nudge_needed,
    })
}

pub(crate) fn execute_skill(input: SkillInput) -> Result<SkillOutput, String> {
    let skill_path = resolve_skill_path(&input.skill)?;
    let prompt = std::fs::read_to_string(&skill_path).map_err(|error| error.to_string())?;
    let skill_root = skill_path
        .parent()
        .ok_or_else(|| format!("skill path has no parent: {}", skill_path.display()))?;
    let description = parse_skill_description(&prompt);
    let references = collect_skill_references(skill_root, &prompt);
    Ok(SkillOutput {
        skill: input.skill,
        path: skill_path.display().to_string(),
        root_path: skill_root.display().to_string(),
        args: input.args,
        description,
        prompt,
        scripts: collect_skill_entries(skill_root, "scripts")?,
        assets: collect_skill_entries(skill_root, "assets")?,
        templates: collect_skill_entries(skill_root, "templates")?,
        references,
    })
}

fn validate_todos(todos: &[TodoItem]) -> Result<(), String> {
    if todos.is_empty() {
        return Err(String::from("todos must not be empty"));
    }
    if todos.iter().any(|todo| todo.content.trim().is_empty()) {
        return Err(String::from("todo content must not be empty"));
    }
    if todos.iter().any(|todo| todo.active_form.trim().is_empty()) {
        return Err(String::from("todo activeForm must not be empty"));
    }
    Ok(())
}

fn todo_store_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("CLAWD_TODO_STORE") {
        return Ok(PathBuf::from(path));
    }
    Ok(std::env::current_dir()
        .map_err(|error| error.to_string())?
        .join(".clawd-todos.json"))
}

fn resolve_skill_path(skill: &str) -> Result<PathBuf, String> {
    let requested = skill.trim().trim_start_matches('/').trim_start_matches('$');
    if requested.is_empty() {
        return Err(String::from("skill must not be empty"));
    }

    let mut candidates = Vec::new();
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        candidates.push(PathBuf::from(codex_home).join("skills"));
    }
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join(".agents").join("skills"));
        candidates.push(home.join(".config").join("opencode").join("skills"));
        candidates.push(home.join(".codex").join("skills"));
    }
    candidates.push(PathBuf::from("/home/bellman/.codex/skills"));

    for root in candidates {
        let direct = root.join(requested).join("SKILL.md");
        if direct.exists() {
            return Ok(direct);
        }
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path().join("SKILL.md");
                if path.exists()
                    && entry
                        .file_name()
                        .to_string_lossy()
                        .eq_ignore_ascii_case(requested)
                {
                    return Ok(path);
                }
            }
        }
    }

    Err(format!("unknown skill: {requested}"))
}

fn parse_skill_description(contents: &str) -> Option<String> {
    contents.lines().find_map(|line| {
        line.strip_prefix("description:")
            .map(str::trim)
            .filter(|trimmed| !trimmed.is_empty())
            .map(ToString::to_string)
    })
}

fn collect_skill_entries(skill_root: &Path, child: &str) -> Result<Vec<SkillFileEntry>, String> {
    let directory = skill_root.join(child);
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_files_recursively(skill_root, &directory, &mut files)?;
    Ok(files)
}

fn collect_files_recursively(
    skill_root: &Path,
    current: &Path,
    files: &mut Vec<SkillFileEntry>,
) -> Result<(), String> {
    let mut entries = std::fs::read_dir(current)
        .map_err(|error| error.to_string())?
        .flatten()
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        let metadata = entry.metadata().map_err(|error| error.to_string())?;
        if metadata.is_dir() {
            collect_files_recursively(skill_root, &path, files)?;
            continue;
        }
        if metadata.is_file() {
            files.push(SkillFileEntry {
                relative_path: relative_to_root(skill_root, &path),
                absolute_path: path.display().to_string(),
            });
        }
    }
    Ok(())
}

fn collect_skill_references(skill_root: &Path, prompt: &str) -> Vec<SkillReferenceEntry> {
    prompt
        .lines()
        .flat_map(extract_markdown_links)
        .filter(|(_, target)| is_relative_reference(target))
        .map(|(label, target)| {
            let absolute = skill_root.join(target);
            SkillReferenceEntry {
                label,
                relative_path: target.to_string(),
                absolute_path: absolute.display().to_string(),
                exists: absolute.exists(),
            }
        })
        .collect()
}

fn extract_markdown_links(line: &str) -> Vec<(Option<String>, &str)> {
    let mut links = Vec::new();
    let mut remaining = line;
    while let Some(open_label) = remaining.find('[') {
        let after_open = &remaining[open_label + 1..];
        let Some(close_label) = after_open.find(']') else {
            break;
        };
        let label = &after_open[..close_label];
        let after_label = &after_open[close_label + 1..];
        let Some(open_target) = after_label.find('(') else {
            remaining = after_label;
            continue;
        };
        let after_target_open = &after_label[open_target + 1..];
        let Some(close_target) = after_target_open.find(')') else {
            break;
        };
        let target = after_target_open[..close_target].trim();
        links.push((
            (!label.trim().is_empty()).then(|| label.trim().to_string()),
            target,
        ));
        remaining = &after_target_open[close_target + 1..];
    }
    links
}

fn is_relative_reference(target: &str) -> bool {
    !target.is_empty()
        && !target.starts_with("http://")
        && !target.starts_with("https://")
        && !target.starts_with("app://")
        && !target.starts_with("plugin://")
        && !target.starts_with('#')
}

fn relative_to_root(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
