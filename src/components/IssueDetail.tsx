import { useState } from 'react';
import { 
  AlertCircle, 
  AlertTriangle, 
  Info, 
  ChevronDown, 
  ChevronRight,
  ExternalLink,
  Wrench,
  BookOpen,
  Lightbulb,
  Zap,
  X
} from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import type { DetailedIssue, Consequence } from '../types';

interface IssueDetailProps {
  issue: DetailedIssue;
  onClose: () => void;
  onDismiss?: (reason: string) => void;
}

export function IssueDetail({ issue, onClose, onDismiss }: IssueDetailProps) {
  const [showTechnical, setShowTechnical] = useState(false);
  const [showReferences, setShowReferences] = useState(false);

  const getSeverityIcon = (severity: string) => {
    switch (severity.toLowerCase()) {
      case 'error':
        return <AlertCircle className="w-5 h-5 text-red-600 dark:text-red-400" />;
      case 'warning':
        return <AlertTriangle className="w-5 h-5 text-yellow-600 dark:text-yellow-400" />;
      case 'info':
        return <Info className="w-5 h-5 text-blue-600 dark:text-blue-400" />;
      default:
        return <Lightbulb className="w-5 h-5 text-purple-600 dark:text-purple-400" />;
    }
  };

  const getConsequenceSeverityColor = (severity: string) => {
    switch (severity) {
      case 'critical':
        return 'text-red-800 dark:text-red-400 bg-red-100 dark:bg-red-900/30';
      case 'serious':
        return 'text-orange-800 dark:text-orange-400 bg-orange-100 dark:bg-orange-900/30';
      case 'problematic':
        return 'text-yellow-800 dark:text-yellow-400 bg-yellow-100 dark:bg-yellow-900/30';
      default:
        return 'text-blue-800 dark:text-blue-400 bg-blue-100 dark:bg-blue-900/30';
    }
  };

  const getLikelihoodBadge = (likelihood: string) => {
    const colors: Record<string, string> = {
      certain: 'bg-red-600 dark:bg-red-600',
      likely: 'bg-orange-600 dark:bg-orange-600',
      occasional: 'bg-orange-400 dark:bg-yellow-300',
      rare: 'bg-gray-500 dark:bg-gray-600',
    };
    return colors[likelihood] || 'bg-gray-500 dark:bg-gray-600';
  };

  return (
    <div className="fixed inset-0 bg-black/50 dark:bg-black/50 flex items-center justify-center z-50 p-4">
      <div className="bg-white dark:bg-gray-800 rounded-lg shadow-xl w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-start justify-between p-4 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-start gap-3">
            {getSeverityIcon(issue.severity)}
            <div>
              <h2 className="text-lg font-semibold text-gray-900 dark:text-white">{issue.title}</h2>
              <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{issue.summary}</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-white transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content - Scrollable */}
        <div className="flex-1 overflow-y-auto p-4 space-y-6">
          {/* Risk Score (if available) */}
          {issue.risk_score && (
            <section className="bg-gray-100 dark:bg-gray-700/50 rounded-lg p-4 border border-gray-300 dark:border-gray-600">
              <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
                <Zap className="w-4 h-4" />
                Risk Assessment
              </h3>
              <div className="space-y-2">
                <div className="flex items-center gap-3">
                  <span className="text-xs text-gray-600 dark:text-gray-400">Risk Index:</span>
                  <span className={`px-3 py-1 rounded font-mono text-sm font-medium ${
                    issue.risk_score.value >= 70 
                      ? 'bg-red-600 text-white'
                      : issue.risk_score.value >= 40
                      ? 'bg-yellow-600 text-white'
                      : 'bg-green-600 text-white'
                  }`}>
                    {issue.risk_score.value.toFixed(0)}/100
                  </span>
                </div>
                {issue.risk_score.inductance_nh !== undefined && (
                  <div className="text-sm text-gray-700 dark:text-gray-300">
                    <span className="text-gray-600 dark:text-gray-400">Inductance:</span>{' '}
                    <span className="font-mono text-blue-600 dark:text-blue-300">{issue.risk_score.inductance_nh.toFixed(1)} nH</span>
                    {issue.risk_score.limit_nh !== undefined && (
                      <>
                        {' '}/ <span className="font-mono text-gray-600 dark:text-gray-400">limit: {issue.risk_score.limit_nh.toFixed(1)} nH</span>
                        {issue.risk_score.inductance_nh > issue.risk_score.limit_nh && (
                          <span className="ml-2 text-red-600 dark:text-red-400 font-medium">⚠ Exceeds limit</span>
                        )}
                      </>
                    )}
                  </div>
                )}
                {issue.risk_score.details && (
                  <p className="text-xs text-gray-600 dark:text-gray-400 italic mt-2">{issue.risk_score.details}</p>
                )}
              </div>
            </section>
          )}

          {/* What's the problem */}
          <section>
            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-2 flex items-center gap-2">
              <AlertCircle className="w-4 h-4 text-blue-600 dark:text-blue-400" />
              What's the problem?
            </h3>
            <p className="text-gray-700 dark:text-gray-200">{issue.explanation.what}</p>
          </section>

          {/* Why does this matter */}
          <section className="bg-gray-100 dark:bg-gray-700/50 rounded-lg p-4">
            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
              <Zap className="w-4 h-4 text-yellow-600 dark:text-yellow-400" />
              Why does this matter?
            </h3>
            <p className="text-gray-700 dark:text-gray-200 mb-4">{issue.explanation.why.summary}</p>

            {issue.explanation.why.consequences.length > 0 && (
              <div className="space-y-2">
                <h4 className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase">
                  What could go wrong:
                </h4>
                <ul className="space-y-2">
                  {issue.explanation.why.consequences.map((c: Consequence, i: number) => (
                    <li
                      key={i}
                      className={`flex items-start gap-2 p-2 rounded ${getConsequenceSeverityColor(c.severity)}`}
                    >
                      <span className="flex-1 text-sm">{c.description}</span>
                      <span
                        className={`text-xs px-2 py-0.5 rounded text-white ${getLikelihoodBadge(c.likelihood)}`}
                      >
                        {c.likelihood}
                      </span>
                    </li>
                  ))}
                </ul>
              </div>
            )}

            {issue.explanation.why.failure_examples.length > 0 && (
              <div className="mt-4">
                <h4 className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                  Real-world examples:
                </h4>
                <ul className="list-disc ml-5 text-sm text-gray-700 dark:text-gray-300 space-y-1">
                  {issue.explanation.why.failure_examples.map((ex, i) => (
                    <li key={i}>{ex}</li>
                  ))}
                </ul>
              </div>
            )}
          </section>

          {/* How to fix */}
          <section>
            <h3 className="text-sm font-medium text-gray-700 dark:text-gray-300 mb-3 flex items-center gap-2">
              <Wrench className="w-4 h-4 text-gray-600 dark:text-gray-400" />
              How to fix it
            </h3>
            
            <ol className="space-y-3">
              {issue.explanation.how_to_fix.steps.map((step) => (
                <li key={step.step_number} className="flex gap-3">
                  <span className="flex-shrink-0 w-6 h-6 rounded-full bg-blue-600 text-white flex items-center justify-center text-sm font-medium">
                    {step.step_number}
                  </span>
                  <div>
                    <p className="font-medium text-gray-900 dark:text-white">{step.instruction}</p>
                    {step.details && (
                      <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">{step.details}</p>
                    )}
                  </div>
                </li>
              ))}
            </ol>

            {/* Component Suggestions */}
            {issue.explanation.how_to_fix.component_suggestions.length > 0 && (
              <div className="mt-4 p-3 bg-gray-100 dark:bg-gray-700/50 rounded-lg">
                <h4 className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                  Suggested components:
                </h4>
                {issue.explanation.how_to_fix.component_suggestions.map((comp, i) => (
                  <div key={i} className="text-sm">
                    <span className="font-mono text-blue-600 dark:text-blue-300">{comp.value}</span>{' '}
                    <span className="text-gray-700 dark:text-gray-300">{comp.component_type}</span>
                    <span className="text-gray-500 dark:text-gray-500"> ({comp.footprint})</span>
                    <p className="text-gray-600 dark:text-gray-400 text-xs mt-1">{comp.notes}</p>
                    {comp.example_part_numbers.length > 0 && (
                      <p className="text-gray-500 dark:text-gray-500 text-xs mt-1">
                        Examples: {comp.example_part_numbers.join(', ')}
                      </p>
                    )}
                  </div>
                ))}
              </div>
            )}

            {/* Pitfalls */}
            {issue.explanation.how_to_fix.pitfalls.length > 0 && (
              <div className="mt-4 p-3 bg-amber-50 dark:bg-amber-900/30 border border-amber-200 dark:border-amber-700 rounded-lg">
                <h4 className="text-sm font-medium text-amber-800 dark:text-amber-300 mb-2">
                  ⚠️ Common mistakes:
                </h4>
                <ul className="list-disc ml-5 text-sm text-amber-700 dark:text-amber-200 space-y-1">
                  {issue.explanation.how_to_fix.pitfalls.map((p, i) => (
                    <li key={i}>{p}</li>
                  ))}
                </ul>
              </div>
            )}

            {/* Verification */}
            <div className="mt-4 p-3 bg-green-50 dark:bg-green-900/30 border border-green-200 dark:border-green-700 rounded-lg">
              <h4 className="text-sm font-medium text-green-800 dark:text-green-300 mb-1">
                ✓ How to verify:
              </h4>
              <p className="text-sm text-green-700 dark:text-green-200">
                {issue.explanation.how_to_fix.verification}
              </p>
            </div>
          </section>

          {/* Technical Background (Collapsible) */}
          {issue.explanation.technical_background && (
            <section>
              <button
                onClick={() => setShowTechnical(!showTechnical)}
                className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition-colors w-full"
              >
                {showTechnical ? (
                  <ChevronDown className="w-4 h-4" />
                ) : (
                  <ChevronRight className="w-4 h-4" />
                )}
                <BookOpen className="w-4 h-4" />
                Technical Background
              </button>

              {showTechnical && (
                <div className="mt-3 p-4 bg-gray-100 dark:bg-gray-700/50 rounded-lg space-y-4">
                  <p className="font-medium text-blue-600 dark:text-blue-300">
                    {issue.explanation.technical_background.concept}
                  </p>
                  <pre className="text-sm text-gray-700 dark:text-gray-300 whitespace-pre-wrap font-mono bg-gray-50 dark:bg-gray-800 p-3 rounded">
                    {issue.explanation.technical_background.detailed_explanation}
                  </pre>

                  {issue.explanation.technical_background.equations.length > 0 && (
                    <div className="space-y-3">
                      {issue.explanation.technical_background.equations.map((eq, i) => (
                        <div key={i} className="p-3 bg-blue-50 dark:bg-blue-900/30 rounded">
                          <div className="text-sm font-medium text-blue-700 dark:text-blue-300">{eq.name}</div>
                          <div className="font-mono text-lg text-gray-900 dark:text-white my-2">{eq.formula}</div>
                          {eq.example_calculation && (
                            <div className="text-sm text-gray-600 dark:text-gray-400">
                              Example: {eq.example_calculation}
                            </div>
                          )}
                        </div>
                      ))}
                    </div>
                  )}

                  {issue.explanation.technical_background.related_concepts.length > 0 && (
                    <div>
                      <h5 className="text-xs font-medium text-gray-600 dark:text-gray-400 uppercase mb-2">
                        Related concepts:
                      </h5>
                      <div className="flex flex-wrap gap-2">
                        {issue.explanation.technical_background.related_concepts.map((c, i) => (
                          <span
                            key={i}
                            className="px-2 py-1 bg-gray-200 dark:bg-gray-600 rounded text-xs text-gray-700 dark:text-gray-300"
                          >
                            {c}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}
            </section>
          )}

          {/* References (Collapsible) */}
          {issue.explanation.references.length > 0 && (
            <section>
              <button
                onClick={() => setShowReferences(!showReferences)}
                className="flex items-center gap-2 text-sm font-medium text-gray-700 dark:text-gray-300 hover:text-gray-900 dark:hover:text-white transition-colors w-full"
              >
                {showReferences ? (
                  <ChevronDown className="w-4 h-4" />
                ) : (
                  <ChevronRight className="w-4 h-4" />
                )}
                <ExternalLink className="w-4 h-4" />
                Learn More ({issue.explanation.references.length} references)
              </button>

              {showReferences && (
                <ul className="mt-3 space-y-2">
                  {issue.explanation.references.map((ref, i) => (
                    <li key={i} className="flex items-center gap-2">
                      <span className="text-xs px-2 py-0.5 bg-gray-200 dark:bg-gray-600 rounded text-gray-700 dark:text-gray-300">
                        {ref.reference_type.replace('_', ' ')}
                      </span>
                      {ref.url ? (
                        <button
                          onClick={() => openUrl(ref.url!).catch(console.error)}
                          className="text-blue-600 dark:text-blue-400 hover:underline text-sm text-left flex items-center gap-1"
                        >
                          {ref.title}
                          <ExternalLink className="w-3 h-3 flex-shrink-0" />
                        </button>
                      ) : (
                        <span className="text-gray-700 dark:text-gray-300 text-sm">{ref.title}</span>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </section>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-between items-center p-4 border-t border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-900">
          <div className="flex gap-2">
            {issue.user_actions.dismiss_options.map((opt) => (
              <button
                key={opt.reason_code}
                onClick={() => onDismiss?.(opt.reason_code)}
                className="text-sm text-gray-600 dark:text-gray-400 hover:text-gray-900 dark:hover:text-white transition-colors"
              >
                {opt.label}
              </button>
            ))}
          </div>
          <button
            onClick={onClose}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors"
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}
