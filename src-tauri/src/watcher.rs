use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, DebouncedEvent, Debouncer};
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// KiCAD file extensions to monitor
/// Supports both legacy (KiCad 4-5) and modern (KiCad 6+) formats
const KICAD_EXTENSIONS: &[&str] = &["kicad_sch", "kicad_pcb", "kicad_pro", "sch", "brd"];

/// Events emitted by the file watcher
#[derive(Debug, Clone)]
pub enum WatchEvent {
    FileCreated(PathBuf),
    FileModified(PathBuf),
    FileDeleted(PathBuf),
    FileRenamed { from: PathBuf, to: PathBuf },
}

/// Watcher for KiCAD project directories
pub struct ProjectWatcher {
    debouncer: Option<std::sync::Arc<std::sync::Mutex<Debouncer<RecommendedWatcher>>>>,
    event_tx: broadcast::Sender<WatchEvent>,
    watched_path: Option<PathBuf>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl ProjectWatcher {
    /// Create a new ProjectWatcher instance
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            debouncer: None,
            event_tx: tx,
            watched_path: None,
            handle: None,
        }
    }

    /// Start watching a directory recursively
    pub async fn watch(&mut self, path: PathBuf) -> Result<()> {
        // Stop any existing watcher
        self.unwatch().await?;

        let path = path.canonicalize()
            .with_context(|| format!("Failed to canonicalize path: {:?}", path))?;

        // Handle both file and directory paths
        let watch_path = if path.is_file() {
            path.parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| path.clone())
        } else {
            path.clone()
        };

        if !watch_path.exists() {
            anyhow::bail!("Path does not exist: {:?}", watch_path);
        }

        info!("Starting to watch: {:?}", watch_path);

        // Create a new broadcast channel for this watch session
        let (tx, _) = broadcast::channel(100);
        self.event_tx = tx;
        let event_tx_clone = self.event_tx.clone();
        self.watched_path = Some(watch_path.clone());

        // Create debouncer with 2 second delay to prevent excessive analysis triggers
        // This gives editors time to finish saving and prevents multiple rapid re-analyses
        let (debounce_tx, debounce_rx) = std::sync::mpsc::channel();
        
        let mut debouncer = new_debouncer(
            std::time::Duration::from_secs(2),
            debounce_tx,
        )
        .context("Failed to create file watcher debouncer")?;

        // Start watching
        debouncer
            .watcher()
            .watch(&watch_path, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to start watching: {:?}", watch_path))?;

        // Wrap debouncer in Arc<Mutex> to share across tasks
        let debouncer_arc = std::sync::Arc::new(std::sync::Mutex::new(debouncer));
        self.debouncer = Some(debouncer_arc.clone());

        // Spawn a task to process events
        let handle = tokio::spawn(async move {
            let _debouncer = debouncer_arc;
            
            loop {
                // Check for events with a timeout
                match debounce_rx.recv_timeout(std::time::Duration::from_millis(100)) {
                    Ok(Ok(events)) => {
                        for debounced_event in events {
                            if let Some(watch_events) = Self::process_debounced_event(&debounced_event) {
                                for watch_event in watch_events {
                                    if let Err(e) = event_tx_clone.send(watch_event.clone()) {
                                        warn!("Failed to send watch event: {}", e);
                                    } else {
                                        info!("Emitted watch event: {:?}", watch_event);
                                    }
                                }
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        error!("Watcher error: {:?}", e);
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                        // No events, continue
                    }
                    Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                        info!("Watcher channel disconnected");
                        break;
                    }
                }
                
                // Yield to allow other tasks to run
                tokio::task::yield_now().await;
            }
        });

        self.handle = Some(handle);

        info!("Successfully started watching: {:?}", watch_path);
        Ok(())
    }

    /// Stop watching
    pub async fn unwatch(&mut self) -> Result<()> {
        // Cancel the spawned task
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        // Drop the debouncer
        if let Some(debouncer) = self.debouncer.take() {
            if let Ok(mut d) = debouncer.lock() {
                if let Some(path) = &self.watched_path {
                    let _ = d.watcher().unwatch(path);
                }
            }
        }

        self.watched_path = None;
        
        info!("Stopped watching");
        Ok(())
    }

    /// Subscribe to watch events
    pub fn subscribe(&self) -> broadcast::Receiver<WatchEvent> {
        self.event_tx.subscribe()
    }

    /// Get the currently watched path
    pub fn watched_path(&self) -> Option<&PathBuf> {
        self.watched_path.as_ref()
    }

    /// Check if the watcher is active
    pub fn is_watching(&self) -> bool {
        self.debouncer.is_some() && self.watched_path.is_some()
    }

    /// Check if a path is a KiCAD file
    fn is_kicad_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| KICAD_EXTENSIONS.contains(&ext))
            .unwrap_or(false)
    }

    /// Process a debounced event and convert to WatchEvent
    fn process_debounced_event(event: &DebouncedEvent) -> Option<Vec<WatchEvent>> {
        let path = &event.path;
        
        // Only process KiCAD files
        if !Self::is_kicad_file(path) {
            return None;
        }

        // DebouncedEvent only has path and kind (Any)
        // We treat all events as modifications for simplicity
        Some(vec![WatchEvent::FileModified(path.clone())])
    }
}

impl Default for ProjectWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProjectWatcher {
    fn drop(&mut self) {
        // Best effort cleanup - can't call async unwatch in Drop
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }
        if let Some(debouncer) = self.debouncer.take() {
            if let Ok(mut d) = debouncer.lock() {
                if let Some(path) = &self.watched_path {
                    let _ = d.watcher().unwatch(path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_watcher_creation() {
        let watcher = ProjectWatcher::new();
        assert!(!watcher.is_watching());
        assert!(watcher.watched_path().is_none());
    }

    #[tokio::test]
    async fn test_watch_directory() {
        let temp_dir = TempDir::new().unwrap();
        let mut watcher = ProjectWatcher::new();

        let result = watcher.watch(temp_dir.path().to_path_buf()).await;
        assert!(result.is_ok());
        assert!(watcher.is_watching());

        watcher.unwatch().await.unwrap();
        assert!(!watcher.is_watching());
    }

    #[test]
    fn test_is_kicad_file() {
        assert!(ProjectWatcher::is_kicad_file(Path::new("test.kicad_sch")));
        assert!(ProjectWatcher::is_kicad_file(Path::new("test.kicad_pcb")));
        assert!(ProjectWatcher::is_kicad_file(Path::new("test.kicad_pro")));
        assert!(ProjectWatcher::is_kicad_file(Path::new("test.sch"))); // Legacy format
        assert!(ProjectWatcher::is_kicad_file(Path::new("test.brd"))); // Legacy format
        assert!(!ProjectWatcher::is_kicad_file(Path::new("test.txt")));
        assert!(!ProjectWatcher::is_kicad_file(Path::new("test")));
    }
}
