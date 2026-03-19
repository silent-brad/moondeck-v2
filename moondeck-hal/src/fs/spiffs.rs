use anyhow::{Context, Result};
use esp_idf_sys as sys;
use std::ffi::CString;
use std::fs;
use std::path::Path;

pub struct FileSystem {
    mount_point: String,
}

impl FileSystem {
    pub fn mount(partition_label: &str, mount_point: &str) -> Result<Self> {
        let c_partition = CString::new(partition_label).context("Invalid partition label")?;
        let c_mount = CString::new(mount_point).context("Invalid mount point")?;

        let conf = sys::esp_vfs_spiffs_conf_t {
            base_path: c_mount.as_ptr(),
            partition_label: c_partition.as_ptr(),
            max_files: 5,
            format_if_mount_failed: true,
        };

        unsafe {
            sys::esp!(sys::esp_vfs_spiffs_register(&conf)).context("Failed to mount SPIFFS")?;
        }

        log::info!("SPIFFS mounted at {}", mount_point);

        Ok(Self {
            mount_point: mount_point.to_string(),
        })
    }

    pub fn mount_point(&self) -> &str {
        &self.mount_point
    }

    pub fn path(&self, relative: &str) -> String {
        format!("{}/{}", self.mount_point, relative.trim_start_matches('/'))
    }

    pub fn read_file(&self, relative_path: &str) -> Result<String> {
        let full_path = self.path(relative_path);
        fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read file: {}", full_path))
    }

    pub fn write_file(&self, relative_path: &str, content: &str) -> Result<()> {
        let full_path = self.path(relative_path);
        fs::write(&full_path, content)
            .with_context(|| format!("Failed to write file: {}", full_path))
    }

    pub fn exists(&self, relative_path: &str) -> bool {
        let full_path = self.path(relative_path);
        Path::new(&full_path).exists()
    }

    pub fn list_dir(&self, relative_path: &str) -> Result<Vec<String>> {
        let full_path = self.path(relative_path);
        let mut entries = Vec::new();

        for entry in fs::read_dir(&full_path)
            .with_context(|| format!("Failed to read directory: {}", full_path))?
        {
            if let Ok(entry) = entry {
                if let Some(name) = entry.file_name().to_str() {
                    entries.push(name.to_string());
                }
            }
        }

        Ok(entries)
    }

    pub fn delete_file(&self, relative_path: &str) -> Result<()> {
        let full_path = self.path(relative_path);
        fs::remove_file(&full_path).with_context(|| format!("Failed to delete file: {}", full_path))
    }

    pub fn create_dir(&self, relative_path: &str) -> Result<()> {
        let full_path = self.path(relative_path);
        fs::create_dir_all(&full_path)
            .with_context(|| format!("Failed to create directory: {}", full_path))
    }
}

impl Drop for FileSystem {
    fn drop(&mut self) {
        let c_partition = CString::new(self.mount_point.as_str()).ok();
        if let Some(partition) = c_partition {
            unsafe {
                let _ = sys::esp_vfs_spiffs_unregister(partition.as_ptr());
            }
        }
    }
}
