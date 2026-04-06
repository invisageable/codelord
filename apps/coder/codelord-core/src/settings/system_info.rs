//! System information for diagnostics.

use sysinfo::System;

/// Bytes per GiB.
const BYTES_PER_GIB: f64 = 1_073_741_824.0;

/// System information.
#[derive(Debug, Clone)]
pub struct SystemInfo {
  pub os_name: String,
  pub os_version: String,
  pub kernel_version: String,
  pub cpu_model: String,
  pub cpu_cores: usize,
  pub total_memory_gb: f64,
  pub available_memory_gb: f64,
  pub app_version: String,
  pub process_id: u32,
}

impl SystemInfo {
  /// Creates a new [`SystemInfo`] instance.
  pub fn new() -> Self {
    let sys = System::new_all();

    let os_name = System::name().unwrap_or_default();
    let os_version = System::os_version().unwrap_or_default();
    let kernel_version = System::kernel_version().unwrap_or_default();

    let cpu_model = sys
      .cpus()
      .first()
      .map(|cpu| cpu.brand().to_string())
      .unwrap_or_else(|| "Unknown".to_string());

    let cpu_cores =
      System::physical_core_count().unwrap_or_else(|| sys.cpus().len());

    let total_memory_gb = sys.total_memory() as f64 / BYTES_PER_GIB;
    let available_memory_gb = sys.available_memory() as f64 / BYTES_PER_GIB;

    let app_version = env!("CARGO_PKG_VERSION").to_string();
    let process_id = std::process::id();

    Self {
      os_name,
      os_version,
      kernel_version,
      cpu_model,
      cpu_cores,
      total_memory_gb,
      available_memory_gb,
      app_version,
      process_id,
    }
  }

  /// OS display string.
  pub fn os_display(&self) -> String {
    format!("{} {}", self.os_name, self.os_version)
  }

  /// CPU display string.
  pub fn cpu_display(&self) -> String {
    format!("{} ({} cores)", self.cpu_model, self.cpu_cores)
  }

  /// Total memory display string.
  pub fn total_memory_display(&self) -> String {
    format!("{:.1} GB", self.total_memory_gb)
  }

  /// Available memory display string.
  pub fn available_memory_display(&self) -> String {
    format!("{:.1} GB", self.available_memory_gb)
  }

  /// Format for clipboard.
  pub fn format(&self) -> String {
    format!(
      "[SYSTEM iNFORMATiON]\n\
       ├── OS: {} {}\n\
       ├── Kernel: {}\n\
       ├── CPU: {} ({} cores)\n\
       ├── Total RAM: {:.1} GB\n\
       └── Available RAM: {:.1} GB\n\
       \n\
       [APPLiCATiON]\n\
       ├── Version: {}\n\
       └── PID: {}",
      self.os_name,
      self.os_version,
      self.kernel_version,
      self.cpu_model,
      self.cpu_cores,
      self.total_memory_gb,
      self.available_memory_gb,
      self.app_version,
      self.process_id,
    )
  }
}

impl Default for SystemInfo {
  fn default() -> Self {
    Self::new()
  }
}
