import { useState, useEffect } from 'react';
import { Network, BarChart3, Search, Loader2, AlertCircle, Download, X, ChevronDown, ChevronRight, RefreshCw } from 'lucide-react';
import { api, isTauri } from '../lib/api';
import { useStore } from '../lib/store';
import type { 
  CircuitStats, 
  DecouplingAnalysisResponse, 
  ConnectivityAnalysisResponse, 
  SignalAnalysisResponse,
  ComponentInfo,
  NetInfo,
} from '../types';

export function CircuitAnalysisPanel() {
  const { project, showToast } = useStore();
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  
  // Statistics
  const [stats, setStats] = useState<CircuitStats | null>(null);
  
  // Analyses
  const [decouplingAnalysis, setDecouplingAnalysis] = useState<DecouplingAnalysisResponse | null>(null);
  const [connectivityAnalysis, setConnectivityAnalysis] = useState<ConnectivityAnalysisResponse | null>(null);
  const [signalAnalysis, setSignalAnalysis] = useState<SignalAnalysisResponse | null>(null);
  
  // Explorer
  const [netQuery, setNetQuery] = useState('');
  const [componentQuery, setComponentQuery] = useState('');
  const [netComponents, setNetComponents] = useState<ComponentInfo[]>([]);
  const [componentNets, setComponentNets] = useState<NetInfo[]>([]);
  const [querying, setQuerying] = useState(false);
  const [explorerError, setExplorerError] = useState<string | null>(null);
  const [lastQueryType, setLastQueryType] = useState<'net' | 'component' | null>(null);
  
  // UI State
  const [, setActiveSection] = useState<'stats' | 'decoupling' | 'connectivity' | 'signals' | 'explorer'>('stats');
  const [expandedSections, setExpandedSections] = useState<Set<string>>(new Set(['stats']));

  useEffect(() => {
    if (project) {
      loadStats();
    }
  }, [project]);

  const loadStats = async () => {
    if (!project) return;
    
    setLoading(true);
    setError(null);
    try {
      const s = await api.getCircuitStats();
      setStats(s);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load circuit statistics');
    } finally {
      setLoading(false);
    }
  };

  const analyzeDecoupling = async () => {
    setLoading(true);
    setError(null);
    try {
      const analysis = await api.analyzeCircuitDecoupling();
      setDecouplingAnalysis(analysis);
      setActiveSection('decoupling');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to analyze decoupling');
    } finally {
      setLoading(false);
    }
  };

  const analyzeConnectivity = async () => {
    setLoading(true);
    setError(null);
    try {
      const analysis = await api.analyzeCircuitConnectivity();
      setConnectivityAnalysis(analysis);
      setActiveSection('connectivity');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to analyze connectivity');
    } finally {
      setLoading(false);
    }
  };

  const analyzeSignals = async () => {
    setLoading(true);
    setError(null);
    try {
      const analysis = await api.analyzeCircuitSignals();
      setSignalAnalysis(analysis);
      setActiveSection('signals');
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to analyze signals');
    } finally {
      setLoading(false);
    }
  };

  const queryNet = async () => {
    if (!netQuery.trim()) return;
    
    setQuerying(true);
    setError(null);
    setExplorerError(null);
    setComponentNets([]);
    try {
      const components = await api.getNetComponents(netQuery.trim());
      setNetComponents(components);
      setLastQueryType('net');
      setActiveSection('explorer');
      setExpandedSections((prev) => new Set([...prev, 'explorer']));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setExplorerError(msg);
      setNetComponents([]);
      setLastQueryType(null);
    } finally {
      setQuerying(false);
    }
  };

  const queryComponent = async () => {
    if (!componentQuery.trim()) return;
    
    setQuerying(true);
    setError(null);
    setExplorerError(null);
    setNetComponents([]);
    try {
      const nets = await api.getComponentNets(componentQuery.trim());
      setComponentNets(nets);
      setLastQueryType('component');
      setActiveSection('explorer');
      setExpandedSections((prev) => new Set([...prev, 'explorer']));
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      setError(msg);
      setExplorerError(msg);
      setComponentNets([]);
      setLastQueryType(null);
    } finally {
      setQuerying(false);
    }
  };

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  const exportUCS = async () => {
    if (!project) {
      setError('Please open a project first');
      return;
    }

    try {
      const ucs = await api.getCircuitUCS();
      const json = JSON.stringify(ucs, null, 2);

      if (isTauri()) {
        // Use Tauri's save dialog
        const { save } = await import('@tauri-apps/plugin-dialog');
        const path = await save({
          filters: [{
            name: 'JSON',
            extensions: ['json']
          }],
          defaultPath: 'circuit_ucs.json',
          title: 'Save UCS File',
        });

        if (path) {
          const { writeTextFile } = await import('@tauri-apps/plugin-fs');
          // Write to the user-selected path (path from save dialog is already absolute)
          await writeTextFile(path, json);
          showToast('UCS file saved successfully!');
        }
      } else {
        // Fallback for web browser
        const blob = new Blob([json], { type: 'application/json' });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'circuit_ucs.json';
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);
        showToast('UCS file downloaded successfully!');
      }
    } catch (e) {
      setError(`Failed to export UCS: ${e instanceof Error ? e.message : String(e)}`);
    }
  };

  if (!project) {
    return (
      <div className="flex items-center justify-center h-full text-gray-500 dark:text-gray-400">
        Open a project to analyze circuit
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-800">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <Network className="w-5 h-5 text-blue-500" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Circuit Analysis
          </h2>
        </div>
        <button
          onClick={exportUCS}
          disabled={!project}
          className="px-3 py-2 text-sm bg-gray-200 dark:bg-gray-600 hover:bg-gray-300 dark:hover:bg-gray-500 rounded-lg transition-colors flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          <Download className="w-4 h-4" />
          Export UCS
        </button>
      </div>

      {/* Error */}
      {error && (
        <div className="mx-4 mt-4 p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-700 dark:text-red-300 flex items-center gap-2">
          <AlertCircle className="w-4 h-4" />
          {error}
          <button
            onClick={() => setError(null)}
            className="ml-auto p-1 hover:bg-red-100 dark:hover:bg-red-900/40 rounded"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4">
        {/* Statistics */}
        <div className="border border-gray-200 dark:border-gray-600 rounded-lg">
          <button
            onClick={() => toggleSection('stats')}
            className="w-full p-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            <div className="flex items-center gap-2">
              {expandedSections.has('stats') ? (
                <ChevronDown className="w-4 h-4 text-gray-400" />
              ) : (
                <ChevronRight className="w-4 h-4 text-gray-400" />
              )}
              <BarChart3 className="w-5 h-5 text-blue-500" />
              <span className="font-medium text-gray-900 dark:text-white">Circuit Statistics</span>
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                loadStats();
              }}
              disabled={loading}
              className="p-2 hover:bg-gray-200 dark:hover:bg-gray-600 rounded transition-colors"
            >
              {loading ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                <RefreshCw className="w-4 h-4" />
              )}
            </button>
          </button>
          {expandedSections.has('stats') && stats && (
            <div className="px-4 pb-4 space-y-3">
              <div className="grid grid-cols-2 gap-4">
                <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="text-2xl font-bold text-gray-900 dark:text-white">
                    {stats.component_count}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Components</div>
                </div>
                <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="text-2xl font-bold text-gray-900 dark:text-white">
                    {stats.net_count}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Nets</div>
                </div>
                <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="text-2xl font-bold text-gray-900 dark:text-white">
                    {stats.ic_count}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">ICs</div>
                </div>
                <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="text-2xl font-bold text-gray-900 dark:text-white">
                    {stats.power_net_count}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Power Nets</div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Decoupling Analysis */}
        <div className="border border-gray-200 dark:border-gray-600 rounded-lg">
          <button
            onClick={() => {
              toggleSection('decoupling');
              if (!decouplingAnalysis) {
                analyzeDecoupling();
              }
            }}
            className="w-full p-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            <div className="flex items-center gap-2">
              {expandedSections.has('decoupling') ? (
                <ChevronDown className="w-4 h-4 text-gray-400" />
              ) : (
                <ChevronRight className="w-4 h-4 text-gray-400" />
              )}
              <span className="font-medium text-gray-900 dark:text-white">Decoupling Analysis</span>
            </div>
          </button>
          {expandedSections.has('decoupling') && decouplingAnalysis && (
            <div className="px-4 pb-4 space-y-3">
              <div className="grid grid-cols-2 gap-4">
                <div className="p-3 bg-green-50 dark:bg-green-900/20 rounded-lg">
                  <div className="text-xl font-bold text-green-600 dark:text-green-400">
                    {decouplingAnalysis.ics_with_decoupling}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">ICs with Decoupling</div>
                </div>
                <div className="p-3 bg-red-50 dark:bg-red-900/20 rounded-lg">
                  <div className="text-xl font-bold text-red-600 dark:text-red-400">
                    {decouplingAnalysis.ics_missing_decoupling}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Missing Decoupling</div>
                </div>
              </div>
              {decouplingAnalysis.missing_details.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                    Missing Decoupling Details
                  </h4>
                  <div className="space-y-2">
                    {decouplingAnalysis.missing_details.map((detail, idx) => (
                      <div key={idx} className="p-3 bg-yellow-50 dark:bg-yellow-900/20 rounded-lg text-sm">
                        <div className="font-medium text-gray-900 dark:text-white">
                          {detail.ic_ref} {detail.ic_value && `(${detail.ic_value})`}
                        </div>
                        <div className="text-gray-600 dark:text-gray-400 mt-1">
                          {detail.recommendation}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Connectivity Analysis */}
        <div className="border border-gray-200 dark:border-gray-600 rounded-lg">
          <button
            onClick={() => {
              toggleSection('connectivity');
              if (!connectivityAnalysis) {
                analyzeConnectivity();
              }
            }}
            className="w-full p-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            <div className="flex items-center gap-2">
              {expandedSections.has('connectivity') ? (
                <ChevronDown className="w-4 h-4 text-gray-400" />
              ) : (
                <ChevronRight className="w-4 h-4 text-gray-400" />
              )}
              <span className="font-medium text-gray-900 dark:text-white">Connectivity Analysis</span>
            </div>
          </button>
          {expandedSections.has('connectivity') && connectivityAnalysis && (
            <div className="px-4 pb-4 space-y-3">
              <div className="grid grid-cols-2 gap-4">
                <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="text-xl font-bold text-gray-900 dark:text-white">
                    {connectivityAnalysis.floating_components.length}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Floating Components</div>
                </div>
                <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                  <div className="text-xl font-bold text-gray-900 dark:text-white">
                    {connectivityAnalysis.single_connection_nets.length}
                  </div>
                  <div className="text-sm text-gray-600 dark:text-gray-400">Single-Connection Nets</div>
                </div>
              </div>
              {connectivityAnalysis.floating_components.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                    Floating Components
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {connectivityAnalysis.floating_components.map((ref) => (
                      <span
                        key={ref}
                        className="px-2 py-1 text-xs bg-red-100 dark:bg-red-900/30 text-red-700 dark:text-red-300 rounded"
                      >
                        {ref}
                      </span>
                    ))}
                  </div>
                </div>
              )}
              {connectivityAnalysis.single_connection_nets.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                    Single-Connection Nets
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {connectivityAnalysis.single_connection_nets.map((net) => (
                      <span
                        key={net}
                        className="px-2 py-1 text-xs bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 rounded"
                      >
                        {net}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Signal Analysis */}
        <div className="border border-gray-200 dark:border-gray-600 rounded-lg">
          <button
            onClick={() => {
              toggleSection('signals');
              if (!signalAnalysis) {
                analyzeSignals();
              }
            }}
            className="w-full p-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            <div className="flex items-center gap-2">
              {expandedSections.has('signals') ? (
                <ChevronDown className="w-4 h-4 text-gray-400" />
              ) : (
                <ChevronRight className="w-4 h-4 text-gray-400" />
              )}
              <span className="font-medium text-gray-900 dark:text-white">Signal Analysis</span>
            </div>
          </button>
          {expandedSections.has('signals') && signalAnalysis && (
            <div className="px-4 pb-4 space-y-3">
              <div className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                <div className="text-xl font-bold text-gray-900 dark:text-white">
                  {signalAnalysis.i2c_buses.length}
                </div>
                <div className="text-sm text-gray-600 dark:text-gray-400">I2C Buses</div>
              </div>
              {signalAnalysis.i2c_buses.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                    I2C Buses
                  </h4>
                  <div className="space-y-2">
                    {signalAnalysis.i2c_buses.map((bus, idx) => (
                      <div key={idx} className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg text-sm">
                        <div className="flex items-center gap-2 mb-1">
                          <span className="font-medium text-gray-900 dark:text-white">
                            SDA: {bus.sda_net || 'N/A'} | SCL: {bus.scl_net || 'N/A'}
                          </span>
                          {bus.has_pullups ? (
                            <span className="px-2 py-0.5 text-xs bg-green-100 dark:bg-green-900/30 text-green-700 dark:text-green-300 rounded">
                              Pull-ups OK
                            </span>
                          ) : (
                            <span className="px-2 py-0.5 text-xs bg-yellow-100 dark:bg-yellow-900/30 text-yellow-700 dark:text-yellow-300 rounded">
                              Missing Pull-ups
                            </span>
                          )}
                        </div>
                        <div className="text-gray-600 dark:text-gray-400">
                          {bus.device_count} devices
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {signalAnalysis.unterminated_signals.length > 0 && (
                <div>
                  <h4 className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                    Unterminated Signals ({signalAnalysis.unterminated_signal_count})
                  </h4>
                  <div className="flex flex-wrap gap-2">
                    {signalAnalysis.unterminated_signals.map((net) => (
                      <span
                        key={net}
                        className="px-2 py-1 text-xs bg-orange-100 dark:bg-orange-900/30 text-orange-700 dark:text-orange-300 rounded"
                      >
                        {net}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        {/* Net/Component Explorer */}
        <div className="border border-gray-200 dark:border-gray-600 rounded-lg">
          <button
            onClick={() => toggleSection('explorer')}
            className="w-full p-4 flex items-center justify-between hover:bg-gray-50 dark:hover:bg-gray-700 transition-colors"
          >
            <div className="flex items-center gap-2">
              {expandedSections.has('explorer') ? (
                <ChevronDown className="w-4 h-4 text-gray-400" />
              ) : (
                <ChevronRight className="w-4 h-4 text-gray-400" />
              )}
              <Search className="w-5 h-5 text-blue-500" />
              <span className="font-medium text-gray-900 dark:text-white">Net/Component Explorer</span>
            </div>
          </button>
          {expandedSections.has('explorer') && (
            <div className="px-4 pb-4 space-y-4">
              {explorerError && (
                <div className="p-3 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-sm text-red-700 dark:text-red-300 flex items-center gap-2">
                  <AlertCircle className="w-4 h-4 flex-shrink-0" />
                  <span className="flex-1">{explorerError}</span>
                  <button
                    type="button"
                    onClick={() => setExplorerError(null)}
                    className="p-1 hover:bg-red-100 dark:hover:bg-red-900/40 rounded"
                    aria-label="Dismiss"
                  >
                    <X className="w-4 h-4" />
                  </button>
                </div>
              )}

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Query Net (find components on net)
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={netQuery}
                    onChange={(e) => setNetQuery(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && queryNet()}
                    placeholder="e.g., VCC, GND, I2C_SDA"
                    className="flex-1 px-3 py-2 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                  <button
                    type="button"
                    onClick={queryNet}
                    disabled={querying || !netQuery.trim()}
                    className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors min-w-[4.5rem]"
                  >
                    {querying ? <Loader2 className="w-4 h-4 animate-spin" /> : 'Query'}
                  </button>
                </div>
                {lastQueryType === 'net' && !querying && (
                  <div className="mt-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                    <div className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                      Components on {netQuery}:
                    </div>
                    {netComponents.length > 0 ? (
                      <div className="space-y-1">
                        {netComponents.map((comp) => (
                          <div key={comp.ref_des} className="text-sm text-gray-600 dark:text-gray-400">
                            {comp.ref_des} {comp.value && `(${comp.value})`} {comp.is_ic && '(IC)'}
                          </div>
                        ))}
                      </div>
                    ) : (
                      <p className="text-sm text-gray-500 dark:text-gray-400">No components found on this net.</p>
                    )}
                  </div>
                )}
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1">
                  Query Component (find nets on component)
                </label>
                <p className="text-xs text-gray-500 dark:text-gray-400 mb-1">
                  Use component reference (e.g. PWR1, R10, LCD-5in_DISP1), not net names like VLED+.
                </p>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={componentQuery}
                    onChange={(e) => setComponentQuery(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && queryComponent()}
                    placeholder="e.g., PWR1, R10, U1, LCD-5in_DISP1"
                    className="flex-1 px-3 py-2 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border border-gray-200 dark:border-gray-600 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                  />
                  <button
                    type="button"
                    onClick={queryComponent}
                    disabled={querying || !componentQuery.trim()}
                    className="px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors min-w-[4.5rem]"
                  >
                    {querying ? <Loader2 className="w-4 h-4 animate-spin" /> : 'Query'}
                  </button>
                </div>
                {lastQueryType === 'component' && !querying && (
                  <div className="mt-3 p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
                    <div className="text-sm font-medium text-gray-900 dark:text-white mb-2">
                      Nets on {componentQuery}:
                    </div>
                    {componentNets.length > 0 ? (
                      <div className="space-y-1">
                        {componentNets.map((net) => (
                          <div key={net.name} className="text-sm text-gray-600 dark:text-gray-400">
                            {net.name} {net.is_power_rail && '(Power)'} {net.voltage_level != null && `(${net.voltage_level}V)`}
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="space-y-1">
                        <p className="text-sm text-gray-500 dark:text-gray-400">
                          No component with reference &quot;{componentQuery}&quot;.
                        </p>
                        <p className="text-xs text-gray-500 dark:text-gray-400">
                          Use a component ref (e.g. PWR1, R10, LCD-5in_DISP1). For net names like VLED+ or +3V3, use Query Net above.
                        </p>
                      </div>
                    )}
                  </div>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
