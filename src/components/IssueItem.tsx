import { useState } from 'react';
import { ChevronDown, ChevronRight, AlertCircle, AlertTriangle, Info, Lightbulb, MapPin, Cpu } from 'lucide-react';
import type { Issue, Severity } from '../types';

interface IssueItemProps {
  issue: Issue;
}

const severityConfig: Record<Severity, { 
  icon: typeof AlertCircle; 
  color: string; 
  bgColor: string;
  label: string;
}> = {
  Error: { 
    icon: AlertCircle, 
    color: 'text-red-500', 
    bgColor: 'bg-red-50 dark:bg-red-900/20',
    label: 'Error'
  },
  Warning: { 
    icon: AlertTriangle, 
    color: 'text-yellow-500', 
    bgColor: 'bg-yellow-50 dark:bg-yellow-900/20',
    label: 'Warning'
  },
  Info: { 
    icon: Info, 
    color: 'text-blue-500', 
    bgColor: 'bg-blue-50 dark:bg-blue-900/20',
    label: 'Info'
  },
  Suggestion: { 
    icon: Lightbulb, 
    color: 'text-green-500', 
    bgColor: 'bg-green-50 dark:bg-green-900/20',
    label: 'Suggestion'
  },
};

export function IssueItem({ issue }: IssueItemProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const config = severityConfig[issue.severity];
  const Icon = config.icon;

  return (
    <div className={`rounded-lg border border-gray-200 dark:border-gray-700 overflow-hidden ${config.bgColor}`}>
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className="w-full flex items-start gap-3 p-3 text-left hover:bg-black/5 dark:hover:bg-white/5 transition-colors"
      >
        <span className="flex-shrink-0 mt-0.5">
          {isExpanded ? (
            <ChevronDown className="w-4 h-4 text-gray-400" />
          ) : (
            <ChevronRight className="w-4 h-4 text-gray-400" />
          )}
        </span>
        
        <Icon className={`w-5 h-5 flex-shrink-0 ${config.color}`} />
        
        <div className="flex-1 min-w-0">
          <p className="text-sm text-gray-900 dark:text-white">
            {issue.message}
          </p>
          
          {issue.component && (
            <span className="inline-flex items-center gap-1 mt-1 text-xs text-gray-500 dark:text-gray-400">
              <Cpu className="w-3 h-3" />
              {issue.component}
            </span>
          )}
        </div>
      </button>

      {isExpanded && (
        <div className="px-3 pb-3 pt-1 ml-12 space-y-2 border-t border-gray-200/50 dark:border-gray-700/50">
          {/* Suggestion */}
          {issue.suggestion && (
            <div className="flex items-start gap-2">
              <Lightbulb className="w-4 h-4 text-green-500 flex-shrink-0 mt-0.5" />
              <p className="text-sm text-gray-600 dark:text-gray-300">
                {issue.suggestion}
              </p>
            </div>
          )}

          {/* Location */}
          {issue.location && (
            <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
              <MapPin className="w-3 h-3" />
              <span>
                Position: ({issue.location.x.toFixed(1)}, {issue.location.y.toFixed(1)})
              </span>
            </div>
          )}

          {/* Risk Score */}
          {issue.risk_score && (
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-xs font-medium text-gray-700 dark:text-gray-300">
                <span>Risk Score:</span>
                <span className={`px-2 py-0.5 rounded ${
                  issue.risk_score.value >= 70 
                    ? 'bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400'
                    : issue.risk_score.value >= 40
                    ? 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400'
                    : 'bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400'
                }`}>
                  {issue.risk_score.value.toFixed(0)}
                </span>
              </div>
              {issue.risk_score.inductance_nh !== undefined && issue.risk_score.limit_nh !== undefined && (
                <div className="text-xs text-gray-600 dark:text-gray-400">
                  Inductance: <span className="font-mono">{issue.risk_score.inductance_nh.toFixed(1)} nH</span>
                  {issue.risk_score.limit_nh && (
                    <>
                      {' '}(limit: <span className="font-mono">{issue.risk_score.limit_nh.toFixed(1)} nH</span>)
                      {issue.risk_score.inductance_nh > issue.risk_score.limit_nh && (
                        <span className="ml-1 text-red-600 dark:text-red-400 font-medium">⚠ Exceeds limit</span>
                      )}
                    </>
                  )}
                </div>
              )}
              {issue.risk_score.details && (
                <div className="text-xs text-gray-500 dark:text-gray-400 italic">
                  {issue.risk_score.details}
                </div>
              )}
            </div>
          )}

          {/* Rule ID */}
          <div className="text-xs text-gray-400 dark:text-gray-500">
            Rule: {issue.rule_id}
          </div>
        </div>
      )}
    </div>
  );
}
