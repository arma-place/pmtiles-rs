use duplicate::duplicate_item;
use futures::{AsyncRead, AsyncReadExt, AsyncSeekExt};
use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
    io::{Cursor, Error, ErrorKind, Read, Result, Seek},
};

use ahash::{AHasher, RandomState};

use crate::{Directory, Entry};

#[derive(Debug)]
enum TileManagerTile {
    Hash(u64),
    OffsetLength(u64, u32),
}

pub struct FinishResult {
    pub data: Vec<u8>,
    pub num_addressed_tiles: u64,
    pub num_tile_entries: u64,
    pub num_tile_content: u64,
    pub directory: Directory,
}

#[derive(Debug)]
pub struct TileManager<R> {
    /// hash of tile -> bytes of tile
    data_by_hash: HashMap<u64, Vec<u8>>,

    /// tile_id -> hash of tile
    tile_by_id: HashMap<u64, TileManagerTile>,

    /// hash of tile -> ids with this hash
    ids_by_hash: HashMap<u64, HashSet<u64>, RandomState>,

    reader: Option<R>,
}

impl<R> TileManager<R> {
    pub fn new(reader: Option<R>) -> Self {
        Self {
            data_by_hash: HashMap::default(),
            tile_by_id: HashMap::default(),
            ids_by_hash: HashMap::default(),
            reader,
        }
    }

    fn calculate_hash(value: &impl Hash) -> u64 {
        let mut hasher = AHasher::default();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Add tile to writer
    pub fn add_tile(&mut self, tile_id: u64, data: impl Into<Vec<u8>>) {
        let vec: Vec<u8> = data.into();

        // remove tile just to make sure that there
        // are no unreachable tiles
        self.remove_tile(tile_id);

        let hash = Self::calculate_hash(&vec);

        self.tile_by_id.insert(tile_id, TileManagerTile::Hash(hash));

        self.data_by_hash.insert(hash, vec);

        self.ids_by_hash
            .entry(hash)
            .or_insert_with(HashSet::new)
            .insert(tile_id);
    }

    pub(crate) fn add_offset_tile(&mut self, tile_id: u64, offset: u64, length: u32) {
        self.tile_by_id
            .insert(tile_id, TileManagerTile::OffsetLength(offset, length));
    }

    /// Remove tile from writer
    pub fn remove_tile(&mut self, tile_id: u64) -> bool {
        match self.tile_by_id.remove(&tile_id) {
            None => false, // tile was not found
            Some(tile) => {
                let TileManagerTile::Hash(hash) = tile else { return true; };

                // find set which includes all ids which have this hash
                let ids_with_hash = self.ids_by_hash.entry(hash).or_default();

                // remove current id from set
                ids_with_hash.remove(&tile_id);

                // delete data for this hash, if there are
                // no other ids that reference this hash
                if ids_with_hash.is_empty() {
                    self.data_by_hash.remove(&hash);
                    self.ids_by_hash.remove(&hash);
                }

                true
            }
        }
    }

    pub fn get_tile_ids(&self) -> Vec<&u64> {
        self.tile_by_id.keys().collect()
    }

    pub fn num_addressed_tiles(&self) -> usize {
        self.tile_by_id.len()
    }

    fn push_entry(entries: &mut Vec<Entry>, tile_id: u64, offset: u64, length: u32) {
        if let Some(last) = entries.last_mut() {
            if tile_id == last.tile_id + u64::from(last.run_length)
                && last.offset == offset
                && last.length == length
            {
                last.run_length += 1;
                return;
            }
        }

        entries.push(Entry {
            tile_id,
            offset,
            length,
            run_length: 1,
        });
    }
}

#[duplicate_item(
    async    add_await(code) RTraits                                                  SeekFrom                get_tile_content         get_tile         finish;
    []       [code]          [Read + Seek]                                            [std::io::SeekFrom]     [get_tile_content]       [get_tile]       [finish];
    [async]  [code.await]    [AsyncRead + AsyncReadExt + Send + Unpin + AsyncSeekExt] [futures::io::SeekFrom] [get_tile_content_async] [get_tile_async] [finish_async];
)]
impl<R: RTraits> TileManager<R> {
    async fn get_tile_content(
        reader: &mut Option<R>,
        data_by_hash: &HashMap<u64, Vec<u8>>,
        tile: &TileManagerTile,
    ) -> Result<Option<Vec<u8>>> {
        match tile {
            TileManagerTile::Hash(hash) => Ok(data_by_hash.get(hash).cloned()),
            TileManagerTile::OffsetLength(offset, length) => match reader {
                Some(r) => {
                    add_await([r.seek(SeekFrom::Start(*offset))])?;
                    let mut buf = vec![0; *length as usize];
                    add_await([r.read_exact(&mut buf)])?;
                    Ok(Some(buf))
                }
                None => Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "Tried to read from non-existent reader",
                )),
            },
        }
    }

    pub async fn get_tile(&mut self, tile_id: u64) -> Result<Option<Vec<u8>>> {
        match self.tile_by_id.get(&tile_id) {
            None => Ok(None),
            Some(tile) => add_await([Self::get_tile_content(
                &mut self.reader,
                &self.data_by_hash,
                tile,
            )]),
        }
    }

    pub async fn finish(mut self) -> Result<FinishResult> {
        type OffsetLen = (u64, u32);

        let mut id_tile = self
            .tile_by_id
            .into_iter()
            .collect::<Vec<(u64, TileManagerTile)>>();
        id_tile.sort_by(|a, b| a.0.cmp(&b.0));

        let mut entries = Vec::<Entry>::new();
        let mut data = Vec::<u8>::new();

        let mut num_addressed_tiles: u64 = 0;
        let mut num_tile_content: u64 = 0;

        // hash => offset+length
        let mut offset_length_map = HashMap::<u64, OffsetLen, RandomState>::default();

        for (tile_id, tile) in id_tile {
            let Some(mut tile_data) = add_await([Self::get_tile_content(&mut self.reader, &self.data_by_hash, &tile)])? else { continue; };

            let hash = if let TileManagerTile::Hash(h) = tile {
                h
            } else {
                Self::calculate_hash(&tile_data)
            };

            num_addressed_tiles += 1;

            if let Some((offset, length)) = offset_length_map.get(&hash) {
                Self::push_entry(&mut entries, tile_id, *offset, *length);
            } else {
                let offset = data.len() as u64;

                #[allow(clippy::cast_possible_truncation)]
                let length = tile_data.len() as u32;

                data.append(&mut tile_data);
                num_tile_content += 1;

                Self::push_entry(&mut entries, tile_id, offset, length);
                offset_length_map.insert(hash, (offset, length));
            }
        }

        let num_tile_entries = entries.len() as u64;

        Ok(FinishResult {
            data,
            directory: entries.into(),
            num_addressed_tiles,
            num_tile_content,
            num_tile_entries,
        })
    }
}

impl Default for TileManager<Cursor<&[u8]>> {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_tile_none() -> Result<()> {
        let mut manager = TileManager::default();

        assert!(manager.get_tile(42)?.is_none());

        Ok(())
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn test_get_tile_some() -> Result<()> {
        let mut manager = TileManager::default();

        let contents = vec![1u8, 3, 3, 7, 4, 2];

        manager.add_tile(42, contents.clone());

        let opt = manager.get_tile(42)?;

        assert!(opt.is_some());
        assert_eq!(opt.unwrap(), contents);

        Ok(())
    }

    #[test]
    fn test_add_tile() {
        let mut manager = TileManager::default();

        manager.add_tile(1337, vec![1, 3, 3, 7, 4, 2]);
        assert_eq!(manager.data_by_hash.len(), 1);

        manager.add_tile(42, vec![4, 2, 1, 3, 3, 7]);
        assert_eq!(manager.data_by_hash.len(), 2);
    }

    #[test]
    fn test_add_tile_dedup() {
        let mut manager = TileManager::default();

        let contents = vec![1u8, 3, 3, 7, 4, 2];

        manager.add_tile(42, contents.clone());
        manager.add_tile(1337, contents);

        assert_eq!(manager.data_by_hash.len(), 1);
    }

    #[test]
    fn test_add_tile_update() {
        let mut manager = TileManager::default();

        manager.add_tile(1337, vec![1, 3, 3, 7, 4, 2]);
        assert_eq!(manager.data_by_hash.len(), 1);
        assert_eq!(manager.tile_by_id.len(), 1);
        assert_eq!(manager.ids_by_hash.len(), 1);

        manager.add_tile(1337, vec![4, 2, 1, 3, 3, 7]);
        assert_eq!(manager.data_by_hash.len(), 1);
        assert_eq!(manager.tile_by_id.len(), 1);
        assert_eq!(manager.ids_by_hash.len(), 1);
    }

    #[test]
    fn test_remove_tile() {
        let mut manager = TileManager::default();

        manager.add_tile(42, vec![1u8, 3, 3, 7, 4, 2]);

        assert_eq!(manager.tile_by_id.len(), 1);
        assert_eq!(manager.data_by_hash.len(), 1);
        assert_eq!(manager.ids_by_hash.len(), 1);

        assert!(manager.remove_tile(42));

        assert_eq!(manager.tile_by_id.len(), 0);
        assert_eq!(manager.data_by_hash.len(), 0);
        assert_eq!(manager.ids_by_hash.len(), 0);
    }

    #[test]
    fn test_remove_tile_non_existent() {
        let mut manager = TileManager::default();

        let removed = manager.remove_tile(42);

        assert!(!removed);
    }

    #[test]
    fn test_remove_tile_dupe() {
        let mut manager = TileManager::default();

        let contents = vec![1u8, 3, 3, 7, 4, 2];
        manager.add_tile(69, contents.clone());
        manager.add_tile(42, contents.clone());
        manager.add_tile(1337, contents);

        assert_eq!(manager.data_by_hash.len(), 1);

        manager.remove_tile(1337);
        assert_eq!(manager.data_by_hash.len(), 1);
        assert_eq!(manager.ids_by_hash.len(), 1);

        manager.remove_tile(69);
        assert_eq!(manager.data_by_hash.len(), 1);
        assert_eq!(manager.ids_by_hash.len(), 1);

        manager.remove_tile(42);
        assert_eq!(manager.data_by_hash.len(), 0);
        assert_eq!(manager.ids_by_hash.len(), 0);
    }

    #[test]
    fn test_finish() -> Result<()> {
        let mut manager = TileManager::default();

        let tile_0 = vec![0u8, 3, 3, 7, 4, 2];
        let tile_42 = vec![42u8, 3, 3, 7, 4, 2];
        let tile_1337 = vec![1u8, 3, 3, 7, 4, 2];

        manager.add_tile(0, tile_0.clone());
        manager.add_tile(42, tile_42.clone());
        manager.add_tile(1337, tile_1337.clone());

        let result = manager.finish()?;
        let data = result.data;
        let directory = result.directory;

        assert_eq!(data.len(), tile_0.len() + tile_42.len() + tile_1337.len());
        assert_eq!(directory.len(), 3);
        assert_eq!(result.num_tile_entries, 3);
        assert_eq!(result.num_addressed_tiles, 3);
        assert_eq!(result.num_tile_content, 3);

        Ok(())
    }

    #[test]
    fn test_finish_dupes() -> Result<()> {
        let mut manager = TileManager::default();

        let content = vec![1u8, 3, 3, 7, 4, 2];

        manager.add_tile(0, content.clone());
        manager.add_tile(1, vec![1]);
        manager.add_tile(1337, content.clone());

        let result = manager.finish()?;
        let data = result.data;
        let directory = result.directory;

        assert_eq!(data.len(), content.len() + 1);
        assert_eq!(directory.len(), 3);
        assert_eq!(result.num_tile_entries, 3);
        assert_eq!(result.num_addressed_tiles, 3);
        assert_eq!(result.num_tile_content, 2);
        assert_eq!(directory[0].offset, directory[2].offset);
        assert_eq!(directory[0].length, directory[2].length);

        Ok(())
    }

    #[test]
    fn test_finish_dupes_reader() -> Result<()> {
        let reader = Cursor::new(vec![1u8, 3, 3, 7, 1, 3, 3, 7]);

        let mut manager = TileManager::new(Some(reader));

        manager.add_offset_tile(0, 0, 4);
        manager.add_offset_tile(5, 0, 4);
        manager.add_offset_tile(10, 4, 4);
        manager.add_tile(15, vec![1, 3, 3, 7]);
        manager.add_tile(20, vec![1, 3, 3, 7]);

        let result = manager.finish()?;
        let data = result.data;
        let directory = result.directory;

        assert_eq!(data.len(), 4);
        assert_eq!(directory.len(), 5);
        assert_eq!(result.num_tile_entries, 5);
        assert_eq!(result.num_addressed_tiles, 5);
        assert_eq!(result.num_tile_content, 1);
        assert_eq!(directory[0].offset, 0);
        assert_eq!(directory[0].length, 4);
        assert_eq!(directory[1].offset, 0);
        assert_eq!(directory[1].length, 4);
        assert_eq!(directory[2].offset, 0);
        assert_eq!(directory[2].length, 4);
        assert_eq!(directory[3].offset, 0);
        assert_eq!(directory[3].length, 4);
        assert_eq!(directory[4].offset, 0);
        assert_eq!(directory[4].length, 4);

        Ok(())
    }

    #[test]
    fn test_finish_run_length() -> Result<()> {
        let mut manager = TileManager::default();

        let content = vec![1u8, 3, 3, 7, 4, 2];

        manager.add_tile(0, content.clone());
        manager.add_tile(1, content.clone());
        manager.add_tile(2, content.clone());
        manager.add_tile(3, content.clone());
        manager.add_tile(4, content);

        let result = manager.finish()?;
        let directory = result.directory;

        assert_eq!(directory.len(), 1);
        assert_eq!(directory[0].run_length, 5);
        assert_eq!(result.num_tile_entries, 1);
        assert_eq!(result.num_addressed_tiles, 5);
        assert_eq!(result.num_tile_content, 1);

        Ok(())
    }

    #[test]
    fn test_finish_clustered() -> Result<()> {
        let mut manager = TileManager::default();

        // add tiles in random order
        manager.add_tile(42, vec![42]);
        manager.add_tile(1337, vec![13, 37]);
        manager.add_tile(69, vec![69]);
        manager.add_tile(1, vec![1]);

        let result = manager.finish()?;
        let directory = result.directory;

        // make sure entries are in asc order
        assert_eq!(directory[0].tile_id, 1);
        assert_eq!(directory[1].tile_id, 42);
        assert_eq!(directory[2].tile_id, 69);
        assert_eq!(directory[3].tile_id, 1337);

        // make sure data offsets are in asc order (clustered)
        assert!(directory[1].offset > directory[0].offset);
        assert!(directory[2].offset > directory[1].offset);
        assert!(directory[3].offset > directory[2].offset);

        Ok(())
    }
}
