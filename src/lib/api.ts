import { invoke } from "@tauri-apps/api/core";
import type { 
  ProjectInfo, 
  AnalysisResult, 
  Schematic, 
  Issue, 
  AIAnalysis,
  DetailedIssue,
  ProviderStatus,
  DatasheetInfo,
  UserDatasheetInfo,
  Settings,
  ComponentInputDto,
  ClassificationResult,
  ClassifierConfig,
  RoleCategoryInfo,
  PcbSummary,
  PowerNetSpec,
  TraceWidthResult,
  CurrentCapacityIssue,
  CurrentCapacityReport,
  EmiReport,
  NetClassificationSummary,
  RuleViolation,
  RulesLoadResult,
  ComplianceAuditReport,
  // UCS types
  UnifiedCircuitSchema,
  CircuitStats,
  AiCircuitSummary,
  DecouplingAnalysisResponse,
  ConnectivityAnalysisResponse,
  SignalAnalysisResponse,
  ComponentInfo,
  NetInfo,
  ICRiskScore,
} from "../types";

// Check if running inside Tauri
export const isTauri = (): boolean => {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
};

// Safe invoke wrapper that checks for Tauri context
async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!isTauri()) {
    throw new Error(`Cannot call "${cmd}": Not running inside Tauri. Please run the app with "npm run tauri dev".`);
  }
  return invoke<T>(cmd, args);
}

export const api = {
  // ============================================================================
  // Project Management
  // ============================================================================
  
  async openProject(path: string): Promise<ProjectInfo> {
    return safeInvoke("open_project", { path });
  },

  async closeProject(): Promise<void> {
    return safeInvoke("close_project");
  },

  // Project watching
  async watchProject(projectPath: string): Promise<void> {
    return safeInvoke("watch_project", { projectPath });
  },

  async stopWatching(projectPath: string): Promise<void> {
    return safeInvoke("stop_watching", { projectPath });
  },

  // Schematic parsing
  async parseSchematic(schematicPath: string): Promise<Schematic> {
    return safeInvoke("parse_schematic", { schematicPath });
  },

  // Get current schematic from state (avoids re-parsing)
  async getCurrentSchematic(): Promise<Schematic> {
    return safeInvoke("get_current_schematic");
  },

  // ============================================================================
  // Analysis
  // ============================================================================

  // Design rule check
  async runDRC(schematicPath: string): Promise<Issue[]> {
    return safeInvoke("run_drc", { schematicPath });
  },

  // Analyze current project
  async analyzeDesign(): Promise<Issue[]> {
    return safeInvoke("analyze_design");
  },

  // Run datasheet-aware checks
  async runDatasheetCheck(): Promise<Issue[]> {
    return safeInvoke("run_datasheet_check");
  },

  // Run full analysis (DRC + Datasheet)
  async runFullAnalysis(): Promise<DetailedIssue[]> {
    return safeInvoke("run_full_analysis");
  },

  // Get detailed explanation for an issue
  async getIssueDetails(issue: Issue): Promise<DetailedIssue> {
    return safeInvoke("get_issue_details", { issue });
  },

  // Get all issue details
  async getAllIssueDetails(): Promise<DetailedIssue[]> {
    return safeInvoke("get_all_issue_details");
  },

  // ============================================================================
  // AI Integration
  // ============================================================================

  // AI analyze with router (uses best available provider: Claude or Ollama)
  async aiAnalyze(): Promise<AIAnalysis> {
    return safeInvoke("ai_analyze_with_router");
  },

  // Ask AI with router (uses best available provider: Claude or Ollama)
  async askAI(question: string): Promise<string> {
    return safeInvoke("ask_ai_with_router", { question });
  },

  // Configure Claude API key
  async configureClaude(apiKey: string): Promise<void> {
    return safeInvoke("configure_claude", { apiKey });
  },

  // Configure Ollama
  async configureOllama(url: string | null, model: string): Promise<boolean> {
    return safeInvoke("configure_ollama", { url, model });
  },

  // List Ollama models
  async listOllamaModels(url?: string): Promise<string[]> {
    return safeInvoke("list_ollama_models", { url: url || null });
  },

  // Set preferred AI provider
  async setAIProvider(provider: string): Promise<void> {
    return safeInvoke("set_ai_provider", { provider });
  },

  // Get AI provider status
  async getAIStatus(): Promise<ProviderStatus> {
    return safeInvoke("get_ai_status");
  },

  // ============================================================================
  // Settings & History
  // ============================================================================

  // Set API key (legacy)
  async setApiKey(key: string): Promise<void> {
    return safeInvoke("set_api_key", { key });
  },

  // Get settings
  async getSettings(): Promise<Settings> {
    return safeInvoke("get_settings");
  },

  // Update settings
  async updateSettings(settings: Settings): Promise<void> {
    return safeInvoke("update_settings", { settings });
  },

  // Project history
  async getProjectHistory(): Promise<ProjectInfo[]> {
    return safeInvoke("get_project_history");
  },

  // Analysis results
  async getAnalysisResults(projectPath: string): Promise<AnalysisResult[]> {
    return safeInvoke("get_analysis_results", { projectPath });
  },

  // ============================================================================
  // Datasheet Information
  // ============================================================================

  // Get list of supported ICs with datasheet info
  async getSupportedDatasheets(): Promise<DatasheetInfo[]> {
    return safeInvoke("get_supported_datasheets");
  },

  // Upload a user datasheet file
  async uploadDatasheet(filePath: string): Promise<string> {
    return safeInvoke("upload_datasheet", { filePath });
  },

  // Get list of user-uploaded datasheets
  async getUserDatasheets(): Promise<UserDatasheetInfo[]> {
    return safeInvoke("get_user_datasheets");
  },

  // Delete a user-uploaded datasheet
  async deleteDatasheet(filename: string): Promise<string> {
    return safeInvoke("delete_datasheet", { filename });
  },

  // ============================================================================
  // UCS (Unified Circuit Schema) API
  // ============================================================================

  /**
   * Get the current circuit as UCS JSON
   * This is the main entry point for accessing the circuit data in a CAD-agnostic format
   */
  async getCircuitUCS(): Promise<UnifiedCircuitSchema> {
    return safeInvoke("get_circuit_ucs");
  },

  /**
   * Get circuit statistics (component count, net count, etc.)
   */
  async getCircuitStats(): Promise<CircuitStats> {
    return safeInvoke("get_circuit_stats");
  },

  /**
   * Get AI-friendly circuit summary
   * This provides a condensed view optimized for AI analysis
   */
  async getCircuitAISummary(): Promise<AiCircuitSummary> {
    return safeInvoke("get_circuit_ai_summary");
  },

  /**
   * Get a filtered UCS slice for specific components
   * Useful for focused AI analysis on specific parts of the circuit
   * @param componentRefs - Array of component reference designators (e.g., ["U1", "C1", "R1"])
   */
  async getCircuitSlice(componentRefs: string[]): Promise<UnifiedCircuitSchema> {
    return safeInvoke("get_circuit_slice", { componentRefs });
  },

  /**
   * Analyze decoupling capacitors in the circuit
   * @param maxDistanceMm - Maximum distance to consider for nearby capacitors (default: 20mm)
   */
  async analyzeCircuitDecoupling(maxDistanceMm?: number): Promise<DecouplingAnalysisResponse> {
    return safeInvoke("analyze_circuit_decoupling", { maxDistanceMm: maxDistanceMm ?? null });
  },

  /**
   * Analyze circuit connectivity
   * Finds floating components, single-connection nets, and power/ground connections
   */
  async analyzeCircuitConnectivity(): Promise<ConnectivityAnalysisResponse> {
    return safeInvoke("analyze_circuit_connectivity");
  },

  /**
   * Analyze signal integrity concerns
   * Checks for I2C pull-ups, unterminated high-speed signals, etc.
   */
  async analyzeCircuitSignals(): Promise<SignalAnalysisResponse> {
    return safeInvoke("analyze_circuit_signals");
  },

  /**
   * Get all components connected to a specific net
   * @param netName - Name of the net to query
   */
  async getNetComponents(netName: string): Promise<ComponentInfo[]> {
    return safeInvoke("get_net_components", { netName });
  },

  /**
   * Get all nets connected to a specific component
   * @param refDes - Reference designator of the component (e.g., "U1")
   */
  async getComponentNets(refDes: string): Promise<NetInfo[]> {
    return safeInvoke("get_component_nets", { refDes });
  },

  /**
   * Parse a file directly to UCS format
   * Uses the adapter registry to automatically select the correct parser
   * @param filePath - Path to the schematic file
   */
  async parseFileToUCS(filePath: string): Promise<UnifiedCircuitSchema> {
    return safeInvoke("parse_file_to_ucs", { filePath });
  },

  /**
   * Get list of supported file formats
   * Returns file extensions that can be parsed (e.g., ["kicad_sch"])
   */
  async getSupportedFormats(): Promise<string[]> {
    return safeInvoke("get_supported_formats");
  },

  // ============================================================================
  // Component Role Classification (Phi-3 via Ollama)
  // ============================================================================

  /**
   * Classify a single component's role
   * @param refDes - Reference designator (e.g., "U1")
   * @param partNumber - Part number (e.g., "STM32F411CEU6")
   * @param libId - Optional library ID
   */
  async classifyComponentRole(refDes: string, partNumber: string, libId?: string): Promise<ClassificationResult> {
    return safeInvoke("classify_component_role", { refDes, partNumber, libId: libId || null });
  },

  /**
   * Classify multiple components in batch
   * @param components - Array of component inputs
   */
  async classifyComponentsBatch(components: ComponentInputDto[]): Promise<ClassificationResult[]> {
    return safeInvoke("classify_components_batch", { components });
  },

  /**
   * Classify all components in the current schematic
   */
  async classifySchematicComponents(): Promise<ClassificationResult[]> {
    return safeInvoke("classify_schematic_components");
  },

  /**
   * Configure the classifier with custom Ollama URL and model
   * @param url - Optional Ollama URL (default: http://localhost:11434)
   * @param model - Optional model name (default: phi3)
   */
  async configureClassifier(url?: string, model?: string): Promise<ClassifierConfig> {
    return safeInvoke("configure_classifier", { url: url || null, model: model || null });
  },

  /**
   * Check if the classifier (Phi-3 via Ollama) is available
   */
  async checkClassifierAvailable(): Promise<boolean> {
    return safeInvoke("check_classifier_available");
  },

  /**
   * Get all available component role categories
   */
  async getComponentRoleCategories(): Promise<RoleCategoryInfo[]> {
    return safeInvoke("get_component_role_categories");
  },

  // ============================================================================
  // PCB Compliance Analysis (IPC-2221, EMI, Custom Rules)
  // ============================================================================

  /**
   * Open and parse a PCB file (.kicad_pcb)
   * @param path - Path to .kicad_pcb file
   */
  async openPCB(path: string): Promise<PcbSummary> {
    return safeInvoke("open_pcb", { path });
  },

  /**
   * Run IPC-2221 current capacity analysis on all traces
   * @param tempRiseC - Temperature rise in Celsius (default: 10.0)
   */
  async analyzeIPC2221(tempRiseC?: number): Promise<CurrentCapacityReport> {
    return safeInvoke("analyze_ipc2221", { tempRiseC: tempRiseC ?? null });
  },

  /**
   * Check power traces against expected currents
   * @param powerNets - Array of power net specifications
   * @param tempRiseC - Temperature rise in Celsius (default: 10.0)
   */
  async checkPowerTraceCapacity(powerNets: PowerNetSpec[], tempRiseC?: number): Promise<CurrentCapacityIssue[]> {
    return safeInvoke("check_power_trace_capacity", { powerNets, tempRiseC: tempRiseC ?? null });
  },

  /**
   * Calculate required trace width for a given current
   * @param currentA - Current in Amperes
   * @param copperOz - Copper weight in oz (default: 1.0)
   * @param tempRiseC - Temperature rise in Celsius (default: 10.0)
   * @param isExternal - External layer (default: true)
   */
  async calculateTraceWidth(currentA: number, copperOz?: number, tempRiseC?: number, isExternal?: boolean): Promise<TraceWidthResult> {
    return safeInvoke("calculate_trace_width", { 
      currentA, 
      copperOz: copperOz ?? null, 
      tempRiseC: tempRiseC ?? null, 
      isExternal: isExternal ?? null 
    });
  },

  /**
   * Run EMI analysis on the PCB
   */
  async analyzeEMI(): Promise<EmiReport> {
    return safeInvoke("analyze_emi");
  },

  /**
   * Classify all nets in the PCB
   */
  async classifyPCBNets(): Promise<NetClassificationSummary> {
    return safeInvoke("classify_pcb_nets");
  },

  /**
   * Load custom compliance rules from JSON
   * @param rulesJson - JSON string containing rule definitions
   */
  async loadCustomRules(rulesJson: string): Promise<RulesLoadResult> {
    return safeInvoke("load_custom_rules", { rulesJson });
  },

  /**
   * Run custom rules check on the PCB
   */
  async checkCustomRules(): Promise<RuleViolation[]> {
    return safeInvoke("check_custom_rules");
  },

  /**
   * Get sample rules.json template
   */
  async getSampleRules(): Promise<string> {
    return safeInvoke("get_sample_rules");
  },

  /**
   * Run full PCB compliance audit (IPC-2221 + EMI + Custom Rules)
   * @param tempRiseC - Temperature rise in Celsius (default: 10.0)
   */
  async runPCBComplianceAudit(tempRiseC?: number): Promise<ComplianceAuditReport> {
    return safeInvoke("run_pcb_compliance_audit", { tempRiseC: tempRiseC ?? null });
  },

  // ============================================================================
  // DRS (Decoupling Risk Scoring) Analysis
  // ============================================================================

  /**
   * Run DRS analysis on currently loaded schematic and PCB
   * Requires both files to be loaded in the app state
   */
  async runDRSAnalysis(): Promise<ICRiskScore[]> {
    return safeInvoke("run_drs_analysis");
  },

  /**
   * Run DRS analysis with explicit file paths
   * @param schematicPath - Path to .kicad_sch file
   * @param pcbPath - Path to .kicad_pcb file
   */
  async runDRSAnalysisFromFiles(schematicPath: string, pcbPath: string): Promise<ICRiskScore[]> {
    return safeInvoke("run_drs_analysis_from_files", { schematicPath, pcbPath });
  },
};

// Helper to generate unique IDs
export function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}
