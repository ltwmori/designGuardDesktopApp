use tauri::Manager;
use tracing_subscriber;

mod commands;
mod state;
mod db;
pub mod watcher;

// Re-export validation types from library for commands
pub use designguard::{Issue, Severity};

use state::AppState;

pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Initialize database
            let app_data_dir = app.path().app_data_dir().unwrap();
            std::fs::create_dir_all(&app_data_dir).unwrap();
            
            // Create datasheets directory for user-uploaded datasheets
            let datasheets_dir = app_data_dir.join("datasheets");
            std::fs::create_dir_all(&datasheets_dir).unwrap();
            
            let db_path = app_data_dir.join("kicad_ai.db");
            let db = db::Database::new(&db_path)?;
            
            // Initialize app state
            let app_state = AppState::new(db);
            app.manage(app_state);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Core project commands
            commands::open_project,
            commands::close_project,
            commands::analyze_design,
            commands::set_api_key,
            commands::get_settings,
            commands::update_settings,
            // File watching
            commands::watch_project,
            commands::stop_watching,
            // Parsing and DRC
            commands::parse_schematic,
            commands::get_current_schematic,
            commands::run_drc,
            // History
            commands::get_project_history,
            commands::get_analysis_results,
            // NEW: Datasheet-aware checking
            commands::run_datasheet_check,
            commands::run_full_analysis,
            commands::get_issue_details,
            commands::get_all_issue_details,
            commands::get_supported_datasheets,
            commands::upload_datasheet,
            commands::get_user_datasheets,
            commands::delete_datasheet,
            // NEW: Ollama integration
            commands::configure_ollama,
            commands::list_ollama_models,
            commands::set_ai_provider,
            commands::get_ai_status,
            commands::ai_analyze_with_router,
            commands::ask_ai_with_router,
            commands::configure_claude,
            // NEW: UCS (Unified Circuit Schema) commands
            commands::get_circuit_ucs,
            commands::get_circuit_stats,
            commands::get_circuit_ai_summary,
            commands::get_circuit_slice,
            commands::analyze_circuit_decoupling,
            commands::analyze_circuit_connectivity,
            commands::analyze_circuit_signals,
            commands::get_net_components,
            commands::get_component_nets,
            commands::parse_file_to_ucs,
            commands::get_supported_formats,
            // NEW: Component Role Classification (Phi-3 via Ollama)
            commands::classify_component_role,
            commands::classify_components_batch,
            commands::classify_schematic_components,
            commands::configure_classifier,
            commands::check_classifier_available,
            commands::get_component_role_categories,
            // NEW: PCB Compliance (IPC-2221, EMI, Custom Rules)
            commands::open_pcb,
            commands::analyze_ipc2221,
            commands::check_power_trace_capacity,
            commands::calculate_trace_width,
            commands::analyze_emi,
            commands::classify_pcb_nets,
            commands::load_custom_rules,
            commands::check_custom_rules,
            commands::get_sample_rules,
            commands::run_pcb_compliance_audit,
            // NEW: DRS (Decoupling Risk Scoring)
            commands::run_drs_analysis,
            commands::run_drs_analysis_from_files,
            commands::trace_capacitor_to_ic_path,
            commands::find_all_capacitor_ic_paths,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
