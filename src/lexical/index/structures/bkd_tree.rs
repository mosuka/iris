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
        mins: &[Option<f64>],
        maxs: &[Option<f64>],
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
    pub min_values: Vec<f64>,
    pub max_values: Vec<f64>,
    pub index_start_offset: u64,
    pub root_node_offset: u64,
}

/// Writer for BKD Trees.
pub struct BKDWriter<W: StorageOutput> {
    writer: StructWriter<W>,
    block_size: usize,
    num_blocks: u64,
    num_dims: u32,
    min_values: Vec<f64>,
    max_values: Vec<f64>,
    index_nodes: Vec<IndexNode>,
}

/// Internal index node for navigation
#[derive(Debug, Clone)]
struct IndexNode {
    split_dim: u32,
    split_value: f64,
    left_offset: u64,
    right_offset: u64,
    // Helper to back-patch offsets during writing
    left_child_idx: Option<usize>,
    right_child_idx: Option<usize>,
}

impl<W: StorageOutput> BKDWriter<W> {
    pub fn new(writer: W, num_dims: u32) -> Self {
        BKDWriter {
            writer: StructWriter::new(writer),
            block_size: 512,
            num_blocks: 0,
            num_dims,
            min_values: vec![f64::MAX; num_dims as usize],
            max_values: vec![f64::MIN; num_dims as usize],
            index_nodes: Vec::new(),
        }
    }

    /// Set custom block size
    pub fn with_block_size(mut self, block_size: usize) -> Self {
        self.block_size = block_size;
        self
    }

    /// Write a BKD tree from entries.
    pub fn write(&mut self, entries: &[(Vec<f64>, u64)]) -> Result<()> {
        if entries.is_empty() {
            // Write basic header for empty tree
            self.write_header(0, 0, 0)?;
            return Ok(());
        }

        // Calculate global min/max
        for (vals, _) in entries {
            for (i, &val) in vals.iter().enumerate() {
                self.min_values[i] = self.min_values[i].min(val);
                self.max_values[i] = self.max_values[i].max(val);
            }
        }

        let total_count = entries.len() as u64;

        // Reserve space for header:
        // Magic(4) + Version(4) + num_dims(4) + bytes_per_dim(4) + total_count(8) + num_blocks(8)
        // + min_values(num_dims * 8) + max_values(num_dims * 8) + index_start(8) + root_offset(8)
        let header_size = 4 + 4 + 4 + 4 + 8 + 8 + (self.num_dims as u64 * 8 * 2) + 8 + 8;

        self.writer.write_u32(0)?; // Placeholder
        self.writer.seek(SeekFrom::Start(header_size))?;

        // Recursively build tree and write leaf blocks
        let mut mutable_entries = entries.to_vec();
        let root_node_idx = self.build_subtree(&mut mutable_entries, 0)?;

        // Write index section after all leaves
        let index_start_offset = self.writer.stream_position()?;
        self.write_index()?;

        // Root node size calculation needs to be updated.
        // Node: split_dim(4) + split_value(8) + left_offset(8) + right_offset(8) = 28 bytes
        let node_size = 4 + 8 + 8 + 8;

        let root_node_offset = if let Some(idx) = root_node_idx {
            index_start_offset + (idx as u64) * node_size
        } else {
            header_size
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
        self.writer.write_u32(self.num_dims)?;
        self.writer.write_u32(8)?; // Bytes per dim (f64)
        self.writer.write_u64(total_count)?;
        self.writer.write_u64(self.num_blocks)?;
        for &v in &self.min_values {
            self.writer.write_f64(v)?;
        }
        for &v in &self.max_values {
            self.writer.write_f64(v)?;
        }
        self.writer.write_u64(index_start)?;
        self.writer.write_u64(root_offset)?;
        Ok(())
    }

    /// Recursively build subtree, write leaves, and return index node index
    fn build_subtree(
        &mut self,
        entries: &mut [(Vec<f64>, u64)],
        depth: u32,
    ) -> Result<Option<usize>> {
        if entries.is_empty() {
            return Ok(None);
        }

        if entries.len() <= self.block_size {
            // Write leaf block
            self.write_leaf_block(entries)?;
            self.num_blocks += 1;
            return Ok(None);
        }

        // Split dimension: round robin
        let split_dim = depth % self.num_dims;

        // Sort by split dimension to find median
        entries.sort_by(|a, b| {
            a.0[split_dim as usize]
                .partial_cmp(&b.0[split_dim as usize])
                .unwrap_or(Ordering::Equal)
        });

        // Recurse - we write leaves in post-order or similar?
        // Actually, to know offsets, we write leaves as we go.
        // But for the index, we need offsets of children.
        // If child is a leaf, offset is the leaf block offset.
        // If child is a node, offset is the node offset (which is written later).
        // This suggests we should write all leaves first, or handle offsets carefully.

        // Simpler approach for 1D:
        // Internal nodes are written AFTER all leaves.
        // We track the tree structure in memory (IndexNode vec) and then flatten it.

        let mid = entries.len() / 2;
        let (left_entries, right_entries) = entries.split_at_mut(mid);
        let split_value = right_entries[0].0[split_dim as usize];

        // Record current node
        let node_idx = self.index_nodes.len();
        self.index_nodes.push(IndexNode {
            split_dim,
            split_value,
            left_offset: 0,
            right_offset: 0,
            left_child_idx: None,
            right_child_idx: None,
        });

        let left_file_pos_before = self.writer.stream_position()?;
        let left_child_node_idx = self.build_subtree(left_entries, depth + 1)?;
        let left_is_leaf = left_child_node_idx.is_none();

        let right_file_pos_before = self.writer.stream_position()?;
        let right_child_node_idx = self.build_subtree(right_entries, depth + 1)?;
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

    fn write_leaf_block(&mut self, entries: &[(Vec<f64>, u64)]) -> Result<()> {
        let count = entries.len() as u32;
        self.writer.write_u32(count)?;

        // Write values for all dimensions
        for (vals, _) in entries {
            for &val in vals {
                self.writer.write_f64(val)?;
            }
        }

        // Write doc ids
        for (_, doc_id) in entries {
            self.writer.write_u64(*doc_id)?;
        }

        Ok(())
    }

    fn write_index(&mut self) -> Result<()> {
        let start_pos = self.writer.stream_position()?;
        let node_size = 4 + 8 + 8 + 8; // split_dim + split_value + left + right = 28 bytes

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
            self.writer.write_u32(node.split_dim)?;
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
            return Err(crate::error::IrisError::storage(format!(
                "Invalid BKD magic: {:x}",
                magic
            )));
        }

        let version = reader.read_u32()?;
        let num_dims = reader.read_u32()?;
        let bytes_per_dim = reader.read_u32()?;
        let total_point_count = reader.read_u64()?;
        let num_blocks = reader.read_u64()?;
        let mut min_values = Vec::with_capacity(num_dims as usize);
        for _ in 0..num_dims {
            min_values.push(reader.read_f64()?);
        }
        let mut max_values = Vec::with_capacity(num_dims as usize);
        for _ in 0..num_dims {
            max_values.push(reader.read_f64()?);
        }
        let index_start_offset = reader.read_u64()?;
        let root_node_offset = reader.read_u64()?;

        let header = BKDFileHeader {
            magic,
            version,
            num_dims,
            bytes_per_dim,
            total_point_count,
            num_blocks,
            min_values,
            max_values,
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
        ctx: &QueryContext,
        collector: &mut Vec<u64>,
    ) -> Result<()> {
        if offset < self.header.index_start_offset {
            return self.visit_leaf_block(reader, offset, ctx, collector);
        }

        reader.seek(SeekFrom::Start(offset))?;
        let split_dim = reader.read_u32()? as usize;
        let split_value = reader.read_f64()?;
        let left_offset = reader.read_u64()?;
        let right_offset = reader.read_u64()?;

        // Logic check for split dimension
        let min = ctx.mins[split_dim];
        let max = ctx.maxs[split_dim];

        let go_left = min.is_none_or(|m| {
            if ctx.include_min {
                m <= split_value
            } else {
                m < split_value
            }
        });

        if go_left {
            self.visit_node(reader, left_offset, ctx, collector)?;
        }

        let go_right = max.is_none_or(|m| {
            if ctx.include_max {
                m >= split_value
            } else {
                m > split_value
            }
        });

        if go_right {
            self.visit_node(reader, right_offset, ctx, collector)?;
        }

        Ok(())
    }

    fn visit_leaf_block<R: StorageInput>(
        &self,
        reader: &mut StructReader<R>,
        offset: u64,
        ctx: &QueryContext,
        collector: &mut Vec<u64>,
    ) -> Result<()> {
        reader.seek(SeekFrom::Start(offset))?;
        let count = reader.read_u32()?;

        let mut points = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let mut vals = Vec::with_capacity(self.header.num_dims as usize);
            for _ in 0..self.header.num_dims {
                vals.push(reader.read_f64()?);
            }
            points.push(vals);
        }

        let mut doc_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            doc_ids.push(reader.read_u64()?);
        }

        for (vals, doc_id) in points.iter().zip(doc_ids.iter()) {
            let mut matches = true;
            for (i, &val) in vals.iter().enumerate() {
                let gte_min =
                    ctx.mins[i].is_none_or(|m| if ctx.include_min { val >= m } else { val > m });
                let lte_max =
                    ctx.maxs[i].is_none_or(|m| if ctx.include_max { val <= m } else { val < m });
                if !gte_min || !lte_max {
                    matches = false;
                    break;
                }
            }
            if matches {
                collector.push(*doc_id);
            }
        }
        Ok(())
    }
}

struct QueryContext<'a> {
    mins: &'a [Option<f64>],
    maxs: &'a [Option<f64>],
    include_min: bool,
    include_max: bool,
}

impl BKDTree for BKDReader {
    /// Perform a range search.
    fn range_search(
        &self,
        mins: &[Option<f64>],
        maxs: &[Option<f64>],
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

        if root_offset < self.header.index_start_offset && self.header.total_point_count > 0 {
            // Single leaf block case or root leaf
            let ctx = QueryContext {
                mins,
                maxs,
                include_min,
                include_max,
            };
            self.visit_leaf_block(&mut reader, root_offset, &ctx, &mut doc_ids)?;
        } else {
            let ctx = QueryContext {
                mins,
                maxs,
                include_min,
                include_max,
            };
            self.visit_node(&mut reader, root_offset, &ctx, &mut doc_ids)?;
        }

        doc_ids.sort_unstable();
        doc_ids.dedup();

        Ok(doc_ids)
    }
}

/// A simple BKD Tree for efficient range queries.
#[derive(Debug, Clone)]
pub struct SimpleBKDTree {
    /// Sorted array of (points, doc_id) pairs.
    entries: Vec<(Vec<f64>, u64)>,
    num_dims: u32,
    field_name: String,
}

impl BKDTree for SimpleBKDTree {
    fn range_search(
        &self,
        mins: &[Option<f64>],
        maxs: &[Option<f64>],
        include_min: bool,
        include_max: bool,
    ) -> Result<Vec<u64>> {
        let mut doc_ids = Vec::new();

        for (vals, doc_id) in &self.entries {
            let mut matches = true;
            for i in 0..self.num_dims as usize {
                let val = vals[i];
                let gte_min = mins[i].is_none_or(|m| if include_min { val >= m } else { val > m });
                let lte_max = maxs[i].is_none_or(|m| if include_max { val <= m } else { val < m });
                if !gte_min || !lte_max {
                    matches = false;
                    break;
                }
            }
            if matches {
                doc_ids.push(*doc_id);
            }
        }

        doc_ids.sort_unstable();
        doc_ids.dedup();
        Ok(doc_ids)
    }
}

impl SimpleBKDTree {
    /// Create a new BKD Tree from unsorted (points, doc_id) pairs.
    pub fn new(field_name: String, num_dims: u32, mut entries: Vec<(Vec<f64>, u64)>) -> Self {
        // Simple sorting by first dimension for consistent ordering
        entries.sort_by(|a, b| {
            if a.0.is_empty() || b.0.is_empty() {
                std::cmp::Ordering::Equal
            } else {
                a.0[0]
                    .partial_cmp(&b.0[0])
                    .unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        SimpleBKDTree {
            entries,
            num_dims,
            field_name,
        }
    }

    /// Create an empty BKD Tree.
    pub fn empty(field_name: String, num_dims: u32) -> Self {
        SimpleBKDTree {
            entries: Vec::new(),
            num_dims,
            field_name,
        }
    }

    /// Get the field name this tree is built for.
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Get the number of entries in this tree.
    pub fn size(&self) -> usize {
        self.entries.len()
    }

    /// Check if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Storage;
    use crate::storage::memory::{MemoryStorage, MemoryStorageConfig};
    use std::sync::Arc;

    fn create_test_tree() -> SimpleBKDTree {
        let entries = vec![
            (vec![1.0], 10), // doc 10, value 1.0
            (vec![3.0], 20), // doc 20, value 3.0
            (vec![2.0], 30), // doc 30, value 2.0
            (vec![5.0], 40), // doc 40, value 5.0
            (vec![4.0], 50), // doc 50, value 4.0
            (vec![1.5], 60), // doc 60, value 1.5
        ];
        SimpleBKDTree::new("test_field".to_string(), 1, entries)
    }

    #[test]
    fn test_bkd_writer_reader_2d() {
        let storage = Arc::new(MemoryStorage::new(MemoryStorageConfig::default()));
        let entries = vec![
            (vec![10.0, 20.0], 1),
            (vec![15.0, 25.0], 2),
            (vec![20.0, 30.0], 3),
        ];

        // Write
        {
            let output = storage.create_output("test_2d.bkd").unwrap();
            let mut writer = BKDWriter::new(output, 2);
            writer.write(&entries).unwrap();
            writer.finish().unwrap();
        }

        // Read
        {
            let reader = BKDReader::open(storage.clone(), "test_2d.bkd").unwrap();
            assert_eq!(reader.header.num_dims, 2);

            // Search [10, 10] to [15, 25]
            let results = reader
                .range_search(
                    &[Some(10.0), Some(10.0)],
                    &[Some(15.0), Some(25.0)],
                    true,
                    true,
                )
                .unwrap();
            assert_eq!(results, vec![1, 2]);
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
            (vec![1.0], 10),
            (vec![1.5], 60),
            (vec![2.0], 30),
            (vec![3.0], 20),
            (vec![4.0], 50),
            (vec![5.0], 40),
        ];
        assert_eq!(tree.entries, expected_order);
    }

    #[test]
    fn test_empty_tree() {
        let tree = SimpleBKDTree::empty("empty_field".to_string(), 1);

        assert_eq!(tree.size(), 0);
        assert!(tree.is_empty());
        assert_eq!(
            tree.range_search(&[Some(1.0)], &[Some(5.0)], true, true)
                .unwrap(),
            Vec::<u64>::new()
        );
    }

    #[test]
    fn test_range_search_exact_bounds() {
        let tree = create_test_tree();

        // Range [2.0, 4.0] should match docs 30, 20, 50
        let results = tree
            .range_search(&[Some(2.0)], &[Some(4.0)], true, true)
            .unwrap();
        let mut expected = vec![30, 20, 50]; // docs with values 2.0, 3.0, 4.0
        expected.sort();

        assert_eq!(results, expected);
    }
}
