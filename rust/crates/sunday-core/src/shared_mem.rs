use std::fs::OpenOptions;
use std::path::PathBuf;
use memmap2::MmapMut;
use crate::error::SUNDAYError;
use parking_lot::RwLock;
use std::sync::Arc;

/// Shared Memory Segment for high-performance data exchange.
/// Caches the mmap handle to avoid expensive re-mapping.
pub struct SharedMemorySegment {
    path: PathBuf,
    name: String,
    mmap: RwLock<Option<Arc<MmapMut>>>,
}

impl SharedMemorySegment {
    pub fn new(name: &str) -> Self {
        let mut path = std::env::temp_dir();
        path.push(format!("sunday_shm_{}", name));
        Self {
            path,
            name: name.to_string(),
            mmap: RwLock::new(None),
        }
    }

    fn ensure_mapped(&self, size: usize) -> Result<Arc<MmapMut>, SUNDAYError> {
        {
            let read_guard = self.mmap.read();
            if let Some(mmap) = &*read_guard {
                if mmap.len() >= size {
                    return Ok(Arc::clone(mmap));
                }
            }
        }

        let mut write_guard = self.mmap.write();
        
        // Re-check after acquiring write lock
        if let Some(mmap) = &*write_guard {
            if mmap.len() >= size {
                return Ok(Arc::clone(mmap));
            }
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&self.path)?;

        file.set_len(size as u64)?;

        let mmap = unsafe { MmapMut::map_mut(&file) }
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to mmap SHM: {}", e)))?;

        let arc_mmap = Arc::new(mmap);
        *write_guard = Some(Arc::clone(&arc_mmap));
        Ok(arc_mmap)
    }

    /// Create and write data to shared memory.
    pub fn write(&self, data: &[u8]) -> Result<(), SUNDAYError> {
        let mmap = self.ensure_mapped(data.len())?;
        
        // MmapMut allows concurrent modification if we use raw pointers,
        // but here we are just copying data.
        let mmap_ptr = mmap.as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), mmap_ptr, data.len());
        }
        
        // Flush is expensive but ensures consistency for external processes
        mmap.flush().map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to flush SHM: {}", e)))?;

        Ok(())
    }

    /// Read data from shared memory. Returns a copy of the data.
    pub fn read(&self) -> Result<Vec<u8>, SUNDAYError> {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.path)?;

        let metadata = file.metadata()?;
        let size = metadata.len() as usize;
        
        if size == 0 {
            return Ok(Vec::new());
        }

        let mmap = self.ensure_mapped(size)?;
        Ok(mmap.to_vec())
    }

    /// Access the raw slice for zero-copy operations.
    pub fn as_slice(&self) -> Result<Vec<u8>, SUNDAYError> {
        self.read()
    }

    pub fn handle(&self) -> String {
        self.name.clone()
    }
}
