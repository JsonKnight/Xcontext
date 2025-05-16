use crate::config::Config;
use crate::error::{AppError, Result};
use crate::output_formats::get_builtin_ignore_patterns; // Keep this import
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::{WalkBuilder, WalkState};
use log;
use rayon::prelude::*;
#[cfg(feature = "serde_support")] // Corrected newline before this line
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::mpsc;

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: PathBuf,
    pub content: String,
    pub size: usize,
}

// Corrected: Made TreeNode public and conditional compilation for Serialize
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde_support", derive(Serialize))]
pub struct TreeNode {
    name: String,
    #[cfg_attr(feature = "serde_support", serde(rename = "type"))]
    node_type: String,
    #[cfg_attr(
        feature = "serde_support",
        serde(skip_serializing_if = "Option::is_none") // Corrected syntax is fine here
    )]
    children: Option<Vec<TreeNode>>,
}

pub fn gather_files_and_tree(
    project_root: &Path,
    config: &Config,
    quiet: bool, // Keep quiet for conditional logging
) -> Result<(Vec<FileInfo>, Vec<FileInfo>, Vec<(String, bool)>)> {
    log::debug!("Starting file and tree gathering process...");
    let tree_include_patterns = config.get_effective_include(&config.tree.include);
    let tree_exclude_patterns = config.get_effective_exclude(&config.tree.exclude);
    let source_include_patterns = config.get_effective_include(&config.source.include);
    let source_exclude_patterns = config.get_effective_exclude(&config.source.exclude);
    let docs_include_patterns = config.get_effective_include(&config.docs.include);
    let docs_exclude_patterns = config.get_effective_exclude(&config.docs.exclude);

    log::trace!("Building glob sets for filtering...");
    let tree_include_set = build_glob_set_from_vec(tree_include_patterns)?;
    let tree_exclude_set = build_glob_set_from_vec(tree_exclude_patterns)?;
    let has_tree_includes = !tree_include_patterns.is_empty();

    let source_include_set = build_glob_set_from_vec(source_include_patterns)?;
    let source_exclude_set = build_glob_set_from_vec(source_exclude_patterns)?;
    let has_source_includes = !source_include_patterns.is_empty();

    let docs_active = config.is_docs_section_active();
    let docs_include_set = if docs_active {
        build_glob_set_from_vec(docs_include_patterns)?
    } else {
        GlobSet::empty()
    };
    let docs_exclude_set = if docs_active {
        build_glob_set_from_vec(docs_exclude_patterns)?
    } else {
        GlobSet::empty()
    };
    let has_docs_includes = docs_active && !docs_include_patterns.is_empty();

    let builtin_ignores = get_builtin_ignore_patterns();
    let common_builtin_exclude_set = build_glob_set_from_vec(&builtin_ignores.common)?;
    let tree_builtin_exclude_set = build_glob_set_from_vec(&builtin_ignores.tree)?;
    let source_builtin_exclude_set = build_glob_set_from_vec(&builtin_ignores.source)?;
    let docs_builtin_exclude_set = build_glob_set_from_vec(&builtin_ignores.docs)?;
    let use_builtin_ignores = config.get_effective_builtin_ignore();
    log::trace!("Glob sets built successfully.");

    let mut builder = WalkBuilder::new(project_root);
    builder.threads(rayon::current_num_threads().min(12));
    builder.hidden(false); // Consider making this configurable?

    let use_global_gitignore = config.general.use_gitignore;
    builder.ignore(use_global_gitignore);
    builder.git_ignore(use_global_gitignore);
    builder.git_exclude(use_global_gitignore);
    builder.require_git(false);
    log::debug!(
        "WalkBuilder configured (gitignore: {}, builtin: {})",
        use_global_gitignore,
        use_builtin_ignores
    );

    let walker = builder.build_parallel();
    let project_root_clone = project_root.to_path_buf();

    #[derive(Debug)]
    struct WalkedPathInfo {
        path: PathBuf,
        relative_path: PathBuf,
        is_dir: bool,
    }
    let (tx_walked, rx_walked) = mpsc::channel::<WalkedPathInfo>();
    let tx_for_closure = tx_walked.clone();

    log::info!("Walking project directory: {}", project_root.display());
    walker.run(move || {
        let tx_thread = tx_for_closure.clone();
        let proj_root = project_root_clone.clone();

        Box::new(move |entry_result| {
            match entry_result {
                Ok(entry) => {
                    let path = entry.path();
                    if entry.depth() == 0 {
                        return WalkState::Continue;
                    }
                    // Skip cache directory explicitly if walkbuilder doesn't handle it
                    if path.strip_prefix(&proj_root).map_or(false, |rel| {
                        rel.starts_with(crate::config::DEFAULT_CACHE_DIR) // Use constant
                    }) {
                        log::trace!("Skipping cache directory: {}", path.display());
                        return WalkState::Skip;
                    }

                    if let Some(relative_path) = pathdiff::diff_paths(path, &proj_root) {
                        let is_dir = entry.file_type().map_or(false, |ft| ft.is_dir());
                        log::trace!("Walked path: {}", relative_path.display());
                        if tx_thread
                            .send(WalkedPathInfo {
                                path: path.to_path_buf(),
                                relative_path,
                                is_dir,
                            })
                            .is_err()
                        {
                            log::error!("Receiver dropped for walked paths, stopping walk early.");
                            return WalkState::Quit;
                        }
                    } else {
                        log::warn!("Could not get relative path for: {}", path.display());
                    }
                }
                Err(e) => {
                    log::warn!("Error walking directory: {}", e);
                }
            }
            WalkState::Continue
        })
    });
    drop(tx_walked);

    let walked_paths: Vec<WalkedPathInfo> = rx_walked.into_iter().collect();
    log::info!(
        "Directory walk complete. Found {} potential paths.",
        walked_paths.len()
    );

    log::debug!("Filtering walked paths based on configuration...");
    let mut tree_candidates = Vec::<(String, bool)>::new();
    let mut source_file_paths = Vec::<PathBuf>::new();
    let mut docs_file_paths = Vec::<PathBuf>::new();
    let mut file_read_errors = Vec::<AppError>::new(); // Collect errors

    for walked_info in walked_paths {
        let relative_path = &walked_info.relative_path;
        let absolute_path = &walked_info.path;
        let is_dir = walked_info.is_dir;

        // Explicitly skip .git
        if relative_path.components().next() == Some(std::path::Component::Normal(".git".as_ref()))
        // Corrected: ".git" is fine here
        {
            log::trace!(
                "Explicitly skipping path within .git: {}",
                relative_path.display()
            );
            continue;
        }

        let tree_git_ignore = config.get_effective_gitignore(&config.tree.use_gitignore);
        let docs_git_ignore = config.get_effective_gitignore(&config.docs.use_gitignore);
        let source_git_ignore = config.get_effective_gitignore(&config.source.use_gitignore);

        let include_in_tree = config.tree.enabled
            && should_include(
                relative_path,
                is_dir,
                &tree_include_set,
                has_tree_includes,
                &tree_exclude_set,
                tree_git_ignore,
                project_root, // Pass project root if needed by gitignore logic internally
                use_builtin_ignores,
                &common_builtin_exclude_set,
                &tree_builtin_exclude_set,
            );

        let include_in_docs = !is_dir
            && docs_active
            && should_include(
                relative_path,
                false, // is_dir is false for files
                &docs_include_set,
                has_docs_includes,
                &docs_exclude_set,
                docs_git_ignore,
                project_root,
                use_builtin_ignores,
                &common_builtin_exclude_set,
                &docs_builtin_exclude_set,
            );

        let include_in_source = !is_dir
            && !include_in_docs // Don't include if it's already a doc file
            && config.source.enabled
            && should_include(
                relative_path,
                false, // is_dir is false for files
                &source_include_set,
                has_source_includes,
                &source_exclude_set,
                source_git_ignore,
                project_root,
                use_builtin_ignores,
                &common_builtin_exclude_set,
                &source_builtin_exclude_set,
            );

        if include_in_tree {
            log::trace!("Including in tree: {}", relative_path.display());
            tree_candidates.push((relative_path.to_string_lossy().into_owned(), is_dir));
        }

        if include_in_docs {
            log::trace!("Including in docs: {}", relative_path.display());
            docs_file_paths.push(absolute_path.clone());
        } else if include_in_source {
            log::trace!("Including in source: {}", relative_path.display());
            source_file_paths.push(absolute_path.clone());
        } else if !is_dir && !include_in_tree {
            // Only log file exclusions if not included in tree
            log::trace!("Excluding file: {}", relative_path.display());
        }
    }
    log::debug!("Path filtering complete.");

    log::info!(
        "Reading content for {} source files and {} docs files...",
        source_file_paths.len(),
        docs_file_paths.len()
    );

    let read_files = |paths: Vec<PathBuf>| -> (Vec<FileInfo>, Vec<AppError>) {
        let results: Vec<_> = paths
            .into_par_iter()
            .map(|path| match fs::read(&path) {
                Ok(bytes) => {
                    let size = bytes.len();
                    match String::from_utf8(bytes) {
                        Ok(content) => Ok(FileInfo {
                            path,
                            content,
                            size,
                        }),
                        Err(e) => {
                            log::debug!("Skipping non-UTF-8 file: {} ({})", path.display(), e);
                            Err(AppError::DataLoading(format!(
                                "Skipped non-UTF-8 file: {}",
                                path.display()
                            )))
                        }
                    }
                }
                Err(e) => Err(AppError::FileRead {
                    path: path.clone(),
                    source: e,
                }),
            })
            .collect();

        let mut files = Vec::new();
        let mut errors = Vec::new();
        for res in results {
            match res {
                Ok(info) => files.push(info),
                Err(AppError::DataLoading(_)) => { /* Already logged, skip */ }
                Err(e) => errors.push(e),
            }
        }
        (files, errors)
    };

    let (mut final_source_files, source_errors) = read_files(source_file_paths);
    let (mut final_docs_files, docs_errors) = read_files(docs_file_paths);
    file_read_errors.extend(source_errors);
    file_read_errors.extend(docs_errors);
    log::info!("File reading complete.");

    // Sort results for deterministic output
    final_source_files.par_sort_unstable_by(|a, b| a.path.cmp(&b.path));
    final_docs_files.par_sort_unstable_by(|a, b| a.path.cmp(&b.path));
    tree_candidates.par_sort_unstable_by(|a, b| a.0.cmp(&b.0));

    // Report errors gathered during file reading if not quiet
    if !file_read_errors.is_empty() && !quiet {
        use colored::Colorize; // Only needed here
        eprintln!(
            // Corrected: Print a newline character here, not literal `\n`
            "\n{}",
            "⚠️ Warning: Errors encountered during file reading:".yellow()
        );
        for err in file_read_errors {
            eprintln!(" - {}", err);
        }
        eprintln!("---");
    }

    Ok((final_source_files, final_docs_files, tree_candidates))
}

fn build_glob_set_from_vec(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pattern_str in patterns {
        let mut processed_pattern = pattern_str.trim().to_string();
        if processed_pattern.ends_with('/') && processed_pattern.len() > 1 {
            processed_pattern.push_str("**");
        }
        match Glob::new(&processed_pattern) {
            Ok(glob) => {
                log::trace!(
                    "Adding glob pattern: {} (processed as {})",
                    pattern_str,
                    processed_pattern
                );
                builder.add(glob);
            }
            Err(e) => {
                // Corrected: Use double quotes for format string
                log::error!("Invalid glob pattern \"{}\": {}", pattern_str, e);
                return Err(AppError::Glob(format!(
                    // Corrected: Use double quotes for format string
                    "Invalid glob pattern \"{}\" (processed as \"{}\"): {}",
                    pattern_str, processed_pattern, e
                )));
            }
        }
    }
    builder.build().map_err(|e| {
        log::error!("Error building glob set: {}", e);
        AppError::Glob(e.to_string())
    })
}

fn should_include(
    relative_path: &Path,
    is_dir: bool,
    include_set: &GlobSet,
    has_includes: bool, // True if include patterns were provided
    exclude_set: &GlobSet,
    _use_gitignore: bool, // Handled by WalkBuilder, keep param for signature consistency?
    _project_root: &Path, // Potentially needed if gitignore logic were here
    use_builtin: bool,
    common_builtin_exclude: &GlobSet,
    section_builtin_exclude: &GlobSet,
) -> bool {
    // 1. Check Explicit Excludes
    if exclude_set.is_match(relative_path)
        || (is_dir && exclude_set.is_match(relative_path.join("dummy_file_for_dir_match")))
    {
        log::trace!(
            "Path excluded by explicit exclude set: {}",
            relative_path.display()
        );
        return false;
    }

    // 2. Check Explicit Includes (if any were provided)
    // Check both file and potential directory match for includes
    let included_explicitly = !has_includes
        || include_set.is_match(relative_path)
        || (is_dir && include_set.is_match(relative_path.join("dummy_file_for_dir_match")));

    if !included_explicitly {
        log::trace!(
            "Path not included by explicit include set: {}",
            relative_path.display()
        );
        return false;
    }

    // 3. Gitignore filtering is handled by the WalkBuilder itself

    // 4. Check Built-in Ignores
    if use_builtin {
        if common_builtin_exclude.is_match(relative_path)
            || (is_dir
                && common_builtin_exclude.is_match(relative_path.join("dummy_file_for_dir_match")))
        {
            log::trace!(
                "Path excluded by common built-in ignores: {}",
                relative_path.display()
            );
            return false;
        }
        if section_builtin_exclude.is_match(relative_path)
            || (is_dir
                && section_builtin_exclude.is_match(relative_path.join("dummy_file_for_dir_match")))
        {
            log::trace!(
                "Path excluded by section built-in ignores: {}",
                relative_path.display()
            );
            return false;
        }
    }

    // If not excluded by any rule, include it
    log::trace!("Path included: {}", relative_path.display());
    true
}

pub fn build_tree_from_paths(relative_path_types: &[(String, bool)]) -> Result<Vec<TreeNode>> {
    log::debug!(
        "Building tree structure from {} paths...",
        relative_path_types.len()
    );
    let mut root_nodes: Vec<TreeNode> = Vec::new();

    for (rel_path_str, is_dir) in relative_path_types {
        let rel_path = PathBuf::from(rel_path_str);
        let components: Vec<String> = rel_path
            .components()
            .filter_map(|c| match c {
                Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect();

        if !components.is_empty() {
            if let Err(e) = insert_node(&mut root_nodes, &components, *is_dir) {
                // Corrected: Use double quotes for format string
                log::error!(
                    "Error inserting node into tree for path \"{}\": {}",
                    rel_path_str,
                    e
                );
                // Optionally bubble up the error: return Err(e);
            }
        }
    }

    // Sort root nodes alphabetically
    root_nodes.sort_by(|a, b| a.name.cmp(&b.name));
    log::debug!("Tree structure built successfully.");
    Ok(root_nodes)
}

fn insert_node(
    current_level_nodes: &mut Vec<TreeNode>,
    components: &[String],
    is_dir_at_end: bool, // True if the original path was a directory
) -> Result<()> {
    if components.is_empty() {
        return Ok(());
    }

    let component_name = &components[0];
    let remaining_components = &components[1..];
    let is_last_component = remaining_components.is_empty();

    match current_level_nodes.binary_search_by(|node| node.name.cmp(component_name)) {
        Ok(index) => {
            let existing_node = &mut current_level_nodes[index];

            // If we find an existing node:
            // - Ensure it's marked as a directory if we need to descend further
            // - Ensure it's marked as a directory if the original path was a dir at this level
            if !is_last_component {
                if existing_node.node_type == "file" {
                    // Conflict: Trying to add children to a file node
                    return Err(AppError::Config(format!(
                        "Tree conflict: Trying to create children within file component {}",
                        component_name
                    )));
                }
                // Ensure children structure exists if descending
                if existing_node.children.is_none() {
                    existing_node.children = Some(Vec::new());
                }
                insert_node(
                    existing_node.children.as_mut().unwrap(),
                    remaining_components,
                    is_dir_at_end,
                )?;
                // Keep children sorted
                existing_node
                    .children
                    .as_mut()
                    .unwrap()
                    .sort_by(|a, b| a.name.cmp(&b.name));
            } else if is_dir_at_end && existing_node.node_type == "file" {
                // Update node type if the full path indicates it's a directory
                existing_node.node_type = "directory".to_string();
                // If it's now a directory, ensure it can have children (even if empty for now)
                if existing_node.children.is_none() {
                    existing_node.children = Some(Vec::new());
                }
            }
            // If is_last_component and !is_dir_at_end, no change needed if existing is file or dir.
            // If is_last_component and is_dir_at_end and existing is already dir, no change needed.
        }
        Err(insertion_point) => {
            // Node doesn't exist, create it
            let node_type_str = if is_last_component {
                if is_dir_at_end { "directory" } else { "file" }
            } else {
                // If not the last component, it must be an intermediate directory
                "directory"
            };

            let mut new_node = TreeNode {
                name: component_name.clone(),
                node_type: node_type_str.to_string(),
                children: if node_type_str == "directory" {
                    Some(Vec::new())
                } else {
                    None
                },
            };

            if !is_last_component {
                // Must be a directory if not last, insert remaining components recursively
                insert_node(
                    new_node.children.as_mut().unwrap(), // Safe because type is "directory"
                    remaining_components,
                    is_dir_at_end,
                )?;
                // Keep children sorted
                new_node
                    .children
                    .as_mut()
                    .unwrap()
                    .sort_by(|a, b| a.name.cmp(&b.name));
            }

            current_level_nodes.insert(insertion_point, new_node);
        }
    }
    Ok(())
}
