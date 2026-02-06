import { useState, useEffect } from 'react';
import { AlertTriangle, CheckCircle, AlertCircle, Info, ChevronDown, ChevronRight, RefreshCw, Zap } from 'lucide-react';
import { api } from '../lib/api';
import type { ICRiskScore, NetCriticality } from '../types';

export function DRSPanel() {
  const [riskScores, setRiskScores] = useState<ICRiskScore[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expandedICs, setExpandedICs] = useState<Set<string>>(new Set());

  const parseError = (error: string): { title: string; details: string; suggestions: string[] } => {
    const lowerError = error.toLowerCase();
    
    // No schematic loaded
    if (lowerError.includes('no schematic') || lowerError.includes('schematic file first')) {
      return {
        title: 'No Schematic File Loaded',
        details: 'DRS analysis requires both a schematic (.kicad_sch) and PCB (.kicad_pcb) file to be loaded.',
        suggestions: [
          'Click "Open Project" and select your KiCAD project folder or schematic file',
          'The app will automatically find and load the corresponding PCB file',
          'Make sure both .kicad_sch and .kicad_pcb files exist in the same directory'
        ]
      };
    }
    
    // No PCB loaded
    if (lowerError.includes('no pcb') || lowerError.includes('pcb file first')) {
      return {
        title: 'No PCB File Loaded',
        details: 'DRS analysis requires a PCB layout file (.kicad_pcb) to analyze component placement and routing.',
        suggestions: [
          'Ensure your project has a PCB file (.kicad_pcb) in the same directory as the schematic',
          'Open the project folder instead of just the schematic file',
          'If the PCB file has a different name, rename it to match the schematic name'
        ]
      };
    }
    
    // Parse errors
    if (lowerError.includes('failed to parse')) {
      return {
        title: 'File Parse Error',
        details: error,
        suggestions: [
          'Verify that the files are valid KiCAD format',
          'Check if the files are corrupted or incomplete',
          'Try opening the files in KiCAD to ensure they load correctly'
        ]
      };
    }
    
    // Lock errors
    if (lowerError.includes('lock')) {
      return {
        title: 'Internal Error',
        details: 'The application encountered an internal error while accessing project data.',
        suggestions: [
          'Try closing and reopening the project',
          'Restart the application if the issue persists'
        ]
      };
    }
    
    // Generic error
    return {
      title: 'DRS Analysis Failed',
      details: error,
      suggestions: [
        'Ensure both schematic and PCB files are loaded',
        'Check that the files are valid KiCAD format',
        'Try refreshing the analysis'
      ]
    };
  };

  const loadDRSResults = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const results = await api.runDRSAnalysis();
      setRiskScores(results);
      // Auto-expand high-risk ICs
      const highRiskICs = results
        .filter(r => r.risk_index >= 50)
        .map(r => r.ic_reference);
      setExpandedICs(new Set(highRiskICs));
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to run DRS analysis';
      setError(errorMessage);
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    // Auto-load on mount
    loadDRSResults();
  }, []);

  const toggleIC = (icRef: string) => {
    const newExpanded = new Set(expandedICs);
    if (newExpanded.has(icRef)) {
      newExpanded.delete(icRef);
    } else {
      newExpanded.add(icRef);
    }
    setExpandedICs(newExpanded);
  };

  const getRiskColor = (riskIndex: number) => {
    if (riskIndex >= 70) return 'text-red-600 dark:text-red-400';
    if (riskIndex >= 50) return 'text-orange-600 dark:text-orange-400';
    if (riskIndex >= 30) return 'text-yellow-600 dark:text-yellow-400';
    return 'text-green-600 dark:text-green-400';
  };

  const getRiskIcon = (riskIndex: number) => {
    if (riskIndex >= 70) return <AlertTriangle className="w-5 h-5 text-red-600 dark:text-red-400" />;
    if (riskIndex >= 50) return <AlertCircle className="w-5 h-5 text-orange-600 dark:text-orange-400" />;
    if (riskIndex >= 30) return <Info className="w-5 h-5 text-yellow-600 dark:text-yellow-400" />;
    return <CheckCircle className="w-5 h-5 text-green-600 dark:text-green-400" />;
  };

  const getCriticalityColor = (criticality: NetCriticality) => {
    switch (criticality) {
      case 'Critical': return 'bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200';
      case 'High': return 'bg-orange-100 dark:bg-orange-900/30 text-orange-800 dark:text-orange-200';
      case 'Medium': return 'bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-200';
      case 'Low': return 'bg-blue-100 dark:bg-blue-900/30 text-blue-800 dark:text-blue-200';
    }
  };

  // Sort by risk index (highest first)
  const sortedScores = [...riskScores].sort((a, b) => b.risk_index - a.risk_index);

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <Zap className="w-5 h-5 text-purple-500" />
          <h2 className="font-semibold text-gray-900 dark:text-white">DRS Analysis</h2>
        </div>
        
        <button
          onClick={loadDRSResults}
          disabled={isLoading}
          className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          title="Re-run DRS analysis"
        >
          <RefreshCw className={isLoading ? 'w-4 h-4 animate-spin' : 'w-4 h-4'} />
        </button>
      </div>

      {/* Error message */}
      {error && (() => {
        const errorInfo = parseError(error);
        return (
          <div className="p-4 bg-red-50 dark:bg-red-900/20 border-b border-red-200 dark:border-red-800">
            <div className="flex items-start gap-3">
              <AlertTriangle className="w-5 h-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
              <div className="flex-1 space-y-2">
                <div>
                  <p className="text-sm font-semibold text-red-800 dark:text-red-200">{errorInfo.title}</p>
                  <p className="text-xs text-red-700 dark:text-red-300 mt-1">{errorInfo.details}</p>
                </div>
                {errorInfo.suggestions.length > 0 && (
                  <div className="mt-3 pt-3 border-t border-red-200 dark:border-red-800">
                    <p className="text-xs font-medium text-red-800 dark:text-red-200 mb-2">Suggestions:</p>
                    <ul className="space-y-1.5">
                      {errorInfo.suggestions.map((suggestion, idx) => (
                        <li key={idx} className="text-xs text-red-700 dark:text-red-300 flex items-start gap-2">
                          <span className="text-red-500 dark:text-red-400 mt-0.5">•</span>
                          <span>{suggestion}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            </div>
          </div>
        );
      })()}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-3">
        {isLoading ? (
          <div className="text-center text-gray-500 dark:text-gray-400 py-8">
            <RefreshCw className="w-6 h-6 mx-auto mb-2 animate-spin" />
            <p className="text-sm">Running DRS analysis...</p>
          </div>
        ) : error ? (
          <div className="text-center text-gray-500 dark:text-gray-400 py-8">
            <AlertCircle className="w-8 h-8 mx-auto mb-2 text-yellow-500" />
            <p className="text-sm font-medium">DRS Analysis Unavailable</p>
            <p className="text-xs mt-2 max-w-md mx-auto text-left">
              {parseError(error).details}
            </p>
          </div>
        ) : sortedScores.length === 0 ? (
          <div className="text-center text-gray-500 dark:text-gray-400 py-8">
            <Info className="w-8 h-8 mx-auto mb-2 text-blue-500" />
            <p className="text-sm font-medium">No DRS Results</p>
            <p className="text-xs mt-2">No ICs were analyzed. This could mean:</p>
            <ul className="text-xs mt-2 text-left max-w-md mx-auto space-y-1 list-disc list-inside">
              <li>No ICs found in schematic (components starting with 'U')</li>
              <li>ICs don't have footprints on the PCB</li>
              <li>IC footprints don't have detectable power pins</li>
            </ul>
          </div>
        ) : (
          sortedScores.map((score) => (
            <ICRiskCard
              key={score.ic_reference}
              score={score}
              expanded={expandedICs.has(score.ic_reference)}
              onToggle={() => toggleIC(score.ic_reference)}
              getRiskColor={getRiskColor}
              getRiskIcon={getRiskIcon}
              getCriticalityColor={getCriticalityColor}
            />
          ))
        )}
      </div>
    </div>
  );
}

interface ICRiskCardProps {
  score: ICRiskScore;
  expanded: boolean;
  onToggle: () => void;
  getRiskColor: (risk: number) => string;
  getRiskIcon: (risk: number) => JSX.Element;
  getCriticalityColor: (criticality: NetCriticality) => string;
}

function ICRiskCard({ score, expanded, onToggle, getRiskColor, getRiskIcon, getCriticalityColor }: ICRiskCardProps) {
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      {/* Header */}
      <div
        className="p-3 bg-gray-50 dark:bg-gray-800/50 cursor-pointer hover:bg-gray-100 dark:hover:bg-gray-800 transition-colors"
        onClick={onToggle}
      >
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3 flex-1">
            {expanded ? (
              <ChevronDown className="w-4 h-4 text-gray-500 dark:text-gray-400" />
            ) : (
              <ChevronRight className="w-4 h-4 text-gray-500 dark:text-gray-400" />
            )}
            {getRiskIcon(score.risk_index)}
            <div className="flex-1">
              <div className="flex items-center gap-2">
                <span className="font-medium text-gray-900 dark:text-white">
                  {score.ic_reference}
                </span>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  {score.ic_value}
                </span>
                <span className={`px-2 py-0.5 rounded text-xs font-medium ${getCriticalityColor(score.net_criticality)}`}>
                  {score.net_criticality}
                </span>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3">
            <span className={`text-lg font-bold ${getRiskColor(score.risk_index)}`}>
              {score.risk_index.toFixed(1)}
            </span>
          </div>
        </div>
      </div>

      {/* Expanded content */}
      {expanded && (
        <div className="p-4 space-y-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-900">
          {/* Risk breakdown */}
          <div>
            <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">Risk Breakdown</h4>
            <div className="space-y-2">
              <div className="flex justify-between text-sm">
                <span className="text-gray-600 dark:text-gray-400">Proximity Penalty:</span>
                <span className="font-medium">{score.proximity_penalty.toFixed(2)}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-600 dark:text-gray-400">Inductance Penalty:</span>
                <span className="font-medium">{score.inductance_penalty.toFixed(2)}</span>
              </div>
              <div className="flex justify-between text-sm">
                <span className="text-gray-600 dark:text-gray-400">Mismatch Penalty:</span>
                <span className="font-medium">{score.mismatch_penalty.toFixed(2)}</span>
              </div>
            </div>
          </div>

          {/* Decoupling capacitors */}
          {score.decoupling_capacitors.length > 0 ? (
            <div>
              <h4 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
                Decoupling Capacitors ({score.decoupling_capacitors.length})
              </h4>
              <div className="space-y-2">
                {score.decoupling_capacitors.map((cap, idx) => (
                  <div key={idx} className="p-2 bg-gray-50 dark:bg-gray-800 rounded text-sm">
                    <div className="flex justify-between items-start mb-1">
                      <span className="font-medium">{cap.capacitor_reference}</span>
                      <span className="text-gray-500 dark:text-gray-400">{cap.capacitor_value}</span>
                    </div>
                    <div className="grid grid-cols-2 gap-2 text-xs text-gray-600 dark:text-gray-400">
                      <div>Distance: {cap.distance_mm.toFixed(2)}mm</div>
                      <div>Vias: {cap.via_count}</div>
                      <div>SRF: {cap.capacitor_srf_mhz.toFixed(1)}MHz</div>
                      <div>IC Freq: {cap.ic_switching_freq_mhz.toFixed(1)}MHz</div>
                    </div>
                    {(cap.shared_via || cap.backside_offset || cap.neck_down) && (
                      <div className="mt-2 pt-2 border-t border-gray-200 dark:border-gray-700">
                        <div className="flex flex-wrap gap-1">
                          {cap.shared_via && (
                            <span className="px-1.5 py-0.5 bg-red-100 dark:bg-red-900/30 text-red-800 dark:text-red-200 rounded text-xs">
                              Shared Via
                            </span>
                          )}
                          {cap.backside_offset && (
                            <span className="px-1.5 py-0.5 bg-orange-100 dark:bg-orange-900/30 text-orange-800 dark:text-orange-200 rounded text-xs">
                              Backside Offset
                            </span>
                          )}
                          {cap.neck_down && (
                            <span className="px-1.5 py-0.5 bg-yellow-100 dark:bg-yellow-900/30 text-yellow-800 dark:text-yellow-200 rounded text-xs">
                              Neck-Down
                            </span>
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          ) : (
            <div className="p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded text-sm text-yellow-800 dark:text-yellow-200">
              No decoupling capacitors found for this IC
            </div>
          )}

          {/* High-risk heuristics */}
          {score.high_risk_heuristics.length > 0 && (
            <div>
              <h4 className="text-sm font-semibold text-red-700 dark:text-red-300 mb-2">
                High-Risk Heuristics ({score.high_risk_heuristics.length})
              </h4>
              <div className="space-y-2">
                {score.high_risk_heuristics.map((heuristic, idx) => {
                  if (heuristic.SharedVia) {
                    return (
                      <div key={idx} className="p-2 bg-red-50 dark:bg-red-900/20 rounded text-sm">
                        <span className="font-medium text-red-800 dark:text-red-200">Shared Via:</span>
                        <span className="text-red-700 dark:text-red-300 ml-2">
                          {heuristic.SharedVia.capacitor1} and {heuristic.SharedVia.capacitor2} share the same via
                        </span>
                      </div>
                    );
                  }
                  if (heuristic.BacksideOffset) {
                    return (
                      <div key={idx} className="p-2 bg-orange-50 dark:bg-orange-900/20 rounded text-sm">
                        <span className="font-medium text-orange-800 dark:text-orange-200">Backside Offset:</span>
                        <span className="text-orange-700 dark:text-orange-300 ml-2">
                          {heuristic.BacksideOffset.capacitor} on opposite side with {heuristic.BacksideOffset.via_count} vias
                        </span>
                      </div>
                    );
                  }
                  if (heuristic.NeckDown) {
                    return (
                      <div key={idx} className="p-2 bg-yellow-50 dark:bg-yellow-900/20 rounded text-sm">
                        <span className="font-medium text-yellow-800 dark:text-yellow-200">Neck-Down Effect:</span>
                        <span className="text-yellow-700 dark:text-yellow-300 ml-2">
                          {heuristic.NeckDown.capacitor} connected via {heuristic.NeckDown.trace_width_mm.toFixed(2)}mm trace
                        </span>
                      </div>
                    );
                  }
                  return null;
                })}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
