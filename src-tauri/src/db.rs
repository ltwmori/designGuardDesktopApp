#![allow(dead_code)]

use rusqlite::{Connection, Result as SqlResult, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Mutex;
use crate::commands::{ProjectInfo, AnalysisResult};
use crate::analyzer::rules::Issue;
use crate::ai::claude::AIAnalysis;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Lock error: {0}")]
    Lock(String),
}

pub type Result<T> = std::result::Result<T, DatabaseError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentRecord {
    pub id: Option<i64>,
    pub mpn: String,
    pub manufacturer: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub package: Option<String>,
    pub datasheet_url: Option<String>,
    pub params: Vec<ComponentParam>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentParam {
    pub name: String,
    pub value: Option<f64>,
    pub unit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedAnalysis {
    pub id: i64,
    pub project_path: String,
    pub schematic_hash: String,
    pub issues: Vec<Issue>,
    pub ai_analysis: Option<AIAnalysis>,
    pub analyzed_at: String,
}

pub struct Database {
    conn: Mutex<Connection>,
}

// Implement Send + Sync for Database since we're using Mutex
unsafe impl Send for Database {}
unsafe impl Sync for Database {}

impl Database {
    pub fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn: Mutex::new(conn) };
        db.initialize()?;
        Ok(db)
    }

    pub fn open(path: &Path) -> Result<Self> {
        Self::new(path)
    }

    fn get_conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap()
    }

    pub fn initialize(&self) -> Result<()> {
        let conn = self.get_conn();
        // Legacy tables (keep for backward compatibility)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                last_analyzed TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS analysis_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                issues TEXT,
                suggestions TEXT,
                ai_analysis TEXT
            )",
            [],
        )?;

        // New component storage tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS components (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mpn TEXT NOT NULL UNIQUE,
                manufacturer TEXT,
                description TEXT,
                category TEXT,
                package TEXT,
                datasheet_url TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS component_params (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                component_id INTEGER NOT NULL REFERENCES components(id) ON DELETE CASCADE,
                param_name TEXT NOT NULL,
                param_value REAL,
                param_unit TEXT,
                UNIQUE(component_id, param_name)
            )",
            [],
        )?;

        // Create index for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_component_params_component_id 
             ON component_params(component_id)",
            [],
        )?;

        // New analysis history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS analysis_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_path TEXT NOT NULL,
                schematic_hash TEXT NOT NULL,
                issues_json TEXT NOT NULL,
                ai_analysis_json TEXT,
                analyzed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(project_path, schematic_hash)
            )",
            [],
        )?;

        // Create index for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_analysis_history_project 
             ON analysis_history(project_path)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_analysis_history_hash 
             ON analysis_history(schematic_hash)",
            [],
        )?;

        // Settings table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Items table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                description TEXT,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        Ok(())
    }

    // Component methods

    pub fn upsert_component(&self, component: &ComponentRecord) -> Result<i64> {
        let mut conn = self.get_conn();
        let tx = conn.transaction()?;

        // Check if component exists
        let existing_id: Option<i64> = tx.query_row(
            "SELECT id FROM components WHERE mpn = ?",
            params![component.mpn],
            |row| row.get(0),
        ).optional()?;

        let component_id = if let Some(id) = existing_id {
            // Update existing component
            tx.execute(
                "UPDATE components 
                 SET manufacturer = ?2, description = ?3, category = ?4, 
                     package = ?5, datasheet_url = ?6, updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![
                    id,
                    component.manufacturer,
                    component.description,
                    component.category,
                    component.package,
                    component.datasheet_url,
                ],
            )?;

            // Delete old params
            tx.execute(
                "DELETE FROM component_params WHERE component_id = ?",
                params![id],
            )?;

            id
        } else {
            // Insert new component
            tx.execute(
                "INSERT INTO components (mpn, manufacturer, description, category, package, datasheet_url)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    component.mpn,
                    component.manufacturer,
                    component.description,
                    component.category,
                    component.package,
                    component.datasheet_url,
                ],
            )?;
            tx.last_insert_rowid()
        };

        // Insert parameters
        for param in &component.params {
            tx.execute(
                "INSERT INTO component_params (component_id, param_name, param_value, param_unit)
                 VALUES (?1, ?2, ?3, ?4)",
                params![component_id, param.name, param.value, param.unit],
            )?;
        }

        tx.commit()?;
        Ok(component_id)
    }

    pub fn find_component_by_mpn(&self, mpn: &str) -> Result<Option<ComponentRecord>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT id, mpn, manufacturer, description, category, package, datasheet_url
             FROM components
             WHERE mpn = ?"
        )?;

        let component_opt = stmt.query_row(params![mpn], |row| {
            Ok(ComponentRecord {
                id: Some(row.get(0)?),
                mpn: row.get(1)?,
                manufacturer: row.get(2)?,
                description: row.get(3)?,
                category: row.get(4)?,
                package: row.get(5)?,
                datasheet_url: row.get(6)?,
                params: Vec::new(), // Will be loaded separately
            })
        }).optional()?;

        if let Some(mut component) = component_opt {
            // Load parameters
            let component_id = component.id.unwrap();
            let mut param_stmt = conn.prepare(
                "SELECT param_name, param_value, param_unit
                 FROM component_params
                 WHERE component_id = ?
                 ORDER BY param_name"
            )?;

            let params = param_stmt.query_map(params![component_id], |row| {
                Ok(ComponentParam {
                    name: row.get(0)?,
                    value: row.get(1)?,
                    unit: row.get(2)?,
                })
            })?
            .collect::<SqlResult<Vec<_>>>()
            .map_err(|e| DatabaseError::Sqlite(e))?;

            component.params = params;
            Ok(Some(component))
        } else {
            Ok(None)
        }
    }

    // Analysis history methods

    pub fn save_analysis(
        &self,
        project: &str,
        hash: &str,
        issues: &[Issue],
        ai: Option<&AIAnalysis>,
    ) -> Result<i64> {
        let issues_json = serde_json::to_string(issues)
            .map_err(|e| DatabaseError::Serialization(format!("Failed to serialize issues: {}", e)))?;

        let ai_json = if let Some(ai_analysis) = ai {
            Some(serde_json::to_string(ai_analysis)
                .map_err(|e| DatabaseError::Serialization(format!("Failed to serialize AI analysis: {}", e)))?)
        } else {
            None
        };

        let conn = self.get_conn();
        
        // Use INSERT OR REPLACE to handle unique constraint
        conn.execute(
            "INSERT OR REPLACE INTO analysis_history 
             (project_path, schematic_hash, issues_json, ai_analysis_json, analyzed_at)
             VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP)",
            params![project, hash, issues_json, ai_json],
        )?;

        // Get the ID of the inserted/updated row
        let id: i64 = conn.query_row(
            "SELECT id FROM analysis_history WHERE project_path = ?1 AND schematic_hash = ?2",
            params![project, hash],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn get_cached_analysis(
        &self,
        project: &str,
        hash: &str,
    ) -> Result<Option<CachedAnalysis>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_path, schematic_hash, issues_json, ai_analysis_json, analyzed_at
             FROM analysis_history
             WHERE project_path = ?1 AND schematic_hash = ?2"
        )?;

        let result = stmt.query_row(params![project, hash], |row| {
            let issues_json: String = row.get(3)?;
            let ai_json: Option<String> = row.get(4)?;

            let issues: Vec<Issue> = serde_json::from_str(&issues_json)
                .map_err(|e| rusqlite::Error::InvalidColumnType(
                    3,
                    format!("Failed to deserialize issues: {}", e),
                    rusqlite::types::Type::Text,
                ))?;

            let ai_analysis = if let Some(json) = ai_json {
                Some(serde_json::from_str(&json)
                    .map_err(|e| rusqlite::Error::InvalidColumnType(
                        4,
                        format!("Failed to deserialize AI analysis: {}", e),
                        rusqlite::types::Type::Text,
                    ))?)
            } else {
                None
            };

            Ok(CachedAnalysis {
                id: row.get(0)?,
                project_path: row.get(1)?,
                schematic_hash: row.get(2)?,
                issues,
                ai_analysis,
                analyzed_at: row.get(5)?,
            })
        }).optional()?;

        Ok(result)
    }

    // Settings methods

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT value FROM settings WHERE key = ?"
        )?;

        let result = stmt.query_row(params![key], |row| {
            row.get::<_, String>(0)
        }).optional()?;

        Ok(result)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.get_conn();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value, updated_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)",
            params![key, value],
        )?;
        Ok(())
    }

    // Item methods

    pub fn create_item(&self, name: &str, description: Option<&str>) -> Result<i64> {
        let conn = self.get_conn();
        conn.execute(
            "INSERT INTO items (name, description) VALUES (?1, ?2)",
            params![name, description],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_items(&self) -> Result<Vec<Item>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_at, updated_at FROM items ORDER BY created_at DESC"
        )?;
        
        let items = stmt.query_map([], |row| {
            Ok(Item {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                description: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()
        .map_err(|e| DatabaseError::Sqlite(e))?;

        Ok(items)
    }

    // Legacy methods (for backward compatibility)

    pub fn get_project_history(&self) -> SqlResult<Vec<ProjectInfo>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT path, name, last_analyzed FROM projects ORDER BY created_at DESC"
        )?;
        
        let projects = stmt.query_map([], |row| {
            Ok(ProjectInfo {
                path: row.get(0)?,
                name: row.get(1)?,
                last_analyzed: row.get(2)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

        Ok(projects)
    }

    pub fn get_analysis_results(&self, project_path: &str) -> SqlResult<Vec<AnalysisResult>> {
        let conn = self.get_conn();
        let mut stmt = conn.prepare(
            "SELECT project_path, timestamp, issues, suggestions, ai_analysis 
             FROM analysis_results 
             WHERE project_path = ? 
             ORDER BY timestamp DESC"
        )?;
        
        let results = stmt.query_map([project_path], |row| {
            Ok(AnalysisResult {
                project_path: row.get(0)?,
                timestamp: row.get(1)?,
                issues: serde_json::from_str(row.get::<_, String>(2)?.as_str()).unwrap_or_default(),
                suggestions: serde_json::from_str(row.get::<_, String>(3)?.as_str()).unwrap_or_default(),
                ai_analysis: row.get(4)?,
            })
        })?
        .collect::<SqlResult<Vec<_>>>()?;

        Ok(results)
    }
}

// Helper trait for optional query results
trait OptionalResult<T> {
    fn optional(self) -> SqlResult<Option<T>>;
}

impl<T> OptionalResult<T> for SqlResult<T> {
    fn optional(self) -> SqlResult<Option<T>> {
        match self {
            Ok(val) => Ok(Some(val)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(&db_path).unwrap();
        (db, temp_dir)
    }

    #[test]
    fn test_component_upsert() {
        let (db, _temp) = create_test_db();

        let component = ComponentRecord {
            id: None,
            mpn: "STM32F401RE".to_string(),
            manufacturer: Some("STMicroelectronics".to_string()),
            description: Some("ARM Cortex-M4 MCU".to_string()),
            category: Some("Microcontroller".to_string()),
            package: Some("LQFP64".to_string()),
            datasheet_url: Some("https://example.com/datasheet.pdf".to_string()),
            params: vec![
                ComponentParam {
                    name: "Operating Voltage".to_string(),
                    value: Some(3.3),
                    unit: Some("V".to_string()),
                },
                ComponentParam {
                    name: "Frequency".to_string(),
                    value: Some(84.0),
                    unit: Some("MHz".to_string()),
                },
            ],
        };

        let id = db.upsert_component(&component).unwrap();
        assert!(id > 0);

        // Test update
        let mut updated = component.clone();
        updated.description = Some("Updated description".to_string());
        let id2 = db.upsert_component(&updated).unwrap();
        assert_eq!(id, id2);
    }

    #[test]
    fn test_find_component_by_mpn() {
        let (db, _temp) = create_test_db();

        let component = ComponentRecord {
            id: None,
            mpn: "LM358".to_string(),
            manufacturer: Some("Texas Instruments".to_string()),
            description: Some("Dual Op-Amp".to_string()),
            category: Some("Amplifier".to_string()),
            package: None,
            datasheet_url: None,
            params: vec![],
        };

        db.upsert_component(&component).unwrap();

        let found = db.find_component_by_mpn("LM358").unwrap();
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.mpn, "LM358");
        assert_eq!(found.manufacturer, Some("Texas Instruments".to_string()));

        let not_found = db.find_component_by_mpn("NONEXISTENT").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_save_and_get_analysis() {
        let (db, _temp) = create_test_db();

        let issues = vec![
            Issue {
                risk_score: None,
                id: "1".to_string(),
                rule_id: "test_rule".to_string(),
                severity: crate::analyzer::rules::Severity::Warning,
                message: "Test issue".to_string(),
                component: None,
                location: None,
                suggestion: None,
            },
        ];

        let ai_analysis = AIAnalysis {
            summary: "Test summary".to_string(),
            circuit_description: "Test circuit".to_string(),
            potential_issues: vec![],
            improvement_suggestions: vec![],
            component_recommendations: vec![],
        };

        let id = db.save_analysis("test/project", "hash123", &issues, Some(&ai_analysis)).unwrap();
        assert!(id > 0);

        let cached = db.get_cached_analysis("test/project", "hash123").unwrap();
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.issues.len(), 1);
        assert!(cached.ai_analysis.is_some());
    }

    #[test]
    fn test_settings() {
        let (db, _temp) = create_test_db();

        db.set_setting("test_key", "test_value").unwrap();

        let value = db.get_setting("test_key").unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        let not_found = db.get_setting("nonexistent").unwrap();
        assert!(not_found.is_none());
    }
}
