// Severity levels for issues
export type Severity = 'Error' | 'Warning' | 'Info' | 'Suggestion';

// Position in schematic
export interface Position {
  x: number;
  y: number;
}

// Risk score for an issue
export interface RiskScore {
  value: number;  // 0-100 risk index
  inductance_nh?: number;
  limit_nh?: number;
  metric?: string;
  details?: string;
}

// Issue detected by analyzer
export interface Issue {
  id: string;
  rule_id: string;
  severity: Severity;
  message: string;
  component: string | null;
  location: Position | null;
  suggestion: string | null;
  risk_score?: RiskScore;
}

// Component in schematic
export interface Component {
  uuid: string;
  reference: string;
  value: string;
  lib_id: string;
  footprint: string | null;
  position: Position;
  rotation: number;
  properties: Record<string, string>;
  pins: Pin[];
}

export interface Pin {
  number: string;
  name: string;
  position: Position;
}

// Wire connection
export interface Wire {
  uuid: string;
  start: Position;
  end: Position;
}

// Label in schematic
export type LabelType = 'Local' | 'Global' | 'Hierarchical';

export interface Label {
  uuid: string;
  text: string;
  position: Position;
  rotation: number;
  label_type: LabelType;
}

// Net connection
export interface Net {
  name: string;
  nodes: NetNode[];
}

export interface NetNode {
  component_ref: string;
  pin: string;
}

// Full schematic
export interface Schematic {
  uuid: string;
  filename: string;
  version: string | null;
  components: Component[];
  wires: Wire[];
  labels: Label[];
  nets: Net[];
  power_symbols: Component[];
}

// Project info
export interface ProjectInfo {
  path: string;
  name: string;
  last_analyzed: string | null;
}

// Analysis result from database
export interface AnalysisResult {
  project_path: string;
  timestamp: string;
  issues: string[];
  suggestions: string[];
  ai_analysis: string | null;
}

// AI Analysis response
export interface AIAnalysis {
  summary: string;
  circuit_description: string;
  potential_issues: string[];
  improvement_suggestions: string[];
  component_recommendations: ComponentRecommendation[];
}

export interface ComponentRecommendation {
  component: string;
  current_value: string;
  suggested_value: string | null;
  reason: string;
}

// Chat message
export interface ChatMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

// Settings
export interface Settings {
  theme: 'light' | 'dark' | 'system';
  apiKeyConfigured: boolean;
  auto_analyze?: boolean;
  ai_provider?: string;
  ollama_url?: string;
  ollama_model?: string;
}

// ============================================================================
// NEW TYPES: Detailed Issues, AI Provider Status, Datasheet Info
// ============================================================================

// Detailed issue with full explanation
export interface DetailedIssue {
  id: string;
  severity: Severity;
  rule_id: string;
  title: string;
  summary: string;
  components: string[];
  location: Position | null;
  explanation: IssueExplanation;
  user_actions: UserActions;
  risk_score?: RiskScore;
}

export interface IssueExplanation {
  what: string;
  why: WhySection;
  technical_background: TechnicalBackground | null;
  how_to_fix: HowToFix;
  diagrams: Diagram[];
  references: Reference[];
}

export interface WhySection {
  summary: string;
  consequences: Consequence[];
  failure_examples: string[];
}

export interface Consequence {
  description: string;
  severity: 'annoying' | 'problematic' | 'serious' | 'critical';
  likelihood: 'rare' | 'occasional' | 'likely' | 'certain';
}

export interface TechnicalBackground {
  concept: string;
  detailed_explanation: string;
  equations: Equation[];
  related_concepts: string[];
}

export interface Equation {
  name: string;
  formula: string;
  variables: Variable[];
  example_calculation: string | null;
}

export interface Variable {
  symbol: string;
  name: string;
  unit: string;
  typical_value: string | null;
}

export interface HowToFix {
  steps: FixStep[];
  component_suggestions: ComponentSuggestion[];
  pitfalls: string[];
  verification: string;
}

export interface FixStep {
  step_number: number;
  instruction: string;
  details: string | null;
  image: string | null;
}

export interface ComponentSuggestion {
  component_type: string;
  value: string;
  footprint: string;
  notes: string;
  example_part_numbers: string[];
}

export interface Diagram {
  diagram_type: 'schematic_snippet' | 'waveform_before' | 'waveform_after' | 'placement_guide' | 'comparison';
  title: string;
  description: string;
  svg_content: string | null;
  image_path: string | null;
}

export interface Reference {
  title: string;
  url: string | null;
  reference_type: 'datasheet' | 'application_note' | 'tutorial' | 'wikipedia' | 'text_book';
}

export interface UserActions {
  auto_fix_available: boolean;
  can_dismiss: boolean;
  dismiss_options: DismissOption[];
}

export interface DismissOption {
  label: string;
  reason_code: string;
  requires_comment: boolean;
}

// AI Provider Status
export interface ProviderStatus {
  claude_available: boolean;
  claude_configured: boolean;
  ollama_available: boolean;
  ollama_models: string[];
  preferred: string;
  active_provider: string | null;
}

// Datasheet Info
export interface DatasheetInfo {
  part_numbers: string[];
  manufacturer: string;
  category: string;
  datasheet_url: string | null;
}

// User Datasheet Info (includes filename)
export interface UserDatasheetInfo {
  filename: string;
  part_numbers: string[];
  manufacturer: string;
  category: string;
  datasheet_url: string | null;
}

// ============================================================================
// UCS (Unified Circuit Schema) Types
// ============================================================================

// Source CAD tool
export type SourceCAD = 'kicad' | 'altium' | 'easyeda' | 'eagle' | 'netlist' | 'edif' | 'unknown';

// Electrical pin type
export type ElectricalType = 
  | 'input' 
  | 'output' 
  | 'bidirectional' 
  | 'tri_state' 
  | 'passive' 
  | 'power_in' 
  | 'power_out' 
  | 'open_collector' 
  | 'open_emitter' 
  | 'no_connect' 
  | 'unspecified';

// Signal type classification
export type SignalType = 
  | 'analog' 
  | 'digital' 
  | 'high_speed' 
  | 'power' 
  | 'ground' 
  | 'clock' 
  | 'reset' 
  | 'data' 
  | 'control' 
  | 'unknown';

// Circuit metadata
export interface CircuitMetadata {
  project_name: string;
  source_cad: SourceCAD;
  cad_version: string | null;
  timestamp: string;
  variant: string;
  source_file: string | null;
  schema_version: string;
}

// UCS Position
export interface UcsPosition {
  x: number;
  y: number;
}

// UCS Pin
export interface UcsPin {
  number: string;
  name: string | null;
  electrical_type: ElectricalType;
  connected_net: string | null;
  position: UcsPosition | null;
}

// Attribute value (can be various types)
export type AttributeValue = 
  | string 
  | number 
  | boolean 
  | AttributeValue[] 
  | { [key: string]: AttributeValue };

// UCS Component
export interface UcsComponent {
  ref_des: string;
  mpn: string | null;
  value: string | null;
  footprint: string | null;
  lib_id: string | null;
  is_virtual: boolean;
  pins: UcsPin[];
  position: UcsPosition | null;
  rotation: number;
  attributes: Record<string, AttributeValue>;
  uuid: string;
}

// Net connection
export interface NetConnection {
  ref_des: string;
  pin_number: string;
}

// UCS Net
export interface UcsNet {
  net_name: string;
  voltage_level: number | null;
  is_power_rail: boolean;
  signal_type: SignalType;
  connections: NetConnection[];
  attributes: Record<string, AttributeValue>;
}

// Complete Unified Circuit Schema
export interface UnifiedCircuitSchema {
  metadata: CircuitMetadata;
  components: UcsComponent[];
  nets: UcsNet[];
}

// Circuit statistics
export interface CircuitStats {
  component_count: number;
  net_count: number;
  connection_count: number;
  ic_count: number;
  power_net_count: number;
}

// IC Summary for AI
export interface IcSummary {
  ref_des: string;
  value: string | null;
  mpn: string | null;
  power_nets: string[];
  connected_net_count: number;
  has_decoupling: boolean;
}

// Power rail summary
export interface PowerRailSummary {
  name: string;
  voltage: number | null;
  connected_component_count: number;
}

// AI Circuit Summary
export interface AiCircuitSummary {
  project_name: string;
  source_cad: string;
  component_count: number;
  net_count: number;
  ic_count: number;
  ics: IcSummary[];
  power_rails: PowerRailSummary[];
  potential_issues: string[];
}

// Decoupling analysis response
export interface DecouplingAnalysisResponse {
  ics_with_decoupling: number;
  ics_missing_decoupling: number;
  missing_details: MissingDecouplingInfo[];
}

export interface MissingDecouplingInfo {
  ic_ref: string;
  ic_value: string | null;
  recommendation: string;
}

// Connectivity analysis response
export interface ConnectivityAnalysisResponse {
  floating_components: string[];
  single_connection_nets: string[];
  power_rail_count: number;
  ground_rail_count: number;
}

// Signal analysis response
export interface SignalAnalysisResponse {
  i2c_buses: I2cBusResponse[];
  unterminated_signal_count: number;
  unterminated_signals: string[];
}

export interface I2cBusResponse {
  sda_net: string | null;
  scl_net: string | null;
  has_pullups: boolean;
  device_count: number;
}

// Component info (simplified)
export interface ComponentInfo {
  ref_des: string;
  value: string | null;
  mpn: string | null;
  is_ic: boolean;
}

// Net info (simplified)
export interface NetInfo {
  name: string;
  voltage_level: number | null;
  is_power_rail: boolean;
  signal_type: string;
  connection_count: number;
}

// ============================================================================
// Component Role Classification Types
// ============================================================================

export interface ComponentInputDto {
  ref_des: string;
  part_number: string;
  lib_id?: string;
  footprint?: string;
}

export interface ClassificationResult {
  component: ComponentInput;
  role: string;  // ComponentRole enum as string
  confidence: number;  // 0.0 - 1.0
  alternatives?: Array<{ role: string; confidence: number }>;
  reasoning?: string;
}

export interface ComponentInput {
  ref_des: string;
  part_number: string;
  lib_id?: string;
  footprint?: string;
}

export interface ClassifierConfig {
  url: string;
  model: string;
  available: boolean;
}

export interface RoleCategoryInfo {
  category: string;
  roles: RoleInfo[];
}

export interface RoleInfo {
  name: string;
  description: string;
}

// ============================================================================
// PCB Compliance Types
// ============================================================================

export interface PcbSummary {
  filename: string;
  trace_count: number;
  via_count: number;
  zone_count: number;
  footprint_count: number;
  net_count: number;
  layer_count: number;
  board_thickness_mm: number;
}

export interface PowerNetSpec {
  net_name: string;
  expected_current_a: number;
}

export interface TraceWidthResult {
  required_width_mm: number;
  required_width_mils: number;
  current_a: number;
  copper_oz: number;
  temp_rise_c: number;
  is_external: boolean;
}

export interface CurrentCapacityIssue {
  trace_uuid: string;
  net_name: string;
  layer: string;
  current_width_mm: number;
  required_width_mm: number;
  max_current_a: number;
  expected_current_a: number;
  severity: 'Error' | 'Warning' | 'Info';
  message: string;
}

export interface TraceCurrentAnalysis {
  trace_uuid: string;
  net_name: string;
  layer: string;
  width_mm: number;
  length_mm: number;
  max_current_a: number;
  temp_rise_c: number;
}

export interface NetCurrentSummary {
  net_name: string;
  min_width_mm: number;
  max_width_mm: number;
  min_current_capacity_a: number;
  total_length_mm: number;
  trace_count: number;
}

export interface CurrentCapacityReport {
  temp_rise_c: number;
  outer_copper_oz: number;
  inner_copper_oz: number;
  trace_analyses: TraceCurrentAnalysis[];
  net_summaries: NetCurrentSummary[];
}

export type EmiSeverity = 'Critical' | 'High' | 'Medium' | 'Low' | 'Info';

export type EmiCategory = 
  | 'PlaneGapCrossing'
  | 'MissingReferencePlane'
  | 'ReturnPathDiscontinuity'
  | 'LayerTransition'
  | 'ParallelHighSpeed'
  | 'UnshieldedClock';

export interface Position3D {
  x: number;
  y: number;
  z?: number;
}

export interface EmiIssue {
  id: string;
  severity: EmiSeverity;
  category: EmiCategory;
  net_name: string;
  layer: string;
  location: Position3D | null;
  message: string;
  recommendation: string;
}

export interface EmiReport {
  total_issues: number;
  critical_count: number;
  high_count: number;
  medium_count: number;
  issues: EmiIssue[];
  recommendations: string[];
}

export interface NetClassificationSummary {
  total_nets: number;
  high_speed_count: number;
  clock_count: number;
  power_count: number;
  ground_count: number;
  analog_count: number;
  digital_count: number;
  high_speed_nets: string[];
  clock_nets: string[];
}

export type RuleSeverity = 'Error' | 'Warning' | 'Info';

export type RuleCategory = 
  | 'Manufacturing'
  | 'Signal'
  | 'Power'
  | 'Thermal'
  | 'Mechanical'
  | 'Safety'
  | 'Custom';

export interface RuleViolation {
  rule_id: string;
  rule_name: string;
  severity: RuleSeverity;
  category: RuleCategory;
  message: string;
  location: Position3D | null;
  affected_items: string[];
  suggestion: string | null;
}

export interface RulesLoadResult {
  name: string;
  version: string;
  rule_count: number;
  enabled_count: number;
}

export interface Ipc2221Summary {
  traces_analyzed: number;
  nets_analyzed: number;
  temp_rise_c: number;
  copper_oz: number;
}

export interface EmiSummary {
  total_issues: number;
  critical_count: number;
  high_count: number;
  recommendations: string[];
}

export interface NetSummary {
  total_nets: number;
  high_speed_count: number;
  clock_count: number;
  high_speed_nets: string[];
  clock_nets: string[];
}

export interface ComplianceAuditReport {
  pcb_filename: string;
  total_issues: number;
  critical_issues: number;
  ipc2221_summary: Ipc2221Summary;
  emi_summary: EmiSummary;
  net_summary: NetSummary;
  custom_rule_violations: number;
  emi_issues: EmiIssue[];
  custom_violations: RuleViolation[];
}

// ============================================================================
// DRS (Decoupling Risk Scoring) Types
// ============================================================================

export type NetCriticality = 'Critical' | 'High' | 'Medium' | 'Low';

export interface ICRiskScore {
  ic_reference: string;
  ic_value: string;
  risk_index: number;  // 0-100
  proximity_penalty: number;
  inductance_penalty: number;
  mismatch_penalty: number;
  net_criticality: NetCriticality;
  decoupling_capacitors: CapacitorAnalysis[];
  high_risk_heuristics: HighRiskHeuristic[];
  location: Position | null;
}

export interface CapacitorAnalysis {
  capacitor_reference: string;
  capacitor_value: string;
  distance_mm: number;
  proximity_penalty: number;
  via_count: number;
  dog_bone_length_mm: number;
  inductance_penalty: number;
  capacitor_srf_mhz: number;
  ic_switching_freq_mhz: number;
  mismatch_penalty: number;
  shared_via: boolean;
  backside_offset: boolean;
  neck_down: boolean;
}

export interface HighRiskHeuristic {
  SharedVia?: {
    via_uuid: string;
    capacitor1: string;
    capacitor2: string;
  };
  BacksideOffset?: {
    capacitor: string;
    ic: string;
    via_count: number;
  };
  NeckDown?: {
    capacitor: string;
    trace_width_mm: number;
    plane_connection: boolean;
  };
}
