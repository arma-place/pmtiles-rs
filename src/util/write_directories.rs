use duplicate::duplicate_item;
use futures::{AsyncSeekExt, AsyncWrite};
use std::io::{Cursor, Result, Seek, Write};

use crate::{header::HEADER_BYTES, Compression, Directory, Entry};

const MAX_ROOT_DIR_LENGTH: u16 = 16384 - HEADER_BYTES as u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
/// Strategies to divide entries into one or multiple leaf directories, when
/// root directory overflows maximum size.
pub enum WriteDirsOverflowStrategy {
    /// Move all entries to leaf directories, so root directory contains only
    /// pointers to leaf directories.
    ///
    /// Will double the size of leaf directories until the root directory fits into its max size.
    OnlyLeafPointers {
        /// The start size of the leaf directories (default 4096)
        start_size: Option<usize>,
    },
}

impl Default for WriteDirsOverflowStrategy {
    fn default() -> Self {
        Self::OnlyLeafPointers { start_size: None }
    }
}

#[duplicate_item(
    fn_name                        async   SeekFrom                input_traits                                      add_await(code) write_directory(directory, output, compression)        only_leaf_pointer_strategy;
    [write_directories_impl]       []      [std::io::SeekFrom]     [(impl Write + Seek)]                             [code]          [directory.to_writer(output, compression)]             [only_leaf_pointer_strategy];
    [write_directories_impl_async] [async] [futures::io::SeekFrom] [(impl AsyncWrite + Unpin + Send + AsyncSeekExt)] [code.await]    [directory.to_async_writer(output, compression).await] [only_leaf_pointer_strategy_async];
)]
async fn fn_name(
    output: &mut input_traits,
    all_entries: &[Entry],
    compression: Compression,
    overflow_strategy: Option<WriteDirsOverflowStrategy>,
) -> Result<Vec<u8>> {
    let start_pos = add_await([output.stream_position()])?;

    {
        let root_directory = Directory::from(all_entries.to_vec());
        write_directory([root_directory], [output], [compression])?;
    }

    let root_directory_length = add_await([output.stream_position()])? - start_pos;

    if root_directory_length <= u64::from(MAX_ROOT_DIR_LENGTH) {
        return Ok(Vec::new());
    }

    match overflow_strategy.unwrap_or_default() {
        WriteDirsOverflowStrategy::OnlyLeafPointers { start_size } => {
            add_await([only_leaf_pointer_strategy(
                output,
                SeekFrom::Start(start_pos),
                all_entries,
                compression,
                start_size,
            )])
        }
    }
}

/// Writes root directory to a writer and return bytes of leaf directory section.
///
/// # Arguments
/// * `output` - Writer to write root directory to
/// * `all_entries` - All tile entries
/// * `compression` - Compression of directories
/// * `overflow_strategy` - Strategy to use, when root directory does not fit in the first 16kB.
///                         If [`None`] is passed, the best strategy is chosen automatically.
///
/// # Errors
/// Will return [`Err`] if `compression` is set to [`Compression::Unknown`] or an I/O error
/// occurred while writing to `output`.
///
pub fn write_directories(
    output: &mut (impl Write + Seek),
    all_entries: &[Entry],
    compression: Compression,
    overflow_strategy: Option<WriteDirsOverflowStrategy>,
) -> Result<Vec<u8>> {
    write_directories_impl(output, all_entries, compression, overflow_strategy)
}

/// Async version of [`write_directories`](write_directories).
///
/// Writes root directory to a writer and return bytes of leaf directory section.
///
/// # Arguments
/// * `output` - Writer to write root directory to
/// * `all_entries` - All tile entries
/// * `compression` - Compression of directories
/// * `overflow_strategy` - Strategy to use, when root directory does not fit in the first 16kB.
///                         If [`None`] is passed, the best strategy is chosen automatically.
///
/// # Errors
/// Will return [`Err`] if `compression` is set to [`Compression::Unknown`] or an I/O error
/// occurred while writing to `output`.
///
#[allow(clippy::module_name_repetitions)]
pub async fn write_directories_async(
    output: &mut (impl AsyncWrite + Unpin + Send + AsyncSeekExt),
    all_entries: &[Entry],
    compression: Compression,
    overflow_strategy: Option<WriteDirsOverflowStrategy>,
) -> Result<Vec<u8>> {
    write_directories_impl_async(output, all_entries, compression, overflow_strategy).await
}

#[duplicate_item(
    fn_name                            async   SeekFrom                input_traits                                      add_await(code) write_directory(directory, output, compression);
    [only_leaf_pointer_strategy]       []      [std::io::SeekFrom]     [(impl Write + Seek)]                             [code]          [directory.to_writer(output, compression)];
    [only_leaf_pointer_strategy_async] [async] [futures::io::SeekFrom] [(impl AsyncWrite + Unpin + Send + AsyncSeekExt)] [code.await]    [directory.to_async_writer(output, compression).await];
)]
async fn fn_name(
    output: &mut input_traits,
    root_dir_start: SeekFrom,
    all_entries: &[Entry],
    compression: Compression,
    start_size: Option<usize>,
) -> Result<Vec<u8>> {
    let mut leaf_size = start_size.unwrap_or(4096);

    loop {
        let mut root_entries = Vec::<Entry>::new();

        let mut leaf_dir_bytes = Vec::<u8>::new();
        let mut leaf_dir_writer = Cursor::new(&mut leaf_dir_bytes);

        for entries in all_entries.chunks(leaf_size) {
            if entries.is_empty() {
                continue;
            }

            let leaf_dir = Directory::from(entries.to_vec());
            let offset = leaf_dir_writer.stream_position()?;
            leaf_dir.to_writer(&mut leaf_dir_writer, compression)?;
            #[allow(clippy::cast_possible_truncation)]
            let length = (leaf_dir_writer.stream_position()? - offset) as u32;

            root_entries.push(Entry {
                tile_id: entries[0].tile_id,
                length,
                offset,
                run_length: 0,
            });
        }

        let root_directory = Directory::from(root_entries);

        let start_pos = add_await([output.seek(root_dir_start)])?;
        write_directory([root_directory], [output], [compression])?;
        let root_directory_length = add_await([output.stream_position()])? - start_pos;

        if root_directory_length <= u64::from(MAX_ROOT_DIR_LENGTH) {
            return Ok(leaf_dir_bytes);
        }

        leaf_size *= 2;
    }
}
