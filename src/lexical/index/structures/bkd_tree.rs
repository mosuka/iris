//! Simple BKD Tree implementation for numeric range queries.
//!
//! This is a simplified version of Apache Lucene's BKD Tree data structure,
//! optimized for 1-dimensional numeric range filtering.

use crate::error::Result;
use crate::storage::structured::{StructReader, StructWriter};
use crate::storage::{Storage, StorageInput, StorageOutput};
use std::cmp::Ordering;
use std::io::SeekFrom;
use std::sync::Arc;

/// Trait for BKD Tree implementations (in-memory or disk-based).
pub trait BKDTree: Send + Sync + std::fmt::Debug {
    /// Perform a range search.
    fn range_search(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        include_min: bool,
        include_max: bool,
    ) -> Result<Vec<u64>>;
}

/// Magic number for BKD Tree files "BKDT"
pub const BKD_MAGIC: u32 = 0x54444B42;

/// Version 1
pub const BKD_VERSION: u32 = 1;

/// BKD Tree File Header
#[derive(Debug, Clone)]
pub struct BKDFileHeader {
    pub magic: u32,
    pub version: u32,
    pub num_dims: u32,
    pub bytes_per_dim: u32,
    pub total_point_count: u64,
    pub num_blocks: u64,
    pub min_value: f64,
    pub max_value: f64,
    pub index_start_offset: u64,
    pub root_node_offset: u64,
}

/// Writer for BKD Trees.
pub struct BKDWriter<W: StorageOutput> {
    writer: StructWriter<W>,
    block_size: usize,
    num_blocks: u64,
    min_value: f64,
    max_value: f64,
    index_nodes: Vec<IndexNode>,
}

/// Internal index node for navigation
#[derive(Debug, Clone)]
struct IndexNode {
    split_value: f64,
    left_offset: u64,
    right_offset: u64,
    // Helper to back-patch offsets during writing
    left_child_idx: Option<usize>,
    right_child_idx: Option<usize>,
}

impl<W: StorageOutput> BKDWriter<W> {
    pub fn new(writer: W) -> Self {
        BKDWriter {
            writer: StructWriter::new(writer),
            block_size: 512,
            num_blocks: 0,
            min_value: f64::MAX,
            max_value: f64::MIN,
            index_nodes: Vec::new(),
        }
    }

    /// Set custom block size
    pub fn with_block_size(mut self, block_size: usize) -> Self {
        self.block_size = block_size;
        self
    }

    /// Write a BKD tree from sorted entries.
    pub fn write(&mut self, entries: &[(f64, u64)]) -> Result<()> {
        if entries.is_empty() {
            // Write basic header for empty tree
            self.write_header(0, 0, 0)?;
            return Ok(());
        }

        self.min_value = entries.first().unwrap().0;
        self.max_value = entries.last().unwrap().0;
        let total_count = entries.len() as u64;

        // Reserve space for header
        self.writer.write_u32(0)?; // Placeholder for Magic
        self.writer.seek(SeekFrom::Start(80))?; // Reserve 80 bytes for header (approx)

        // Recursively build tree and write leaf blocks
        let root_node_idx = self.build_subtree(entries)?;

        // Write index section after all leaves
        let index_start_offset = self.writer.stream_position()?;
        self.write_index()?;

        let root_node_offset = if let Some(idx) = root_node_idx {
            // Index nodes are written sequentially starting from index_start_offset.
            // Each node is 24 bytes (f64 + u64 + u64)
            index_start_offset + (idx as u64) * 24
        } else {
            0
        };

        // Go back and write real header
        self.writer.seek(SeekFrom::Start(0))?;
        self.write_header(total_count, index_start_offset, root_node_offset)?;

        // Go back to end
        self.writer.seek(SeekFrom::End(0))?;

        Ok(())
    }

    fn write_header(&mut self, total_count: u64, index_start: u64, root_offset: u64) -> Result<()> {
        self.writer.write_u32(BKD_MAGIC)?;
        self.writer.write_u32(BKD_VERSION)?;
        self.writer.write_u32(1)?; // Num dims
        self.writer.write_u32(8)?; // Bytes per dim (f64)
        self.writer.write_u64(total_count)?;
        self.writer.write_u64(self.num_blocks)?;
        self.writer.write_f64(self.min_value)?;
        self.writer.write_f64(self.max_value)?;
        self.writer.write_u64(index_start)?;
        self.writer.write_u64(root_offset)?;
        Ok(())
    }

    /// Recursively build subtree, write leaves, and return index node index
    fn build_subtree(&mut self, entries: &[(f64, u64)]) -> Result<Option<usize>> {
        if entries.is_empty() {
            return Ok(None);
        }

        if entries.len() <= self.block_size {
            // Write leaf block
            self.write_leaf_block(entries)?;
            self.num_blocks += 1;
            return Ok(None); // Leaf has no index node
        }

        // Split
        let mid = entries.len() / 2;
        let split_value = entries[mid].0;

        let left_entries = &entries[..mid];
        let right_entries = &entries[mid..];

        // Recurse - we write leaves in post-order or similar?
        // Actually, to know offsets, we write leaves as we go.
        // But for the index, we need offsets of children.
        // If child is a leaf, offset is the leaf block offset.
        // If child is a node, offset is the node offset (which is written later).
        // This suggests we should write all leaves first, or handle offsets carefully.

        // Simpler approach for 1D:
        // Internal nodes are written AFTER all leaves.
        // We track the tree structure in memory (IndexNode vec) and then flatten it.

        let node_idx = self.index_nodes.len();
        self.index_nodes.push(IndexNode {
            split_value,
            left_offset: 0,  // Placeholder
            right_offset: 0, // Placeholder
            left_child_idx: None,
            right_child_idx: None,
        });

        // Current file position is where the Left child (leaf or subtree blocks) will start?
        // No, `build_subtree` writes leaves.
        // We need to capture the offset where the "Left Child" structure starts.
        // But wait, if Left Child is a Leaf, it's just a file offset.
        // If Left Child is an internal node, that node is written in the Index Section later.

        // Let's refine the Index Format.
        // The Index is keys + offsets.
        // Lucene packs the index.
        // Here we can just write the index array at the end.
        // The "offset" in a node points to:
        // 1. Another Index Node (if it's not a leaf child)
        // 2. A Leaf Block (if it is a leaf child)

        // To distinguish, we can use a high bit or simply check if entry count <= block_size?
        // No, at query time we traverse.

        // Let's store: `left_file_offset` and `right_file_offset`.
        // If the child is a Leaf, `file_offset` points to the block.
        // If the child is an Internal Node, `file_offset` points to that node in the Index Section.

        // Since we write Index Section at the end, we don't know its offsets yet.
        // However, we can write Index Nodes in a known order (e.g. valid array indices)
        // and base the offsets on `index_start_offset`.

        // Let's say:
        // Offset > index_start_offset => Internal Node
        // Offset < index_start_offset => Leaf Block

        // We must ensure this holds.
        // So we need to propagate whether the child was a Leaf or a Node.

        let left_file_pos_before = self.writer.stream_position()?;
        let left_child_node_idx = self.build_subtree(left_entries)?;
        let left_is_leaf = left_child_node_idx.is_none();

        // If left was a leaf, its offset is `left_file_pos_before`.
        // If left was a node, we will resolve its offset later (it will be in the index section).

        let right_file_pos_before = self.writer.stream_position()?;
        let right_child_node_idx = self.build_subtree(right_entries)?;
        let right_is_leaf = right_child_node_idx.is_none();

        // Update current node
        self.index_nodes[node_idx].left_child_idx = left_child_node_idx;
        self.index_nodes[node_idx].right_child_idx = right_child_node_idx;

        // Store the file offsets for leaves immediately
        if left_is_leaf {
            self.index_nodes[node_idx].left_offset = left_file_pos_before;
        }
        if right_is_leaf {
            self.index_nodes[node_idx].right_offset = right_file_pos_before;
        }

        Ok(Some(node_idx))
    }

    fn write_leaf_block(&mut self, entries: &[(f64, u64)]) -> Result<()> {
        let count = entries.len() as u32;
        self.writer.write_u32(count)?;

        // Write values
        for (val, _) in entries {
            self.writer.write_f64(*val)?;
        }

        // Write doc ids (Varint delta encoded could be better, but simple u64 for now)
        for (_, doc_id) in entries {
            self.writer.write_u64(*doc_id)?;
        }

        Ok(())
    }

    fn write_index(&mut self) -> Result<()> {
        let start_pos = self.writer.stream_position()?;

        // We need to allow mapping from node_idx to file_offset.
        // We can layout nodes sequentially: 0, 1, 2...
        // Node 0 is at start_pos. Node 1 is at start_pos + sizeof(Node)...

        // But the tree structure (left/right children) uses `node_idx`.
        // We need to convert `left_child_idx` to a file offset.

        let node_size = 8 + 8 + 8; // split(f64) + left(u64) + right(u64) = 24 bytes

        // First, resolve all offsets
        // We clone to allow mutation while iterating or just use indices.
        // Actually, straightforward:
        // offset(i) = start_pos + i * node_size

        for i in 0..self.index_nodes.len() {
            let left_idx = self.index_nodes[i].left_child_idx;
            if let Some(idx) = left_idx {
                self.index_nodes[i].left_offset = start_pos + (idx as u64) * node_size;
            }

            let right_idx = self.index_nodes[i].right_child_idx;
            if let Some(idx) = right_idx {
                self.index_nodes[i].right_offset = start_pos + (idx as u64) * node_size;
            }
        }

        // Write nodes
        for node in &self.index_nodes {
            self.writer.write_f64(node.split_value)?;
            self.writer.write_u64(node.left_offset)?;
            self.writer.write_u64(node.right_offset)?;
        }

        Ok(())
    }

    /// Finish writing and return the underlying writer.
    pub fn finish(self) -> Result<()> {
        self.writer.close()
    }
}

/// Reader for BKD Trees.
#[derive(Debug)]
pub struct BKDReader {
    header: BKDFileHeader,
    storage: Arc<dyn Storage>,
    path: String,
}

impl BKDReader {
    /// Open a BKD tree from storage and path.
    pub fn open(storage: Arc<dyn Storage>, path: &str) -> Result<Self> {
        let input = storage.open_input(path)?;
        let mut reader = StructReader::new(input)?;

        // Read header
        let magic = reader.read_u32()?;
        if magic != BKD_MAGIC {
            return Err(crate::error::SarissaError::storage(format!(
                "Invalid BKD magic: {:x}",
                magic
            )));
        }

        let version = reader.read_u32()?;
        let num_dims = reader.read_u32()?;
        let bytes_per_dim = reader.read_u32()?;
        let total_point_count = reader.read_u64()?;
        let num_blocks = reader.read_u64()?;
        let min_value = reader.read_f64()?;
        let max_value = reader.read_f64()?;
        let index_start_offset = reader.read_u64()?;
        let root_node_offset = reader.read_u64()?;

        let header = BKDFileHeader {
            magic,
            version,
            num_dims,
            bytes_per_dim,
            total_point_count,
            num_blocks,
            min_value,
            max_value,
            index_start_offset,
            root_node_offset,
        };

        Ok(BKDReader {
            header,
            storage,
            path: path.to_string(),
        })
    }

    fn visit_node<R: StorageInput>(
        &self,
        reader: &mut StructReader<R>,
        offset: u64,
        min: Option<f64>,
        max: Option<f64>,
        include_min: bool,
        include_max: bool,
        collector: &mut Vec<u64>,
    ) -> Result<()> {
        if offset < self.header.index_start_offset {
            return self.visit_leaf_block(
                reader,
                offset,
                min,
                max,
                include_min,
                include_max,
                collector,
            );
        }

        reader.seek(SeekFrom::Start(offset))?;
        let split_value = reader.read_f64()?;
        let left_offset = reader.read_u64()?;
        let right_offset = reader.read_u64()?;

        // Logic:
        // Left child contains values <= split_value.
        // If min <= split_value, go left.
        // We can be conservative: even if min > split_value but close? No.
        // If min is None, go left.
        // If min is strictly > split_value, we might skip left.
        // Note on strictly >: if min > split_value, we definitely don't need left.
        let go_left = min.map_or(true, |m| {
            if include_min {
                m <= split_value
            } else {
                // If exclusive min > split_value, skip left.
                // If exclusive min == split_value, do we need left?
                // Left has values <= split. So if we want > split, we don't need left.
                // So if min >= split_value (exclusive), we skip left?
                // Wait. split_value is just a divider.
                // Left child entries <= split_value.
                // Query: x > min.
                // If min == split_value, we want x > split_value.
                // Left child has x <= split_value. So no intersection.
                m < split_value
            }
        });

        if go_left {
            self.visit_node(
                reader,
                left_offset,
                min,
                max,
                include_min,
                include_max,
                collector,
            )?;
        }

        // Right child contains values >= split_value.
        // (Actually usually > split_value? Lucene implementation details might vary).
        // Assuming rights are potentially > split_value.
        // If max >= split_value, go right.
        let go_right = max.map_or(true, |m| {
            if include_max {
                m >= split_value
            } else {
                m > split_value
            }
        });

        if go_right {
            self.visit_node(
                reader,
                right_offset,
                min,
                max,
                include_min,
                include_max,
                collector,
            )?;
        }

        Ok(())
    }

    fn visit_leaf_block<R: StorageInput>(
        &self,
        reader: &mut StructReader<R>,
        offset: u64,
        min: Option<f64>,
        max: Option<f64>,
        include_min: bool,
        include_max: bool,
        collector: &mut Vec<u64>,
    ) -> Result<()> {
        reader.seek(SeekFrom::Start(offset))?;
        let count = reader.read_u32()?;

        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            values.push(reader.read_f64()?);
        }

        let mut doc_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            doc_ids.push(reader.read_u64()?);
        }

        for (val, doc_id) in values.iter().zip(doc_ids.iter()) {
            let gte_min = min.map_or(true, |m| if include_min { *val >= m } else { *val > m });
            let lte_max = max.map_or(true, |m| if include_max { *val <= m } else { *val < m });
            if gte_min && lte_max {
                collector.push(*doc_id);
            }
        }
        Ok(())
    }
}

impl BKDTree for BKDReader {
    /// Perform a range search.
    /// Perform a range search.
    fn range_search(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        include_min: bool,
        include_max: bool,
    ) -> Result<Vec<u64>> {
        if self.header.total_point_count == 0 {
            return Ok(Vec::new());
        }

        let mut doc_ids = Vec::new();
        let root_offset = self.header.root_node_offset;

        let input = self.storage.open_input(&self.path)?;
        let mut reader = StructReader::new(input)?;

        if root_offset == 0 && self.header.total_point_count > 0 {
            // Single leaf block case.
            // Assume block starts after header (80 bytes).
            self.visit_leaf_block(
                &mut reader,
                80,
                min,
                max,
                include_min,
                include_max,
                &mut doc_ids,
            )?;
        } else {
            self.visit_node(
                &mut reader,
                root_offset,
                min,
                max,
                include_min,
                include_max,
                &mut doc_ids,
            )?;
        }

        doc_ids.sort_unstable();
        doc_ids.dedup();

        Ok(doc_ids)
    }
}

/// A simple BKD Tree for efficient numeric range queries.
///
/// This implementation stores sorted (value, doc_id) pairs and provides
/// efficient range search capabilities using binary search.
#[derive(Debug, Clone)]
pub struct SimpleBKDTree {
    /// Sorted array of (value, doc_id) pairs.
    /// Sorted by value first, then by doc_id for stable ordering.
    sorted_entries: Vec<(f64, u64)>,

    /// Block size for chunked processing (similar to Lucene's 512-1024).
    block_size: usize,

    /// Field name this tree is built for.
    field_name: String,
}

impl BKDTree for SimpleBKDTree {
    fn range_search(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        include_min: bool,
        include_max: bool,
    ) -> Result<Vec<u64>> {
        Ok(self.range_search(min, max, include_min, include_max))
    }
}

impl SimpleBKDTree {
    /// Create a new BKD Tree from unsorted (value, doc_id) pairs.
    pub fn new(field_name: String, mut entries: Vec<(f64, u64)>) -> Self {
        // Sort by value first, then by doc_id for stable ordering
        entries.sort_by(|a, b| {
            a.0.partial_cmp(&b.0)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.1.cmp(&b.1))
        });

        SimpleBKDTree {
            sorted_entries: entries,
            block_size: 512, // Similar to Lucene's default
            field_name,
        }
    }

    /// Create an empty BKD Tree.
    pub fn empty(field_name: String) -> Self {
        SimpleBKDTree {
            sorted_entries: Vec::new(),
            block_size: 512,
            field_name,
        }
    }

    /// Get the field name this tree is built for.
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Get the number of entries in this tree.
    pub fn size(&self) -> usize {
        self.sorted_entries.len()
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.sorted_entries.is_empty()
    }

    /// Perform a range search and return matching document IDs.
    ///
    /// # Arguments
    /// * `min` - Minimum value (inclusive, or None for unbounded)
    /// * `max` - Maximum value (inclusive, or None for unbounded)
    ///
    /// # Returns
    /// Vector of document IDs that match the range criteria.
    pub fn range_search(
        &self,
        min: Option<f64>,
        max: Option<f64>,
        include_min: bool,
        include_max: bool,
    ) -> Vec<u64> {
        if self.sorted_entries.is_empty() {
            return Vec::new();
        }

        // Find the range of indices that match our criteria
        let start_idx = match min {
            Some(min_val) => self.find_first_gte(min_val),
            None => 0,
        };

        let end_idx = match max {
            Some(max_val) => self.find_last_lte(max_val),
            None => self.sorted_entries.len().saturating_sub(1),
        };

        if start_idx > end_idx {
            return Vec::new();
        }

        // Extract document IDs from the matching range
        let mut doc_ids = Vec::new();
        for i in start_idx..=end_idx {
            let (val, doc_id) = self.sorted_entries[i];

            let check_min = min.map_or(true, |m| if include_min { val >= m } else { val > m });
            let check_max = max.map_or(true, |m| if include_max { val <= m } else { val < m });

            if check_min && check_max {
                doc_ids.push(doc_id);
            }
        }

        // Sort document IDs for consistent ordering
        doc_ids.sort_unstable();
        doc_ids.dedup(); // Remove duplicates if any

        doc_ids
    }

    /// Find the first index where value >= target using binary search.
    fn find_first_gte(&self, target: f64) -> usize {
        let mut left = 0;
        let mut right = self.sorted_entries.len();

        while left < right {
            let mid = left + (right - left) / 2;
            if self.sorted_entries[mid].0 >= target {
                right = mid;
            } else {
                left = mid + 1;
            }
        }

        left
    }

    /// Find the last index where value <= target using binary search.
    fn find_last_lte(&self, target: f64) -> usize {
        if self.sorted_entries.is_empty() {
            return 0;
        }

        let mut left = 0;
        let mut right = self.sorted_entries.len();

        while left < right {
            let mid = left + (right - left) / 2;
            if self.sorted_entries[mid].0 <= target {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        if left > 0 { left - 1 } else { 0 }
    }

    /// Get statistics about this BKD Tree.
    pub fn stats(&self) -> BKDTreeStats {
        let num_blocks = self.sorted_entries.len().div_ceil(self.block_size);
        let min_value = self.sorted_entries.first().map(|(v, _)| *v);
        let max_value = self.sorted_entries.last().map(|(v, _)| *v);

        BKDTreeStats {
            field_name: self.field_name.clone(),
            total_entries: self.sorted_entries.len(),
            num_blocks,
            block_size: self.block_size,
            min_value,
            max_value,
        }
    }
}

/// Statistics about a BKD Tree.
#[derive(Debug, Clone)]
pub struct BKDTreeStats {
    /// Field name this tree is built for.
    pub field_name: String,
    /// Total number of entries.
    pub total_entries: usize,
    /// Number of blocks.
    pub num_blocks: usize,
    /// Block size.
    pub block_size: usize,
    /// Minimum value in the tree.
    pub min_value: Option<f64>,
    /// Maximum value in the tree.
    pub max_value: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    use std::sync::Arc;

    fn create_test_tree() -> SimpleBKDTree {
        let entries = vec![
            (1.0, 10), // doc 10, value 1.0
            (3.0, 20), // doc 20, value 3.0
            (2.0, 30), // doc 30, value 2.0
            (5.0, 40), // doc 40, value 5.0
            (4.0, 50), // doc 50, value 4.0
            (1.5, 60), // doc 60, value 1.5
        ];
        SimpleBKDTree::new("test_field".to_string(), entries)
    }

    #[test]
    fn test_bkd_writer_reader() {
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let mut entries = vec![
            (1.0, 10),
            (3.0, 20),
            (2.0, 30),
            (5.0, 40),
            (4.0, 50),
            (1.5, 60),
        ];
        entries.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(a.1.cmp(&b.1)));

        // Write
        {
            let output = storage.create_output("test.bkd").unwrap();
            let mut writer = BKDWriter::new(output);
            writer.write(&entries).unwrap();
            writer.finish().unwrap();
        }

        // Read
        {
            let reader = BKDReader::open(storage.clone(), "test.bkd").unwrap();

            // Verify header
            assert_eq!(reader.header.total_point_count, 6);

            // Test Range Search [2.0, 4.0] -> 20, 30, 50
            let results = reader
                .range_search(Some(2.0), Some(4.0), true, true)
                .unwrap();
            let mut expected = vec![30, 20, 50];
            expected.sort();
            assert_eq!(results, expected);
        }
    }

    #[test]
    fn test_bkd_tree_creation() {
        let tree = create_test_tree();

        assert_eq!(tree.size(), 6);
        assert_eq!(tree.field_name(), "test_field");
        assert!(!tree.is_empty());

        // Verify entries are sorted by value
        let expected_order = vec![
            (1.0, 10),
            (1.5, 60),
            (2.0, 30),
            (3.0, 20),
            (4.0, 50),
            (5.0, 40),
        ];
        assert_eq!(tree.sorted_entries, expected_order);
    }

    #[test]
    fn test_empty_tree() {
        let tree = SimpleBKDTree::empty("empty_field".to_string());

        assert_eq!(tree.size(), 0);
        assert!(tree.is_empty());
        assert_eq!(
            tree.range_search(Some(1.0), Some(5.0), true, true),
            Vec::<u64>::new()
        );
    }

    #[test]
    fn test_range_search_exact_bounds() {
        let tree = create_test_tree();

        // Range [2.0, 4.0] should match docs 30, 20, 50
        let results = tree.range_search(Some(2.0), Some(4.0), true, true);
        let mut expected = vec![30, 20, 50]; // docs with values 2.0, 3.0, 4.0
        expected.sort();

        assert_eq!(results, expected);
    }

    #[test]
    fn test_range_search_partial_bounds() {
        let tree = create_test_tree();

        // Range [3.0, None] should match docs 20, 50, 40
        let results = tree.range_search(Some(3.0), None, true, true);
        let mut expected = vec![20, 50, 40]; // docs with values 3.0, 4.0, 5.0
        expected.sort();

        assert_eq!(results, expected);

        // Range [None, 2.0] should match docs 10, 60, 30
        let results = tree.range_search(None, Some(2.0), true, true);
        let mut expected = vec![10, 60, 30]; // docs with values 1.0, 1.5, 2.0
        expected.sort();

        assert_eq!(results, expected);
    }

    #[test]
    fn test_range_search_no_bounds() {
        let tree = create_test_tree();

        // Range [None, None] should match all docs
        let results = tree.range_search(None, None, true, true);
        let mut expected = vec![10, 20, 30, 40, 50, 60];
        expected.sort();

        assert_eq!(results, expected);
    }

    #[test]
    fn test_range_search_no_matches() {
        let tree = create_test_tree();

        // Range [10.0, 20.0] should match no docs
        let results = tree.range_search(Some(10.0), Some(20.0), true, true);
        assert_eq!(results, Vec::<u64>::new());

        // Range [2.5, 2.5] should match no docs (no exact match)
        let results = tree.range_search(Some(2.5), Some(2.5), true, true);
        assert_eq!(results, Vec::<u64>::new());
    }

    #[test]
    fn test_range_search_single_value() {
        let tree = create_test_tree();

        // Range [3.0, 3.0] should match doc 20
        let results = tree.range_search(Some(3.0), Some(3.0), true, true);
        assert_eq!(results, vec![20]);
    }

    #[test]
    fn test_stats() {
        let tree = create_test_tree();
        let stats = tree.stats();

        assert_eq!(stats.field_name, "test_field");
        assert_eq!(stats.total_entries, 6);
        assert_eq!(stats.block_size, 512);
        assert_eq!(stats.min_value, Some(1.0));
        assert_eq!(stats.max_value, Some(5.0));
    }

    #[test]
    fn test_binary_search_functions() {
        let tree = create_test_tree();

        // Test find_first_gte
        assert_eq!(tree.find_first_gte(0.5), 0); // Before all values
        assert_eq!(tree.find_first_gte(1.0), 0); // Exact match first
        assert_eq!(tree.find_first_gte(1.2), 1); // Between values
        assert_eq!(tree.find_first_gte(3.0), 3); // Exact match middle
        assert_eq!(tree.find_first_gte(6.0), 6); // After all values

        // Test find_last_lte
        assert_eq!(tree.find_last_lte(0.5), 0); // Before all values
        assert_eq!(tree.find_last_lte(1.0), 0); // Exact match first
        assert_eq!(tree.find_last_lte(1.2), 0); // Between values
        assert_eq!(tree.find_last_lte(3.0), 3); // Exact match middle
        assert_eq!(tree.find_last_lte(6.0), 5); // After all values
    }
}
