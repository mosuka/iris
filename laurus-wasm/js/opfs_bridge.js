// OPFS (Origin Private File System) bridge for laurus-wasm.
//
// Provides async functions for reading/writing files in the browser's
// private file system. All functions operate on a named subdirectory
// under the OPFS root to isolate different indexes.
//
// These functions are imported by Rust via `#[wasm_bindgen(module = "/js/opfs_bridge.js")]`.

/**
 * Get or create a subdirectory handle under the OPFS root.
 * @param {string} name - Directory name for this index.
 * @returns {Promise<FileSystemDirectoryHandle>}
 */
async function getIndexDir(name) {
  const root = await navigator.storage.getDirectory();
  return root.getDirectoryHandle(name, { create: true });
}

/**
 * Initialize an OPFS directory for an index.
 * Returns an opaque handle (FileSystemDirectoryHandle) as a JS object.
 * @param {string} name - Index directory name.
 * @returns {Promise<FileSystemDirectoryHandle>}
 */
export async function opfs_init(name) {
  return getIndexDir(name);
}

/**
 * List all file names in the index directory.
 * @param {FileSystemDirectoryHandle} dir - Directory handle from opfs_init.
 * @returns {Promise<string[]>}
 */
export async function opfs_list_files(dir) {
  const names = [];
  for await (const [name, handle] of dir.entries()) {
    if (handle.kind === "file") {
      names.push(name);
    }
  }
  return names;
}

/**
 * Check if a file exists in the index directory.
 * @param {FileSystemDirectoryHandle} dir
 * @param {string} name - File name.
 * @returns {Promise<boolean>}
 */
export async function opfs_file_exists(dir, name) {
  try {
    await dir.getFileHandle(name);
    return true;
  } catch {
    return false;
  }
}

/**
 * Read the entire contents of a file as a Uint8Array.
 * @param {FileSystemDirectoryHandle} dir
 * @param {string} name - File name.
 * @returns {Promise<Uint8Array>}
 */
export async function opfs_read_file(dir, name) {
  const fileHandle = await dir.getFileHandle(name);
  const file = await fileHandle.getFile();
  const buffer = await file.arrayBuffer();
  return new Uint8Array(buffer);
}

/**
 * Write data to a file (creates or overwrites).
 * @param {FileSystemDirectoryHandle} dir
 * @param {string} name - File name.
 * @param {Uint8Array} data - File contents.
 * @returns {Promise<void>}
 */
export async function opfs_write_file(dir, name, data) {
  const fileHandle = await dir.getFileHandle(name, { create: true });
  const writable = await fileHandle.createWritable();
  await writable.write(data);
  await writable.close();
}

/**
 * Delete a file from the index directory.
 * @param {FileSystemDirectoryHandle} dir
 * @param {string} name - File name.
 * @returns {Promise<void>}
 */
export async function opfs_delete_file(dir, name) {
  try {
    await dir.removeEntry(name);
  } catch {
    // Ignore if file doesn't exist
  }
}

/**
 * Delete an entire index directory and all its contents.
 * @param {string} name - Index directory name.
 * @returns {Promise<void>}
 */
export async function opfs_delete_index(name) {
  const root = await navigator.storage.getDirectory();
  try {
    await root.removeEntry(name, { recursive: true });
  } catch {
    // Ignore if directory doesn't exist
  }
}
