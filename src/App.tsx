import { useState, useEffect, useCallback } from 'react';
import { X, AlertCircle, CheckCircle } from 'lucide-react';
import { useStore } from './lib/store';
import { trackTabChanged } from './lib/analytics';
import { ProjectControls } from './components/ProjectControls';
import { ProjectInfo } from './components/ProjectInfo';
import { IssuePanel } from './components/IssuePanel';
import { DRSPanel } from './components/DRSPanel';
import { ComponentClassificationPanel } from './components/ComponentClassificationPanel';
import { PCBCompliancePanel } from './components/PCBCompliancePanel';
import { CircuitAnalysisPanel } from './components/CircuitAnalysisPanel';
import { ChatPanel } from './components/ChatPanel';
import { SettingsDialog } from './components/SettingsDialog';
import { AIAnalysisDialog } from './components/AIAnalysisPanel';
import { AnalyticsConsent } from './components/AnalyticsConsent';
import * as Tabs from '@radix-ui/react-tabs';

function App() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [aiAnalysisOpen, setAIAnalysisOpen] = useState(false);
  const [activeTab, setActiveTab] = useState('issues');
  
  const handleTabChange = (tab: string) => {
    setActiveTab(tab);
    trackTabChanged(tab);
  };
  const { settings, error, clearError, loadProjectHistory, aiAnalysis, toast, clearToast } = useStore();
  
  // Resizable sidebar state
  const [sidebarWidth, setSidebarWidth] = useState(() => {
    const saved = localStorage.getItem('sidebarWidth');
    return saved ? parseInt(saved, 10) : 320; // Default 320px (w-80)
  });
  const [isResizing, setIsResizing] = useState(false);
  
  // Resizable section heights
  const [projectInfoHeight, setProjectInfoHeight] = useState(() => {
    const saved = localStorage.getItem('projectInfoHeight');
    return saved ? parseInt(saved, 10) : 140; // Default 140px (reduced to minimize gap)
  });
  const [isResizingProjectInfo, setIsResizingProjectInfo] = useState(false);
  
  // Save sidebar width to localStorage
  useEffect(() => {
    localStorage.setItem('sidebarWidth', sidebarWidth.toString());
  }, [sidebarWidth]);
  
  // Handle mouse move for resizing
  const handleMouseMove = useCallback((e: MouseEvent) => {
    if (!isResizing) return;
    
    const newWidth = e.clientX;
    // Min width: 240px, Max width: 800px
    const clampedWidth = Math.max(240, Math.min(800, newWidth));
    setSidebarWidth(clampedWidth);
  }, [isResizing]);
  
  // Handle mouse up to stop resizing
  const handleMouseUp = useCallback(() => {
    setIsResizing(false);
  }, []);
  
  // Set up event listeners for resizing
  useEffect(() => {
    if (isResizing) {
      document.addEventListener('mousemove', handleMouseMove);
      document.addEventListener('mouseup', handleMouseUp);
      document.body.style.cursor = 'col-resize';
      document.body.style.userSelect = 'none';
      
      return () => {
        document.removeEventListener('mousemove', handleMouseMove);
        document.removeEventListener('mouseup', handleMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
    }
  }, [isResizing, handleMouseMove, handleMouseUp]);
  
  // Save project info height to localStorage
  useEffect(() => {
    localStorage.setItem('projectInfoHeight', projectInfoHeight.toString());
  }, [projectInfoHeight]);
  
  // Handle mouse move for resizing project info section
  const handleProjectInfoMouseMove = useCallback((e: MouseEvent) => {
    if (!isResizingProjectInfo) return;
    
    const sidebarElement = document.querySelector('aside');
    if (!sidebarElement) return;
    
    const sidebarRect = sidebarElement.getBoundingClientRect();
    const headerHeight = 64; // Approximate header + controls height
    const relativeY = e.clientY - sidebarRect.top - headerHeight;
    
    // Min height: 80px, Max height: 350px (reduced to minimize gap)
    const clampedHeight = Math.max(80, Math.min(350, relativeY));
    setProjectInfoHeight(clampedHeight);
  }, [isResizingProjectInfo]);
  
  // Handle mouse up to stop resizing project info
  const handleProjectInfoMouseUp = useCallback(() => {
    setIsResizingProjectInfo(false);
  }, []);
  
  // Set up event listeners for resizing project info
  useEffect(() => {
    if (isResizingProjectInfo) {
      document.addEventListener('mousemove', handleProjectInfoMouseMove);
      document.addEventListener('mouseup', handleProjectInfoMouseUp);
      document.body.style.cursor = 'row-resize';
      document.body.style.userSelect = 'none';
      
      return () => {
        document.removeEventListener('mousemove', handleProjectInfoMouseMove);
        document.removeEventListener('mouseup', handleProjectInfoMouseUp);
        document.body.style.cursor = '';
        document.body.style.userSelect = '';
      };
    }
  }, [isResizingProjectInfo, handleProjectInfoMouseMove, handleProjectInfoMouseUp]);

  // Open AI analysis dialog when analysis completes
  useEffect(() => {
    if (aiAnalysis) {
      setAIAnalysisOpen(true);
    }
  }, [aiAnalysis]);

  // Initialize theme on mount
  useEffect(() => {
    const applyTheme = () => {
      if (settings.theme === 'dark') {
        document.documentElement.classList.add('dark');
      } else if (settings.theme === 'light') {
        document.documentElement.classList.remove('dark');
      } else {
        // System preference
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        if (prefersDark) {
          document.documentElement.classList.add('dark');
        } else {
          document.documentElement.classList.remove('dark');
        }
      }
    };

    applyTheme();

    // Listen for system theme changes
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = () => {
      if (settings.theme === 'system') {
        applyTheme();
      }
    };
    
    mediaQuery.addEventListener('change', handleChange);
    return () => mediaQuery.removeEventListener('change', handleChange);
  }, [settings.theme]);

  // Load project history on mount
  useEffect(() => {
    loadProjectHistory();
  }, [loadProjectHistory]);

  return (
    <div className="flex h-screen bg-gray-100 dark:bg-gray-900 text-gray-900 dark:text-gray-100">
      {/* Left Sidebar */}
      <aside 
        className="flex flex-col bg-white dark:bg-gray-800 border-r border-gray-200 dark:border-gray-700 relative flex-shrink-0"
        style={{ width: `${sidebarWidth}px`, minWidth: '240px', maxWidth: '800px' }}
      >
        {/* Resize handle */}
        <div
          onMouseDown={(e) => {
            e.preventDefault();
            setIsResizing(true);
          }}
          className={`absolute right-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-blue-500 transition-colors z-10 ${
            isResizing ? 'bg-blue-500' : 'bg-transparent'
          }`}
        />
        {/* Header */}
        <header className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
          <div className="flex items-center gap-3">
            <img 
              src="/designguard_icon_macos_1024.svg" 
              alt="DesignGuard Logo" 
              className="w-10 h-10 rounded-lg object-contain"
            />
            <div>
              <h1 className="font-bold text-gray-900 dark:text-white">DesignGuard</h1>
              <p className="text-xs text-gray-500 dark:text-gray-400">Design Assistant</p>
            </div>
          </div>
        </header>

        {/* Project Controls */}
        <div className="p-4 border-b border-gray-200 dark:border-gray-700">
          <ProjectControls onOpenSettings={() => setSettingsOpen(true)} />
        </div>

        {/* Project Info - Resizable */}
        <div 
          className="border-b border-gray-200 dark:border-gray-700 relative overflow-hidden flex-shrink-0"
          style={{ height: `${projectInfoHeight}px`, minHeight: '80px', maxHeight: '350px' }}
        >
          <div className="p-4 h-full overflow-y-auto">
            <ProjectInfo />
          </div>
          {/* Resize handle for Project Info - Horizontal divider */}
          <div
            onMouseDown={(e) => {
              e.preventDefault();
              setIsResizingProjectInfo(true);
            }}
            className={`absolute bottom-0 left-0 right-0 h-1.5 cursor-row-resize hover:bg-blue-500 transition-colors z-10 ${
              isResizingProjectInfo ? 'bg-blue-500' : 'bg-transparent'
            }`}
            title="Drag to resize"
          />
        </div>

        {/* Analysis Panels - Takes remaining space */}
        <div className="flex-1 overflow-hidden flex flex-col min-h-0">
          <Tabs.Root value={activeTab} onValueChange={handleTabChange} className="flex flex-col h-full">
            <Tabs.List className="flex border-b border-gray-200 dark:border-gray-700">
              <Tabs.Trigger
                value="issues"
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 data-[state=active]:text-gray-900 data-[state=active]:dark:text-white data-[state=active]:border-b-2 data-[state=active]:border-blue-500 hover:text-gray-900 dark:hover:text-white transition-colors"
                title="Design Rule Check (DRC) issues: Missing components, incorrect values, design rule violations"
              >
                Issues
              </Tabs.Trigger>
              <Tabs.Trigger
                value="drs"
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 data-[state=active]:text-gray-900 data-[state=active]:dark:text-white data-[state=active]:border-b-2 data-[state=active]:border-purple-500 hover:text-gray-900 dark:hover:text-white transition-colors"
                title="Decoupling Risk Scoring: Analyzes IC decoupling capacitor placement and routing on PCB (requires PCB file)"
              >
                DRS
              </Tabs.Trigger>
              <Tabs.Trigger
                value="classification"
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 data-[state=active]:text-gray-900 data-[state=active]:dark:text-white data-[state=active]:border-b-2 data-[state=active]:border-green-500 hover:text-gray-900 dark:hover:text-white transition-colors"
                title="Component Roles: AI-powered classification of component functions (requires Ollama/Phi-3)"
              >
                Roles
              </Tabs.Trigger>
              <Tabs.Trigger
                value="compliance"
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 data-[state=active]:text-gray-900 data-[state=active]:dark:text-white data-[state=active]:border-b-2 data-[state=active]:border-orange-500 hover:text-gray-900 dark:hover:text-white transition-colors"
                title="PCB Compliance: IPC-2221 trace width, EMI analysis, and custom design rules (requires PCB file)"
              >
                Compliance
              </Tabs.Trigger>
              <Tabs.Trigger
                value="circuit"
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-600 dark:text-gray-400 data-[state=active]:text-gray-900 data-[state=active]:dark:text-white data-[state=active]:border-b-2 data-[state=active]:border-indigo-500 hover:text-gray-900 dark:hover:text-white transition-colors"
                title="Circuit Analysis: Decoupling, connectivity, and signal integrity analysis using graph-based circuit representation"
              >
                Circuit
              </Tabs.Trigger>
            </Tabs.List>
            {activeTab === 'issues' && (
              <Tabs.Content 
                value="issues" 
                className="flex-1 overflow-hidden flex flex-col min-h-0"
              >
                <IssuePanel />
              </Tabs.Content>
            )}
            {activeTab === 'drs' && (
              <Tabs.Content 
                value="drs" 
                className="flex-1 overflow-hidden flex flex-col min-h-0"
              >
                <DRSPanel />
              </Tabs.Content>
            )}
            {activeTab === 'classification' && (
              <Tabs.Content 
                value="classification" 
                className="flex-1 overflow-hidden flex flex-col min-h-0"
              >
                <ComponentClassificationPanel />
              </Tabs.Content>
            )}
            {activeTab === 'compliance' && (
              <Tabs.Content 
                value="compliance" 
                className="flex-1 overflow-hidden flex flex-col min-h-0"
              >
                <PCBCompliancePanel />
              </Tabs.Content>
            )}
            {activeTab === 'circuit' && (
              <Tabs.Content 
                value="circuit" 
                className="flex-1 overflow-hidden flex flex-col min-h-0"
              >
                <CircuitAnalysisPanel />
              </Tabs.Content>
            )}
          </Tabs.Root>
        </div>
      </aside>

      {/* Main Chat Area */}
      <main className="flex-1 flex flex-col overflow-hidden">
        <ChatPanel onOpenAIAnalysis={() => setAIAnalysisOpen(true)} />
      </main>

      {/* Settings Dialog */}
      <SettingsDialog open={settingsOpen} onOpenChange={setSettingsOpen} />

      {/* AI Analysis Dialog */}
      <AIAnalysisDialog open={aiAnalysisOpen} onOpenChange={setAIAnalysisOpen} />

      {/* Error Toast */}
      {error && (
        <div className="fixed bottom-4 right-4 z-50 max-w-2xl animate-in slide-in-from-bottom-2 fade-in duration-300">
          <div className="flex items-start gap-3 p-4 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 rounded-lg shadow-lg max-h-[80vh] overflow-y-auto">
            <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
            <div className="flex-1 min-w-0">
              <p className="text-sm font-medium text-red-800 dark:text-red-200">Error</p>
              <p className="text-sm text-red-600 dark:text-red-300 mt-1 whitespace-pre-wrap break-words">{error}</p>
            </div>
            <button
              onClick={clearError}
              className="flex-shrink-0 p-1 text-red-400 hover:text-red-600 dark:hover:text-red-300 transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}

      {/* Analytics Consent Banner */}
      <AnalyticsConsent />

      {/* Success Toast */}
      {toast && (
        <div className="fixed bottom-4 right-4 z-50 max-w-md animate-in slide-in-from-bottom-2 fade-in duration-300">
          <div className={`flex items-start gap-3 p-4 border rounded-lg shadow-lg ${
            toast.type === 'success'
              ? 'bg-green-50 dark:bg-green-900/30 border-green-200 dark:border-green-800'
              : 'bg-red-50 dark:bg-red-900/30 border-red-200 dark:border-red-800'
          }`}>
            {toast.type === 'success' ? (
              <CheckCircle className="w-5 h-5 text-green-500 flex-shrink-0 mt-0.5" />
            ) : (
              <AlertCircle className="w-5 h-5 text-red-500 flex-shrink-0 mt-0.5" />
            )}
            <div className="flex-1 min-w-0">
              <p className={`text-sm font-medium ${
                toast.type === 'success'
                  ? 'text-green-800 dark:text-green-200'
                  : 'text-red-800 dark:text-red-200'
              }`}>
                {toast.type === 'success' ? 'Success' : 'Error'}
              </p>
              <p className={`text-sm mt-1 ${
                toast.type === 'success'
                  ? 'text-green-600 dark:text-green-300'
                  : 'text-red-600 dark:text-red-300'
              }`}>
                {toast.message}
              </p>
            </div>
            <button
              onClick={clearToast}
              className={`flex-shrink-0 p-1 transition-colors ${
                toast.type === 'success'
                  ? 'text-green-400 hover:text-green-600 dark:hover:text-green-300'
                  : 'text-red-400 hover:text-red-600 dark:hover:text-red-300'
              }`}
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
