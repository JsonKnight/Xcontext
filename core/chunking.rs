use crate::error::{AppError, Result};
use crate::gather::FileInfo;
use crate::output_formats::{ChunkFile, ChunkInfo, FileContextInfo};
use byte_unit::Byte;
use log;
use std::convert::TryInto;
use std::path::Path;
use std::str::FromStr;

pub fn split_files_into_chunks(
    source_files: Vec<FileInfo>,
    chunk_size_str: &str,
    project_root: &Path,
) -> Result<Vec<ChunkFile>> {
    let byte_value = Byte::from_str(chunk_size_str).map_err(|e| {
        AppError::Chunking(format!(
            "Invalid chunk size format '{}': {}. Use KB, MB, etc.",
            chunk_size_str, e
        ))
    })?;
    let target_chunk_size_bytes: u128 = byte_value.into();
    let target_chunk_size_bytes_usize = target_chunk_size_bytes.try_into().map_err(|_| {
        AppError::Chunking("Chunk size exceeds maximum usize value on this platform.".to_string())
    })?;

    if target_chunk_size_bytes_usize == 0 {
        return Err(AppError::Chunking(
            "Chunk size must be greater than 0 bytes".to_string(),
        ));
    }

    let mut chunks_data: Vec<Vec<FileContextInfo>> = Vec::new();
    let mut current_chunk_files: Vec<FileContextInfo> = Vec::new();
    let mut current_chunk_size: usize = 0;

    let all_file_contexts: Vec<FileContextInfo> = source_files
        .into_iter()
        .map(|finfo| FileContextInfo {
            path: pathdiff::diff_paths(&finfo.path, project_root)
                .unwrap_or_else(|| finfo.path.clone())
                .to_string_lossy()
                .to_string(),
            content: finfo.content,
        })
        .collect();

    for file_context in all_file_contexts {
        let file_size = file_context.content.len(); // Use content length for size

        if file_size == 0 {
            log::trace!("Skipping empty file: {}", file_context.path);
            continue; // Skip empty files
        }

        if file_size > target_chunk_size_bytes_usize {
            log::trace!(
                "File {} ({}) exceeds chunk size ({}), putting in its own chunk.",
                file_context.path,
                file_size,
                target_chunk_size_bytes_usize
            );
            // If the current chunk isn't empty, push it first
            if !current_chunk_files.is_empty() {
                chunks_data.push(std::mem::take(&mut current_chunk_files));
                current_chunk_size = 0; // Reset size for the next chunk
            }
            // Push the large file as its own chunk
            chunks_data.push(vec![file_context]);
            continue; // Move to the next file
        }

        // Check if adding the current file exceeds the chunk size
        if !current_chunk_files.is_empty()
            && (current_chunk_size.saturating_add(file_size)) > target_chunk_size_bytes_usize
        {
            // Current chunk is full, push it and start a new one
            chunks_data.push(std::mem::take(&mut current_chunk_files));
            current_chunk_files = vec![file_context]; // Start new chunk with current file
            current_chunk_size = file_size;
        } else {
            // Add file to the current chunk
            current_chunk_size = current_chunk_size.saturating_add(file_size);
            current_chunk_files.push(file_context);
        }
    }

    // Push the last chunk if it's not empty
    if !current_chunk_files.is_empty() {
        chunks_data.push(current_chunk_files);
    }

    let total_parts = chunks_data.len();
    if total_parts == 0 {
        log::debug!("No non-empty files to chunk.");
        return Ok(Vec::new()); // Return empty vec if no chunks were created
    }

    log::info!("Split content into {} chunks.", total_parts);

    let final_chunks: Vec<ChunkFile> = chunks_data
        .into_iter()
        .enumerate()
        .map(|(i, chunk_files)| {
            let chunk_num = i + 1;
            let chunk_info = ChunkInfo {
                current_part: chunk_num,
                total_parts,
            };
            ChunkFile {
                files: chunk_files,
                chunk_info,
            }
        })
        .collect();

    Ok(final_chunks)
}
