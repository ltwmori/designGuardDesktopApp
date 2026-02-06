import { useState } from 'react';
import { ChevronDown, ChevronRight, Sparkles, AlertTriangle, Lightbulb, Cpu, X } from 'lucide-react';
import * as Dialog from '@radix-ui/react-dialog';
import { useStore } from '../lib/store';

export function AIAnalysisPanel() {
  const { aiAnalysis } = useStore();
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set(['summary', 'issues', 'suggestions'])
  );

  if (!aiAnalysis) return null;

  const toggleSection = (section: string) => {
    const newExpanded = new Set(expandedSections);
    if (newExpanded.has(section)) {
      newExpanded.delete(section);
    } else {
      newExpanded.add(section);
    }
    setExpandedSections(newExpanded);
  };

  return (
    <div className="space-y-4">
      {/* Summary Section */}
      <Section
        title="Summary"
        icon={<Sparkles className="w-4 h-4 text-purple-500" />}
        expanded={expandedSections.has('summary')}
        onToggle={() => toggleSection('summary')}
      >
        <p className="text-sm text-gray-600 dark:text-gray-300">
          {aiAnalysis.summary}
        </p>
      </Section>

      {/* Circuit Description */}
      <Section
        title="Circuit Description"
        icon={<Cpu className="w-4 h-4 text-blue-500" />}
        expanded={expandedSections.has('description')}
        onToggle={() => toggleSection('description')}
      >
        <p className="text-sm text-gray-600 dark:text-gray-300">
          {aiAnalysis.circuit_description}
        </p>
      </Section>

      {/* Potential Issues */}
      {aiAnalysis.potential_issues.length > 0 && (
        <Section
          title={`Potential Issues (${aiAnalysis.potential_issues.length})`}
          icon={<AlertTriangle className="w-4 h-4 text-yellow-500" />}
          expanded={expandedSections.has('issues')}
          onToggle={() => toggleSection('issues')}
        >
          <ul className="space-y-2">
            {aiAnalysis.potential_issues.map((issue, index) => (
              <li 
                key={index}
                className="flex items-start gap-2 text-sm text-gray-600 dark:text-gray-300"
              >
                <span className="w-5 h-5 flex-shrink-0 rounded-full bg-yellow-100 dark:bg-yellow-900/30 text-yellow-600 dark:text-yellow-400 flex items-center justify-center text-xs font-medium">
                  {index + 1}
                </span>
                {issue}
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Improvement Suggestions */}
      {aiAnalysis.improvement_suggestions.length > 0 && (
        <Section
          title={`Suggestions (${aiAnalysis.improvement_suggestions.length})`}
          icon={<Lightbulb className="w-4 h-4 text-green-500" />}
          expanded={expandedSections.has('suggestions')}
          onToggle={() => toggleSection('suggestions')}
        >
          <ul className="space-y-2">
            {aiAnalysis.improvement_suggestions.map((suggestion, index) => (
              <li 
                key={index}
                className="flex items-start gap-2 text-sm text-gray-600 dark:text-gray-300"
              >
                <span className="w-5 h-5 flex-shrink-0 rounded-full bg-green-100 dark:bg-green-900/30 text-green-600 dark:text-green-400 flex items-center justify-center text-xs font-medium">
                  {index + 1}
                </span>
                {suggestion}
              </li>
            ))}
          </ul>
        </Section>
      )}

      {/* Component Recommendations */}
      {aiAnalysis.component_recommendations.length > 0 && (
        <Section
          title={`Component Recommendations (${aiAnalysis.component_recommendations.length})`}
          icon={<Cpu className="w-4 h-4 text-indigo-500" />}
          expanded={expandedSections.has('components')}
          onToggle={() => toggleSection('components')}
        >
          <div className="space-y-3">
            {aiAnalysis.component_recommendations.map((rec, index) => (
              <div 
                key={index}
                className="p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg"
              >
                <div className="flex items-center justify-between mb-1">
                  <span className="font-medium text-sm text-gray-900 dark:text-white">
                    {rec.component}
                  </span>
                  <span className="text-xs text-gray-500 dark:text-gray-400">
                    {rec.current_value}
                    {rec.suggested_value && (
                      <>
                        {' → '}
                        <span className="text-green-600 dark:text-green-400 font-medium">
                          {rec.suggested_value}
                        </span>
                      </>
                    )}
                  </span>
                </div>
                <p className="text-xs text-gray-600 dark:text-gray-300">
                  {rec.reason}
                </p>
              </div>
            ))}
          </div>
        </Section>
      )}
    </div>
  );
}

interface SectionProps {
  title: string;
  icon: React.ReactNode;
  expanded: boolean;
  onToggle: () => void;
  children: React.ReactNode;
}

function Section({ title, icon, expanded, onToggle, children }: SectionProps) {
  return (
    <div className="border border-gray-200 dark:border-gray-700 rounded-lg overflow-hidden">
      <button
        onClick={onToggle}
        className="w-full flex items-center gap-3 p-3 hover:bg-gray-50 dark:hover:bg-gray-700/50 transition-colors"
      >
        {expanded ? (
          <ChevronDown className="w-4 h-4 text-gray-400" />
        ) : (
          <ChevronRight className="w-4 h-4 text-gray-400" />
        )}
        {icon}
        <span className="font-medium text-sm text-gray-900 dark:text-white">
          {title}
        </span>
      </button>
      
      {expanded && (
        <div className="px-4 pb-4 pt-1">
          {children}
        </div>
      )}
    </div>
  );
}

// Dialog version for displaying in a modal
interface AIAnalysisDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function AIAnalysisDialog({ open, onOpenChange }: AIAnalysisDialogProps) {
  const { aiAnalysis } = useStore();

  if (!aiAnalysis) return null;

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50" />
        <Dialog.Content className="fixed left-[50%] top-[50%] z-50 w-full max-w-2xl max-h-[85vh] translate-x-[-50%] translate-y-[-50%] bg-white dark:bg-gray-800 rounded-xl shadow-xl overflow-hidden">
          <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
            <div className="flex items-center gap-2">
              <Sparkles className="w-5 h-5 text-purple-500" />
              <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-white">
                AI Analysis Results
              </Dialog.Title>
            </div>
            <Dialog.Close className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors">
              <X className="w-5 h-5" />
            </Dialog.Close>
          </div>
          
          <div className="p-4 overflow-y-auto max-h-[calc(85vh-80px)]">
            <AIAnalysisPanel />
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
