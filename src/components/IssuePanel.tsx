import { RefreshCw, Sparkles, AlertCircle, AlertTriangle, Info, Lightbulb, CheckCircle, FileText, Shield } from 'lucide-react';
import { useStore } from '../lib/store';
import { api } from '../lib/api';
import { IssueItem } from './IssueItem';
import { IssueDetail } from './IssueDetail';
import { useState } from 'react';
import type { Severity, DetailedIssue, Issue } from '../types';

export function IssuePanel() {
  const { 
    project, 
    issues, 
    isAnalyzing, 
    isAIAnalyzing, 
    runAnalysis, 
    runAIAnalysis,
    settings 
  } = useStore();
  
  const [selectedIssue, setSelectedIssue] = useState<DetailedIssue | null>(null);
  const [loadingDetails, setLoadingDetails] = useState(false);
  const [runningFullAnalysis, setRunningFullAnalysis] = useState(false);
  const [runningDatasheetCheck, setRunningDatasheetCheck] = useState(false);
  const [fullAnalysisResults, setFullAnalysisResults] = useState<DetailedIssue[]>([]);

  const hasIssues = issues.length > 0 || fullAnalysisResults.length > 0;
  const displayIssues = fullAnalysisResults.length > 0 ? fullAnalysisResults.map(di => ({
    id: di.id,
    rule_id: di.rule_id,
    severity: di.severity,
    message: di.title,
    component: di.components[0] || null,
    location: di.location,
    suggestion: di.explanation.how_to_fix.steps[0]?.instruction || null,
    risk_score: di.risk_score,
  })) : issues;

  // Count issues by severity
  const counts: Record<Severity, number> = {
    Error: 0,
    Warning: 0,
    Info: 0,
    Suggestion: 0,
  };
  
  displayIssues.forEach(issue => {
    counts[issue.severity]++;
  });

  const handleIssueClick = async (issue: Issue) => {
    setLoadingDetails(true);
    try {
      const detailed = await api.getIssueDetails(issue);
      setSelectedIssue(detailed);
    } catch (e) {
      console.error('Failed to load issue details:', e);
      // Fallback: create a basic DetailedIssue from the regular Issue
      setSelectedIssue({
        id: issue.id,
        severity: issue.severity,
        rule_id: issue.rule_id,
        title: issue.message,
        summary: issue.message,
        components: issue.component ? [issue.component] : [],
        location: issue.location,
        explanation: {
          what: issue.message,
          why: {
            summary: issue.suggestion || 'This issue may affect circuit functionality.',
            consequences: [],
            failure_examples: [],
          },
          technical_background: null,
          how_to_fix: {
            steps: issue.suggestion ? [{
              step_number: 1,
              instruction: issue.suggestion,
              details: null,
              image: null,
            }] : [],
            component_suggestions: [],
            pitfalls: [],
            verification: 'Re-run analysis to verify the fix.',
          },
          diagrams: [],
          references: [],
        },
        user_actions: {
          auto_fix_available: false,
          can_dismiss: true,
          dismiss_options: [],
        },
        risk_score: issue.risk_score,
      });
    } finally {
      setLoadingDetails(false);
    }
  };

  const handleRunFullAnalysis = async () => {
    if (!project) return;
    
    setRunningFullAnalysis(true);
    try {
      const results = await api.runFullAnalysis();
      setFullAnalysisResults(results);
    } catch (e) {
      console.error('Failed to run full analysis:', e);
    } finally {
      setRunningFullAnalysis(false);
    }
  };

  const handleRunDatasheetCheck = async () => {
    if (!project) return;
    
    setRunningDatasheetCheck(true);
    try {
      const results = await api.runDatasheetCheck();
      // Update store with new issues
      useStore.setState({ issues: results });
      setFullAnalysisResults([]);
    } catch (e) {
      console.error('Failed to run datasheet check:', e);
    } finally {
      setRunningDatasheetCheck(false);
    }
  };

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <h2 className="font-semibold text-gray-900 dark:text-white">Issues</h2>
        
        <div className="flex items-center gap-2">
          <button
            onClick={handleRunDatasheetCheck}
            disabled={!project || runningDatasheetCheck}
            className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            title="Run datasheet check"
          >
            <FileText className={`w-4 h-4 ${runningDatasheetCheck ? 'animate-pulse' : ''}`} />
          </button>
          <button
            onClick={handleRunFullAnalysis}
            disabled={!project || runningFullAnalysis}
            className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            title="Run full analysis (DRC + Datasheet)"
          >
            <Shield className={`w-4 h-4 ${runningFullAnalysis ? 'animate-pulse' : ''}`} />
          </button>
          <button
            onClick={() => runAnalysis()}
            disabled={!project || isAnalyzing}
            className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            title="Re-run DRC analysis"
          >
            <RefreshCw className={`w-4 h-4 ${isAnalyzing ? 'animate-spin' : ''}`} />
          </button>
        </div>
      </div>

      {/* Issue counts */}
      {hasIssues && (
        <div className="flex items-center gap-3 px-4 py-2 border-b border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50">
          {counts.Error > 0 && (
            <span className="flex items-center gap-1 text-xs text-red-600 dark:text-red-400">
              <AlertCircle className="w-3 h-3" />
              {counts.Error}
            </span>
          )}
          {counts.Warning > 0 && (
            <span className="flex items-center gap-1 text-xs text-yellow-600 dark:text-yellow-400">
              <AlertTriangle className="w-3 h-3" />
              {counts.Warning}
            </span>
          )}
          {counts.Info > 0 && (
            <span className="flex items-center gap-1 text-xs text-blue-600 dark:text-blue-400">
              <Info className="w-3 h-3" />
              {counts.Info}
            </span>
          )}
          {counts.Suggestion > 0 && (
            <span className="flex items-center gap-1 text-xs text-green-600 dark:text-green-400">
              <Lightbulb className="w-3 h-3" />
              {counts.Suggestion}
            </span>
          )}
        </div>
      )}

      {/* Issue list */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {!project ? (
          <div className="text-center text-gray-500 dark:text-gray-400 py-8">
            <p className="text-sm">Open a project to see issues</p>
          </div>
        ) : isAnalyzing ? (
          <div className="text-center text-gray-500 dark:text-gray-400 py-8">
            <RefreshCw className="w-6 h-6 mx-auto mb-2 animate-spin" />
            <p className="text-sm">Analyzing design...</p>
          </div>
        ) : !hasIssues ? (
          <div className="text-center text-green-600 dark:text-green-400 py-8">
            <CheckCircle className="w-8 h-8 mx-auto mb-2" />
            <p className="text-sm font-medium">No issues found</p>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Your design looks good!
            </p>
          </div>
        ) : (
          displayIssues.map(issue => (
            <div key={issue.id} onClick={() => handleIssueClick(issue)} className="cursor-pointer">
              <IssueItem issue={issue} />
            </div>
          ))
        )}
        
        {loadingDetails && (
          <div className="text-center text-gray-500 dark:text-gray-400 py-4">
            <RefreshCw className="w-5 h-5 mx-auto mb-2 animate-spin" />
            <p className="text-sm">Loading issue details...</p>
          </div>
        )}
      </div>

      {/* AI Analysis button - Sticky to bottom */}
      <div className="px-4 py-2 border-t border-gray-200 dark:border-gray-700 flex-shrink-0 h-[96px]">
        <button
          onClick={() => runAIAnalysis()}
          disabled={!project || isAIAnalyzing}
          className="w-full flex items-center justify-center gap-2 px-4 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-all dark:focus:ring-offset-gray-800"
        >
          <Sparkles className={`w-4 h-4 ${isAIAnalyzing ? 'animate-pulse' : ''}`} />
          {isAIAnalyzing ? 'Analyzing with AI...' : 'AI Deep Analysis'}
        </button>
        
        {!settings.apiKeyConfigured && (
          <p className="text-xs text-center text-gray-500 dark:text-gray-400 mt-1">
            AI can use either a Claude API key or the local Ollama provider. Configure these in Settings.
          </p>
        )}
      </div>
      
      {/* Issue Detail Dialog */}
      {selectedIssue && (
        <IssueDetail
          issue={selectedIssue}
          onClose={() => setSelectedIssue(null)}
        />
      )}
    </div>
  );
}
