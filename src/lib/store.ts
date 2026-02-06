import { create } from "zustand";
import { persist } from "zustand/middleware";
import { api, generateId, isTauri } from "./api";
import {
  trackProjectOpened,
  trackProjectClosed,
  trackValidationStarted,
  trackValidationCompleted,
  trackAIAnalysisStarted,
  trackAIAnalysisCompleted,
  trackAIAnalysisFailed,
  trackChatMessageSent,
  trackError,
} from "./analytics";
import type { 
  ProjectInfo, 
  Schematic, 
  Issue, 
  AIAnalysis, 
  ChatMessage, 
  Settings 
} from "../types";

interface AppState {
  // Project state
  project: ProjectInfo | null;
  schematic: Schematic | null;
  schematicPath: string | null;
  
  // Analysis state
  issues: Issue[];
  aiAnalysis: AIAnalysis | null;
  
  // Chat state
  messages: ChatMessage[];
  
  // UI state
  isLoading: boolean;
  isAnalyzing: boolean;
  isAIAnalyzing: boolean;
  isChatLoading: boolean;
  error: string | null;
  toast: { message: string; type: 'success' | 'error' } | null;
  
  // Settings
  settings: Settings;
  apiKey: string;
  
  // Project history
  projectHistory: ProjectInfo[];
  
  // Actions
  openProject: (schematicPath: string) => Promise<void>;
  closeProject: () => void;
  runAnalysis: () => Promise<void>;
  runAIAnalysis: () => Promise<void>;
  sendMessage: (message: string) => Promise<void>;
  setApiKey: (key: string) => void;
  setTheme: (theme: Settings['theme']) => void;
  clearMessages: () => void;
  setError: (error: string | null) => void;
  clearError: () => void;
  showToast: (message: string, type?: 'success' | 'error') => void;
  clearToast: () => void;
  loadProjectHistory: () => Promise<void>;
}

// API key storage key
const API_KEY_STORAGE = 'kicad-ai-api-key';

// Load API key from localStorage
const loadApiKey = (): string => {
  if (typeof window !== 'undefined') {
    return localStorage.getItem(API_KEY_STORAGE) || '';
  }
  return '';
};

// Save API key to localStorage
const saveApiKey = (key: string): void => {
  if (typeof window !== 'undefined') {
    if (key) {
      localStorage.setItem(API_KEY_STORAGE, key);
    } else {
      localStorage.removeItem(API_KEY_STORAGE);
    }
  }
};

export const useStore = create<AppState>()(
  persist(
    (set, get) => ({
      // Initial state
      project: null,
      schematic: null,
      schematicPath: null,
      issues: [],
      aiAnalysis: null,
      messages: [],
      isLoading: false,
      isAnalyzing: false,
      isAIAnalyzing: false,
      isChatLoading: false,
      error: null,
      toast: null,
      settings: {
        theme: 'system',
        apiKeyConfigured: !!loadApiKey(),
      },
      apiKey: loadApiKey(),
      projectHistory: [],

      // Open a project (file or directory)
      openProject: async (path: string) => {
        set({ isLoading: true, error: null });
        
        try {
          // Use backend command to handle file/directory discovery and parsing
          // This handles both files and directories, and finds matching schematic/PCB pairs
          const projectInfo = await api.openProject(path);
          
          // Get the already-parsed schematic from backend state (avoids duplicate parsing)
          const schematic = await api.getCurrentSchematic();

          // Watch the project for changes
          try {
            await api.watchProject(projectInfo.path);
          } catch (e) {
            console.warn('Could not start file watcher:', e);
          }

          set({ 
            project: projectInfo, 
            schematic, 
            schematicPath: projectInfo.path,
            isLoading: false,
            issues: [],
            aiAnalysis: null,
            messages: [],
          });

          // Analytics: project opened
          trackProjectOpened({
            component_count: schematic?.components?.length ?? 0,
            net_count: schematic?.nets?.length ?? 0,
          });
          
          // Auto-run DRC analysis
          get().runAnalysis();
        } catch (error) {
          const msg = error instanceof Error ? error.message : "Failed to open project";
          trackError('open_project', msg);
          set({ 
            error: msg,
            isLoading: false 
          });
        }
      },

      // Close current project
      closeProject: () => {
        const { schematicPath } = get();
        
        if (schematicPath) {
          api.stopWatching(schematicPath).catch(console.error);
        }

        trackProjectClosed();
        
        set({
          project: null,
          schematic: null,
          schematicPath: null,
          issues: [],
          aiAnalysis: null,
          messages: [],
          error: null,
        });
      },

      // Run DRC analysis
      runAnalysis: async () => {
        const { schematicPath } = get();
        
        if (!schematicPath) {
          set({ error: "No schematic loaded" });
          return;
        }

        set({ isAnalyzing: true, error: null });
        const startTime = performance.now();
        const checkTypes = ['drc'];

        trackValidationStarted(checkTypes);
        
        try {
          const issues = await api.runDRC(schematicPath);
          const duration = Math.round(performance.now() - startTime);

          trackValidationCompleted({
            issues_found: issues.length,
            duration_ms: duration,
            check_types: checkTypes,
          });
          
          set((state) => ({ 
            issues,
            isAnalyzing: false,
            project: state.project ? {
              ...state.project,
              last_analyzed: new Date().toISOString(),
            } : null,
          }));
        } catch (error) {
          const msg = error instanceof Error ? error.message : "Analysis failed";
          trackError('run_analysis', msg);
          set({ 
            error: msg,
            isAnalyzing: false 
          });
        }
      },

      // Run AI analysis (uses router: Claude or Ollama)
      runAIAnalysis: async () => {
        const { schematicPath, settings } = get();
        
        if (!schematicPath) {
          set({ error: "No schematic loaded" });
          return;
        }

        set({ isAIAnalyzing: true, error: null });
        const startTime = performance.now();
        const provider = settings.ai_provider || 'claude';

        trackAIAnalysisStarted(provider);
        
        try {
          // Use backend router to pick the best available provider (Claude or Ollama)
          const aiAnalysis = await api.aiAnalyze();
          const duration = Math.round(performance.now() - startTime);

          trackAIAnalysisCompleted({
            provider,
            issues_found: aiAnalysis.potential_issues?.length ?? 0,
            suggestions_count: aiAnalysis.improvement_suggestions?.length ?? 0,
            duration_ms: duration,
          });

          set({ aiAnalysis, isAIAnalyzing: false });
        } catch (error) {
          const msg = error instanceof Error ? error.message : (String(error) || "AI analysis failed");
          trackAIAnalysisFailed({ provider, error_type: 'api_error' });
          set({ 
            error: msg,
            isAIAnalyzing: false 
          });
        }
      },

      // Send a chat message (uses router: Claude or Ollama)
      sendMessage: async (content: string) => {
        const { schematicPath, messages } = get();
        
        if (!schematicPath) {
          set({ error: "No schematic loaded" });
          return;
        }

        // Add user message
        const userMessage: ChatMessage = {
          id: generateId(),
          role: 'user',
          content,
          timestamp: new Date(),
        };

        trackChatMessageSent();

        set({ 
          messages: [...messages, userMessage],
          isChatLoading: true,
          error: null,
        });

        try {
          // Use backend router so chat works with either Claude (API key) or Ollama
          const response = await api.askAI(content);
          
          // Add assistant message
          const assistantMessage: ChatMessage = {
            id: generateId(),
            role: 'assistant',
            content: response,
            timestamp: new Date(),
          };

          set((state) => ({ 
            messages: [...state.messages, assistantMessage],
            isChatLoading: false,
          }));
        } catch (error) {
          set({ 
            error: error instanceof Error ? error.message : (String(error) || "Failed to get AI response"),
            isChatLoading: false,
          });
        }
      },

      // Set API key
      setApiKey: (key: string) => {
        saveApiKey(key);
        set((state) => ({
          apiKey: key,
          settings: {
            ...state.settings,
            apiKeyConfigured: !!key,
          },
        }));
      },

      // Set theme
      setTheme: (theme: Settings['theme']) => {
        set((state) => ({
          settings: {
            ...state.settings,
            theme,
          },
        }));
      },

      // Clear chat messages
      clearMessages: () => {
        set({ messages: [] });
      },

      // Set error
      setError: (error: string | null) => {
        set({ error });
      },

      // Clear error
      clearError: () => {
        set({ error: null });
      },

      showToast: (message: string, type: 'success' | 'error' = 'success') => {
        set({ toast: { message, type } });
        // Auto-clear after 3 seconds
        setTimeout(() => {
          get().clearToast();
        }, 3000);
      },

      clearToast: () => {
        set({ toast: null });
      },

      // Load project history
      loadProjectHistory: async () => {
        // Skip if not running in Tauri
        if (!isTauri()) {
          console.warn('Not running in Tauri, skipping project history load');
          return;
        }
        
        try {
          const history = await api.getProjectHistory();
          set({ projectHistory: history });
        } catch (error) {
          console.error('Failed to load project history:', error);
        }
      },
    }),
    {
      name: 'kicad-ai-store',
      partialize: (state) => ({
        settings: state.settings,
        // Don't persist project state, only settings
      }),
    }
  )
);
