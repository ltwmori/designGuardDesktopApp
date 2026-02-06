import { useState } from 'react';
import { Shield, AlertTriangle, Loader2, RefreshCw, FileText, Calculator, Upload, Download, X } from 'lucide-react';
import { api } from '../lib/api';
import { useStore } from '../lib/store';
import { isTauri } from '../lib/api';
import type { 
  ComplianceAuditReport, 
  CurrentCapacityReport, 
  EmiReport, 
  NetClassificationSummary,
  RuleViolation,
  TraceWidthResult,
} from '../types';

export function PCBCompliancePanel() {
  const { project } = useStore();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Audit report
  const [auditReport, setAuditReport] = useState<ComplianceAuditReport | null>(null);
  const [runningAudit, setRunningAudit] = useState(false);
  
  // Individual analyses
  const [ipcReport, setIpcReport] = useState<CurrentCapacityReport | null>(null);
  const [emiReport, setEmiReport] = useState<EmiReport | null>(null);
  const [netSummary, setNetSummary] = useState<NetClassificationSummary | null>(null);
  const [customViolations, setCustomViolations] = useState<RuleViolation[]>([]);
  
  // Custom rules
  const [rulesJson, setRulesJson] = useState<string>('');
  const [loadingRules, setLoadingRules] = useState(false);
  const [rulesLoaded, setRulesLoaded] = useState(false);
  
  // Filters
  const [selectedSeverity, setSelectedSeverity] = useState<string | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [activeSection, setActiveSection] = useState<'overview' | 'ipc2221' | 'emi' | 'nets' | 'rules' | 'calculator'>('overview');

  const runFullAudit = async () => {
    if (!project) {
      setError('No project loaded');
      return;
    }

    setRunningAudit(true);
    setError(null);

    try {
      const report = await api.runPCBComplianceAudit(10.0);
      setAuditReport(report);
      setIpcReport({
        temp_rise_c: report.ipc2221_summary.temp_rise_c,
        outer_copper_oz: report.ipc2221_summary.copper_oz,
        inner_copper_oz: report.ipc2221_summary.copper_oz,
        trace_analyses: [],
        net_summaries: [],
      });
      setEmiReport(report.emi_summary as any);
      setNetSummary(report.net_summary as any);
      setCustomViolations(report.custom_violations);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to run compliance audit');
    } finally {
      setRunningAudit(false);
    }
  };

  const runIPC2221 = async () => {
    setLoading(true);
    setError(null);
    try {
      const report = await api.analyzeIPC2221(10.0);
      setIpcReport(report);
      setActiveSection('ipc2221');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to run IPC-2221 analysis');
    } finally {
      setLoading(false);
    }
  };

  const runEMI = async () => {
    setLoading(true);
    setError(null);
    try {
      const report = await api.analyzeEMI();
      setEmiReport(report);
      setActiveSection('emi');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to run EMI analysis');
    } finally {
      setLoading(false);
    }
  };

  const runNetClassification = async () => {
    setLoading(true);
    setError(null);
    try {
      const summary = await api.classifyPCBNets();
      setNetSummary(summary);
      setActiveSection('nets');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to classify nets');
    } finally {
      setLoading(false);
    }
  };

  const loadSampleRules = async () => {
    setLoadingRules(true);
    try {
      const sample = await api.getSampleRules();
      setRulesJson(sample);
    } catch (e) {
      setError(`Failed to load sample rules: ${e}`);
    } finally {
      setLoadingRules(false);
    }
  };

  const loadCustomRules = async () => {
    if (!rulesJson.trim()) {
      setError('Please enter or load rules JSON');
      return;
    }

    setLoadingRules(true);
    setError(null);
    try {
      await api.loadCustomRules(rulesJson);
      setRulesLoaded(true);
      setError(null);
      // Run check after loading
      try {
        const violations = await api.checkCustomRules();
        setCustomViolations(violations);
      } catch (e) {
        console.error('Failed to check rules:', e);
      }
    } catch (e) {
      setError(`Failed to load rules: ${e}`);
    } finally {
      setLoadingRules(false);
    }
  };

  const handleImportRules = async () => {
    if (!isTauri()) {
      setError('File import is only available in the Tauri app');
      return;
    }

    try {
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }],
        title: 'Select Rules JSON File',
      });

      if (selected && !Array.isArray(selected)) {
        const { readTextFile } = await import('@tauri-apps/plugin-fs');
        const content = await readTextFile(selected);
        setRulesJson(content);
      }
    } catch (e) {
      setError(`Failed to import rules: ${e}`);
    }
  };

  const handleExportRules = async () => {
    if (!isTauri() || !rulesJson) return;

    try {
      const { save } = await import('@tauri-apps/plugin-dialog');
      const path = await save({
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }],
        title: 'Save Rules JSON File',
        defaultPath: 'custom_rules.json',
      });

      if (path) {
        const { writeTextFile } = await import('@tauri-apps/plugin-fs');
        await writeTextFile(path, rulesJson);
      }
    } catch (e) {
      setError(`Failed to export rules: ${e}`);
    }
  };

  // Filter violations
  const filteredViolations = customViolations.filter(v => {
    if (selectedSeverity && v.severity !== selectedSeverity) return false;
    if (selectedCategory && v.category !== selectedCategory) return false;
    return true;
  });

  const filteredEmiIssues = (emiReport?.issues || []).filter(issue => {
    if (selectedSeverity && issue.severity !== selectedSeverity) return false;
    return true;
  });

  const getSeverityColor = (severity: string) => {
    switch (severity) {
      case 'Critical':
      case 'Error':
        return 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-900/20';
      case 'High':
      case 'Warning':
        return 'text-yellow-600 dark:text-yellow-400 bg-yellow-50 dark:bg-yellow-900/20';
      case 'Medium':
      case 'Info':
        return 'text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-900/20';
      default:
        return 'text-gray-600 dark:text-gray-400 bg-gray-50 dark:bg-gray-700/50';
    }
  };

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-800">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <Shield className="w-5 h-5 text-blue-500" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            PCB Compliance
          </h2>
        </div>
        <button
          onClick={runFullAudit}
          disabled={!project || runningAudit}
          className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors disabled:opacity-50 flex items-center gap-2"
        >
          {runningAudit ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Running...
            </>
          ) : (
            <>
              <RefreshCw className="w-4 h-4" />
              Run Full Audit
            </>
          )}
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-4 mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-700 dark:text-red-300 flex items-center gap-2">
          <AlertTriangle className="w-4 h-4" />
          {error}
          <button
            onClick={() => setError(null)}
            className="ml-auto p-1 hover:bg-red-100 dark:hover:bg-red-900/40 rounded"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {/* Navigation */}
      <div className="flex border-b border-gray-200 dark:border-gray-700 overflow-x-auto">
        {[
          { id: 'overview', label: 'Overview' },
          { id: 'ipc2221', label: 'IPC-2221' },
          { id: 'emi', label: 'EMI' },
          { id: 'nets', label: 'Nets' },
          { id: 'rules', label: 'Custom Rules' },
          { id: 'calculator', label: 'Trace Width' },
        ].map(({ id, label }) => (
          <button
            key={id}
            onClick={() => setActiveSection(id as any)}
            className={`px-4 py-2 text-sm font-medium border-b-2 transition-colors ${
              activeSection === id
                ? 'border-blue-500 text-blue-600 dark:text-blue-400'
                : 'border-transparent text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white'
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {activeSection === 'overview' && (
          <div className="space-y-6">
            {!auditReport ? (
              <div className="text-center py-12">
                <Shield className="w-12 h-12 mx-auto mb-4 text-gray-400" />
                <p className="text-gray-500 dark:text-gray-400 mb-4">
                  Run a compliance audit to see results
                </p>
                <button
                  onClick={runFullAudit}
                  disabled={!project || runningAudit}
                  className="px-6 py-3 text-white bg-blue-600 rounded-lg hover:bg-blue-700 disabled:opacity-50"
                >
                  {runningAudit ? 'Running...' : 'Run Full Audit'}
                </button>
              </div>
            ) : (
              <>
                {/* Summary Cards */}
                <div className="grid grid-cols-3 gap-4">
                  <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                    <div className="text-2xl font-bold text-gray-900 dark:text-white">
                      {auditReport.total_issues}
                    </div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">Total Issues</div>
                  </div>
                  <div className="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg">
                    <div className="text-2xl font-bold text-red-600 dark:text-red-400">
                      {auditReport.critical_issues}
                    </div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">Critical</div>
                  </div>
                  <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg">
                    <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                      {auditReport.ipc2221_summary.traces_analyzed}
                    </div>
                    <div className="text-sm text-gray-600 dark:text-gray-400">Traces Analyzed</div>
                  </div>
                </div>

                {/* Quick Actions */}
                <div className="grid grid-cols-2 gap-3">
                  <button
                    onClick={runIPC2221}
                    disabled={loading}
                    className="p-4 text-left border border-gray-200 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                  >
                    <div className="font-medium text-gray-900 dark:text-white">IPC-2221 Analysis</div>
                    <div className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                      Current capacity analysis
                    </div>
                  </button>
                  <button
                    onClick={runEMI}
                    disabled={loading}
                    className="p-4 text-left border border-gray-200 dark:border-gray-600 rounded-lg hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
                  >
                    <div className="font-medium text-gray-900 dark:text-white">EMI Analysis</div>
                    <div className="text-sm text-gray-600 dark:text-gray-400 mt-1">
                      Signal integrity issues
                    </div>
                  </button>
                </div>
              </>
            )}
          </div>
        )}

        {activeSection === 'ipc2221' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">IPC-2221 Analysis</h3>
              <button
                onClick={runIPC2221}
                disabled={loading}
                className="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
              >
                {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
                Analyze
              </button>
            </div>

            {ipcReport ? (
              <div className="space-y-4">
                <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="grid grid-cols-2 gap-4 text-sm">
                    <div>
                      <span className="text-gray-600 dark:text-gray-400">Temperature Rise:</span>
                      <span className="ml-2 font-medium text-gray-900 dark:text-white">
                        {ipcReport.temp_rise_c}°C
                      </span>
                    </div>
                    <div>
                      <span className="text-gray-600 dark:text-gray-400">Copper Weight:</span>
                      <span className="ml-2 font-medium text-gray-900 dark:text-white">
                        {ipcReport.outer_copper_oz} oz
                      </span>
                    </div>
                    <div>
                      <span className="text-gray-600 dark:text-gray-400">Traces Analyzed:</span>
                      <span className="ml-2 font-medium text-gray-900 dark:text-white">
                        {ipcReport.trace_analyses.length}
                      </span>
                    </div>
                    <div>
                      <span className="text-gray-600 dark:text-gray-400">Nets Analyzed:</span>
                      <span className="ml-2 font-medium text-gray-900 dark:text-white">
                        {ipcReport.net_summaries.length}
                      </span>
                    </div>
                  </div>
                </div>

                {ipcReport.net_summaries.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">Net Summaries</h4>
                    <div className="space-y-2">
                      {ipcReport.net_summaries.map((net, idx) => (
                        <div key={idx} className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-sm">
                          <div className="font-medium text-gray-900 dark:text-white">{net.net_name}</div>
                          <div className="text-gray-600 dark:text-gray-400 mt-1">
                            Width: {net.min_width_mm.toFixed(2)}-{net.max_width_mm.toFixed(2)}mm | 
                            Capacity: {net.min_current_capacity_a.toFixed(2)}A | 
                            Length: {net.total_length_mm.toFixed(1)}mm
                          </div>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="text-center py-12 text-gray-500 dark:text-gray-400">
                Click "Analyze" to run IPC-2221 analysis
              </div>
            )}
          </div>
        )}

        {activeSection === 'emi' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">EMI Analysis</h3>
              <div className="flex items-center gap-2">
                <select
                  value={selectedSeverity || ''}
                  onChange={(e) => setSelectedSeverity(e.target.value || null)}
                  className="px-3 py-2 text-sm bg-gray-100 dark:bg-gray-700 border-0 rounded-lg"
                >
                  <option value="">All Severities</option>
                  <option value="Critical">Critical</option>
                  <option value="High">High</option>
                  <option value="Medium">Medium</option>
                  <option value="Low">Low</option>
                </select>
                <button
                  onClick={runEMI}
                  disabled={loading}
                  className="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
                >
                  {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
                  Analyze
                </button>
              </div>
            </div>

            {emiReport ? (
              <div className="space-y-4">
                <div className="grid grid-cols-4 gap-4">
                  <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-center">
                    <div className="text-2xl font-bold text-gray-900 dark:text-white">
                      {emiReport.total_issues}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">Total</div>
                  </div>
                  <div className="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg text-center">
                    <div className="text-2xl font-bold text-red-600 dark:text-red-400">
                      {emiReport.critical_count}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">Critical</div>
                  </div>
                  <div className="p-4 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg text-center">
                    <div className="text-2xl font-bold text-yellow-600 dark:text-yellow-400">
                      {emiReport.high_count}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">High</div>
                  </div>
                  <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-center">
                    <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                      {emiReport.medium_count}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">Medium</div>
                  </div>
                </div>

                {filteredEmiIssues.length > 0 ? (
                  <div className="space-y-2">
                    {filteredEmiIssues.map((issue) => (
                      <div
                        key={issue.id}
                        className={`p-4 rounded-lg border ${getSeverityColor(issue.severity)}`}
                      >
                        <div className="flex items-start justify-between">
                          <div className="flex-1">
                            <div className="flex items-center gap-2 mb-1">
                              <span className="text-xs font-medium px-2 py-0.5 rounded">
                                {issue.severity}
                              </span>
                              <span className="text-xs text-gray-600 dark:text-gray-400">
                                {issue.category}
                              </span>
                            </div>
                            <div className="font-medium text-gray-900 dark:text-white mb-1">
                              {issue.net_name}
                            </div>
                            <div className="text-sm text-gray-700 dark:text-gray-300">
                              {issue.message}
                            </div>
                            <div className="text-sm text-gray-600 dark:text-gray-400 mt-2">
                              <strong>Recommendation:</strong> {issue.recommendation}
                            </div>
                          </div>
                        </div>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="text-center py-8 text-gray-500 dark:text-gray-400">
                    No EMI issues found
                  </div>
                )}

                {emiReport.recommendations.length > 0 && (
                  <div className="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                    <h4 className="font-medium text-blue-900 dark:text-blue-300 mb-2">Recommendations</h4>
                    <ul className="space-y-1 text-sm text-blue-800 dark:text-blue-200">
                      {emiReport.recommendations.map((rec, idx) => (
                        <li key={idx}>• {rec}</li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            ) : (
              <div className="text-center py-12 text-gray-500 dark:text-gray-400">
                Click "Analyze" to run EMI analysis
              </div>
            )}
          </div>
        )}

        {activeSection === 'nets' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Net Classification</h3>
              <button
                onClick={runNetClassification}
                disabled={loading}
                className="px-4 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
              >
                {loading ? <Loader2 className="w-4 h-4 animate-spin" /> : <RefreshCw className="w-4 h-4" />}
                Classify
              </button>
            </div>

            {netSummary ? (
              <div className="space-y-4">
                <div className="grid grid-cols-3 gap-4">
                  <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-center">
                    <div className="text-2xl font-bold text-gray-900 dark:text-white">
                      {netSummary.total_nets}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">Total Nets</div>
                  </div>
                  <div className="p-4 bg-purple-50 dark:bg-purple-900/20 rounded-lg text-center">
                    <div className="text-2xl font-bold text-purple-600 dark:text-purple-400">
                      {netSummary.high_speed_count}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">High-Speed</div>
                  </div>
                  <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg text-center">
                    <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                      {netSummary.clock_count}
                    </div>
                    <div className="text-xs text-gray-600 dark:text-gray-400">Clock</div>
                  </div>
                </div>

                {netSummary.high_speed_nets.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                      High-Speed Nets ({netSummary.high_speed_nets.length})
                    </h4>
                    <div className="flex flex-wrap gap-2">
                      {netSummary.high_speed_nets.map((net) => (
                        <span
                          key={net}
                          className="px-2 py-1 text-xs bg-purple-100 dark:bg-purple-900/30 text-purple-700 dark:text-purple-300 rounded"
                        >
                          {net}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {netSummary.clock_nets.length > 0 && (
                  <div>
                    <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                      Clock Nets ({netSummary.clock_nets.length})
                    </h4>
                    <div className="flex flex-wrap gap-2">
                      {netSummary.clock_nets.map((net) => (
                        <span
                          key={net}
                          className="px-2 py-1 text-xs bg-blue-100 dark:bg-blue-900/30 text-blue-700 dark:text-blue-300 rounded"
                        >
                          {net}
                        </span>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ) : (
              <div className="text-center py-12 text-gray-500 dark:text-gray-400">
                Click "Classify" to classify PCB nets
              </div>
            )}
          </div>
        )}

        {activeSection === 'rules' && (
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Custom Rules</h3>
              <div className="flex items-center gap-2">
                <button
                  onClick={loadSampleRules}
                  disabled={loadingRules}
                  className="px-3 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors disabled:opacity-50"
                >
                  Load Sample
                </button>
                {isTauri() && (
                  <>
                    <button
                      onClick={handleImportRules}
                      className="px-3 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors flex items-center gap-2"
                    >
                      <Upload className="w-4 h-4" />
                      Import
                    </button>
                    {rulesJson && (
                      <button
                        onClick={handleExportRules}
                        className="px-3 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors flex items-center gap-2"
                      >
                        <Download className="w-4 h-4" />
                        Export
                      </button>
                    )}
                  </>
                )}
              </div>
            </div>

            <div className="space-y-3">
              <textarea
                value={rulesJson}
                onChange={(e) => setRulesJson(e.target.value)}
                placeholder="Paste rules JSON here or load sample..."
                className="w-full h-64 px-3 py-2 text-sm font-mono text-gray-900 dark:text-white bg-gray-50 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
              <button
                onClick={loadCustomRules}
                disabled={loadingRules || !rulesJson.trim()}
                className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
              >
                {loadingRules ? (
                  <>
                    <Loader2 className="w-4 h-4 animate-spin" />
                    Loading...
                  </>
                ) : (
                  <>
                    <FileText className="w-4 h-4" />
                    Load Rules
                  </>
                )}
              </button>
            </div>

            {rulesLoaded && (
              <div className="p-3 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-sm text-green-700 dark:text-green-300">
                Rules loaded successfully
              </div>
            )}

            {filteredViolations.length > 0 && (
              <div>
                <div className="flex items-center justify-between mb-3">
                  <h4 className="text-sm font-medium text-gray-900 dark:text-white">
                    Violations ({filteredViolations.length})
                  </h4>
                  <div className="flex items-center gap-2">
                    <select
                      value={selectedSeverity || ''}
                      onChange={(e) => setSelectedSeverity(e.target.value || null)}
                      className="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 border-0 rounded"
                    >
                      <option value="">All Severities</option>
                      <option value="Error">Error</option>
                      <option value="Warning">Warning</option>
                      <option value="Info">Info</option>
                    </select>
                    <select
                      value={selectedCategory || ''}
                      onChange={(e) => setSelectedCategory(e.target.value || null)}
                      className="px-2 py-1 text-xs bg-gray-100 dark:bg-gray-700 border-0 rounded"
                    >
                      <option value="">All Categories</option>
                      <option value="Manufacturing">Manufacturing</option>
                      <option value="Signal">Signal</option>
                      <option value="Power">Power</option>
                      <option value="Thermal">Thermal</option>
                      <option value="Mechanical">Mechanical</option>
                      <option value="Safety">Safety</option>
                    </select>
                  </div>
                </div>
                <div className="space-y-2">
                  {filteredViolations.map((violation) => (
                    <div
                      key={violation.rule_id}
                      className={`p-3 rounded-lg border ${getSeverityColor(violation.severity)}`}
                    >
                      <div className="flex items-start justify-between">
                        <div className="flex-1">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="font-medium text-gray-900 dark:text-white">
                              {violation.rule_name}
                            </span>
                            <span className="text-xs px-2 py-0.5 rounded">
                              {violation.severity}
                            </span>
                          </div>
                          <div className="text-sm text-gray-700 dark:text-gray-300">
                            {violation.message}
                          </div>
                          {violation.suggestion && (
                            <div className="text-sm text-gray-600 dark:text-gray-400 mt-2">
                              <strong>Suggestion:</strong> {violation.suggestion}
                            </div>
                          )}
                          {violation.affected_items.length > 0 && (
                            <div className="text-xs text-gray-500 dark:text-gray-500 mt-2">
                              Affected: {violation.affected_items.join(', ')}
                            </div>
                          )}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        )}

        {activeSection === 'calculator' && (
          <TraceWidthCalculator />
        )}
      </div>
    </div>
  );
}

// Trace Width Calculator Component
function TraceWidthCalculator() {
  const [currentA, setCurrentA] = useState('1.0');
  const [copperOz, setCopperOz] = useState('1.0');
  const [tempRiseC, setTempRiseC] = useState('10.0');
  const [isExternal, setIsExternal] = useState(true);
  const [result, setResult] = useState<TraceWidthResult | null>(null);
  const [calculating, setCalculating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const calculate = async () => {
    const current = parseFloat(currentA);
    const copper = parseFloat(copperOz);
    const temp = parseFloat(tempRiseC);

    if (isNaN(current) || current <= 0) {
      setError('Current must be greater than 0');
      return;
    }
    if (isNaN(copper) || copper <= 0) {
      setError('Copper weight must be greater than 0');
      return;
    }
    if (isNaN(temp) || temp <= 0) {
      setError('Temperature rise must be greater than 0');
      return;
    }

    setCalculating(true);
    setError(null);

    try {
      const res = await api.calculateTraceWidth(current, copper, temp, isExternal);
      setResult(res);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to calculate trace width');
    } finally {
      setCalculating(false);
    }
  };

  return (
    <div className="space-y-4">
      <h3 className="text-lg font-semibold text-gray-900 dark:text-white">Trace Width Calculator</h3>
      
      <div className="p-4 bg-gray-50 dark:bg-gray-700/50 rounded-lg space-y-4">
        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Current (A)
          </label>
          <input
            type="number"
            value={currentA}
            onChange={(e) => setCurrentA(e.target.value)}
            step="0.1"
            min="0.1"
            className="w-full px-3 py-2 text-sm text-gray-900 dark:text-white bg-white dark:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Copper Weight (oz)
          </label>
          <select
            value={copperOz}
            onChange={(e) => setCopperOz(e.target.value)}
            className="w-full px-3 py-2 text-sm text-gray-900 dark:text-white bg-white dark:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
          >
            <option value="0.5">0.5 oz</option>
            <option value="1.0">1.0 oz</option>
            <option value="2.0">2.0 oz</option>
            <option value="3.0">3.0 oz</option>
          </select>
        </div>

        <div>
          <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
            Temperature Rise (°C)
          </label>
          <input
            type="number"
            value={tempRiseC}
            onChange={(e) => setTempRiseC(e.target.value)}
            step="1"
            min="1"
            className="w-full px-3 py-2 text-sm text-gray-900 dark:text-white bg-white dark:bg-gray-600 border border-gray-300 dark:border-gray-500 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
          />
        </div>

        <div>
          <label className="flex items-center gap-2 cursor-pointer">
            <input
              type="checkbox"
              checked={isExternal}
              onChange={(e) => setIsExternal(e.target.checked)}
              className="text-blue-500"
            />
            <span className="text-sm text-gray-700 dark:text-gray-300">External Layer</span>
          </label>
        </div>

        <button
          onClick={calculate}
          disabled={calculating}
          className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
        >
          {calculating ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Calculating...
            </>
          ) : (
            <>
              <Calculator className="w-4 h-4" />
              Calculate
            </>
          )}
        </button>

        {error && (
          <div className="p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-xs text-red-700 dark:text-red-300">
            {error}
          </div>
        )}

        {result && (
          <div className="p-4 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
            <div className="text-sm font-medium text-blue-900 dark:text-blue-300 mb-2">
              Required Trace Width
            </div>
            <div className="text-2xl font-bold text-blue-600 dark:text-blue-400">
              {result.required_width_mm.toFixed(2)} mm
            </div>
            <div className="text-sm text-blue-700 dark:text-blue-300 mt-1">
              {result.required_width_mils.toFixed(1)} mils
            </div>
            <div className="text-xs text-blue-600 dark:text-blue-400 mt-3 pt-3 border-t border-blue-200 dark:border-blue-700">
              For {result.current_a}A @ {result.copper_oz}oz, {result.temp_rise_c}°C rise, {result.is_external ? 'external' : 'internal'} layer
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
