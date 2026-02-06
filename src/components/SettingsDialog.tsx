import { useState, useEffect } from 'react';
import * as Dialog from '@radix-ui/react-dialog';
import { X, Eye, EyeOff, Check, Moon, Sun, Monitor, Cpu, Cloud, RefreshCw, Upload, Trash2, FileJson, Loader2, AlertCircle, BarChart3 } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { useStore } from '../lib/store';
import { api } from '../lib/api';
import { isTauri } from '../lib/api';
import { getConsentStatus, setConsentStatus, trackSettingsChanged } from '../lib/analytics';
import type { Settings, ProviderStatus, UserDatasheetInfo } from '../types';

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SettingsDialog({ open, onOpenChange }: SettingsDialogProps) {
  const { apiKey, settings, setApiKey, setTheme } = useStore();
  
  const [localApiKey, setLocalApiKey] = useState('');
  const [showApiKey, setShowApiKey] = useState(false);
  const [saved, setSaved] = useState(false);
  
  // Ollama state
  const [aiProvider, setAiProvider] = useState<'claude' | 'ollama'>(settings.ai_provider === 'ollama' ? 'ollama' : 'claude');
  const [ollamaUrl, setOllamaUrl] = useState(settings.ollama_url || 'http://localhost:11434');
  const [ollamaModel, setOllamaModel] = useState(settings.ollama_model || 'llama3.1:8b');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [aiStatus, setAiStatus] = useState<ProviderStatus | null>(null);
  const [loadingModels, setLoadingModels] = useState(false);
  const [savingAI, setSavingAI] = useState(false);
  const [aiError, setAiError] = useState<string | null>(null);
  const [aiSuccess, setAiSuccess] = useState<string | null>(null);
  
  // Datasheet state
  const [userDatasheets, setUserDatasheets] = useState<UserDatasheetInfo[]>([]);
  const [loadingDatasheets, setLoadingDatasheets] = useState(false);
  const [uploadingDatasheet, setUploadingDatasheet] = useState(false);
  const [datasheetError, setDatasheetError] = useState<string | null>(null);
  const [datasheetSuccess, setDatasheetSuccess] = useState<string | null>(null);
  
  // Analytics state
  const [analyticsEnabled, setAnalyticsEnabled] = useState(false);

  // Initialize local state when dialog opens
  useEffect(() => {
    if (open) {
      setLocalApiKey(apiKey);
      setSaved(false);
      loadAIStatus();
      loadUserDatasheets();
      setAnalyticsEnabled(getConsentStatus() === true);
    }
  }, [open, apiKey]);
  
  const loadAIStatus = async () => {
    try {
      const status = await api.getAIStatus();
      setAiStatus(status);
      setAiProvider(status.preferred as 'claude' | 'ollama');
      if (status.ollama_models.length > 0) {
        setAvailableModels(status.ollama_models);
      }
    } catch (e) {
      console.error('Failed to load AI status:', e);
    }
  };
  
  const loadModels = async () => {
    setLoadingModels(true);
    setAiError(null);
    try {
      const models = await api.listOllamaModels(ollamaUrl);
      setAvailableModels(models);
      if (models.length === 0) {
        setAiError('No models found. Run "ollama pull llama3.1:8b" to download a model.');
      } else {
        setAiSuccess(`Found ${models.length} models`);
        setTimeout(() => setAiSuccess(null), 3000);
      }
    } catch (e) {
      setAiError('Failed to connect to Ollama. Is it running?');
      setAvailableModels([]);
    } finally {
      setLoadingModels(false);
    }
  };
  
  const handleSaveAISettings = async () => {
    setSavingAI(true);
    setAiError(null);
    setAiSuccess(null);
    
    try {
      await api.setAIProvider(aiProvider);
      
      if (aiProvider === 'ollama') {
        const available = await api.configureOllama(ollamaUrl, ollamaModel);
        if (!available) {
          setAiError('Ollama is not available. Please ensure it is running.');
          setSavingAI(false);
          return;
        }
      }
      
      setAiSuccess('AI settings saved successfully!');
      await loadAIStatus();
      setTimeout(() => setAiSuccess(null), 2000);
    } catch (e) {
      setAiError(`Failed to save AI settings: ${e}`);
    } finally {
      setSavingAI(false);
    }
  };
  
  const loadUserDatasheets = async () => {
    if (!isTauri()) return;
    
    setLoadingDatasheets(true);
    try {
      const datasheets = await api.getUserDatasheets();
      setUserDatasheets(datasheets);
    } catch (e) {
      console.error('Failed to load user datasheets:', e);
    } finally {
      setLoadingDatasheets(false);
    }
  };
  
  const handleFileSelect = async () => {
    if (!isTauri()) {
      setDatasheetError('File upload is only available in the Tauri app');
      return;
    }
    
    setUploadingDatasheet(true);
    setDatasheetError(null);
    setDatasheetSuccess(null);
    
    try {
      // Use Tauri file dialog
      const { open } = await import('@tauri-apps/plugin-dialog');
      const selected = await open({
        multiple: false,
        filters: [{
          name: 'JSON',
          extensions: ['json']
        }],
        title: 'Select Datasheet JSON File',
      });
      
      if (selected && !Array.isArray(selected)) {
        const result = await api.uploadDatasheet(selected);
        setDatasheetSuccess(result);
        await loadUserDatasheets();
        setTimeout(() => setDatasheetSuccess(null), 3000);
      }
    } catch (e) {
      setDatasheetError(`Failed to upload datasheet: ${e}`);
    } finally {
      setUploadingDatasheet(false);
    }
  };
  
  const handleDeleteDatasheet = async (filename: string) => {
    if (!isTauri()) return;
    
    if (!confirm(`Delete datasheet "${filename}"?`)) return;
    
    try {
      await api.deleteDatasheet(filename);
      setDatasheetSuccess('Datasheet deleted successfully');
      await loadUserDatasheets();
      setTimeout(() => setDatasheetSuccess(null), 2000);
    } catch (e) {
      setDatasheetError(`Failed to delete datasheet: ${e}`);
    }
  };

  const handleSaveApiKey = () => {
    setApiKey(localApiKey.trim());
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const handleThemeChange = (newTheme: Settings['theme']) => {
    setTheme(newTheme);
    trackSettingsChanged('theme', newTheme);
    
    // Apply theme
    if (newTheme === 'dark') {
      document.documentElement.classList.add('dark');
    } else if (newTheme === 'light') {
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
  
  const handleAnalyticsToggle = () => {
    const newValue = !analyticsEnabled;
    setAnalyticsEnabled(newValue);
    setConsentStatus(newValue);
  };

  const themeOptions: { value: Settings['theme']; label: string; icon: typeof Sun }[] = [
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'system', label: 'System', icon: Monitor },
  ];

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="fixed inset-0 bg-black/50 backdrop-blur-sm z-50 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0" />
        <Dialog.Content className="fixed left-[50%] top-[50%] z-50 w-full max-w-2xl max-h-[90vh] translate-x-[-50%] translate-y-[-50%] bg-white dark:bg-gray-800 rounded-xl shadow-xl p-6 data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[state=closed]:slide-out-to-left-1/2 data-[state=closed]:slide-out-to-top-[48%] data-[state=open]:slide-in-from-left-1/2 data-[state=open]:slide-in-from-top-[48%] flex flex-col">
          <div className="flex items-center justify-between mb-6 flex-shrink-0">
            <Dialog.Title className="text-lg font-semibold text-gray-900 dark:text-white">
              Settings
            </Dialog.Title>
            <Dialog.Close className="p-2 rounded-lg transition-colors border border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 hover:border-gray-300 dark:hover:border-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800">
              <X className="w-5 h-5" />
            </Dialog.Close>
          </div>

          <div className="space-y-6 overflow-y-auto flex-1 pr-2">
            {/* Theme Section */}
            <div>
              <label className="block text-sm font-medium text-gray-900 dark:text-white mb-3">
                Theme
              </label>
              <div className="grid grid-cols-3 gap-2">
                {themeOptions.map(({ value, label, icon: Icon }) => (
                  <button
                    key={value}
                    onClick={() => handleThemeChange(value)}
                    className={`flex flex-col items-center gap-2 p-3 rounded-lg border-2 transition-colors ${
                      settings.theme === value
                        ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20'
                        : 'border-gray-200 dark:border-gray-600 hover:border-gray-300 dark:hover:border-gray-500'
                    }`}
                  >
                    <Icon className={`w-5 h-5 ${
                      settings.theme === value 
                        ? 'text-blue-500' 
                        : 'text-gray-500 dark:text-gray-400'
                    }`} />
                    <span className={`text-xs font-medium ${
                      settings.theme === value 
                        ? 'text-blue-600 dark:text-blue-400' 
                        : 'text-gray-600 dark:text-gray-300'
                    }`}>
                      {label}
                    </span>
                  </button>
                ))}
              </div>
            </div>

            {/* AI Provider Section */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
              <label className="block text-sm font-medium text-gray-900 dark:text-white mb-3">
                <span className="flex items-center gap-2">
                  <Cpu className="w-4 h-4" />
                  AI Provider
                </span>
              </label>
              
              <div className="space-y-3">
                <div className="flex items-center gap-3">
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="radio"
                      name="aiProvider"
                      value="claude"
                      checked={aiProvider === 'claude'}
                      onChange={() => setAiProvider('claude')}
                      className="text-blue-500"
                    />
                    <Cloud className="w-4 h-4 text-purple-400" />
                    <span className="text-sm text-gray-700 dark:text-gray-300">Claude (Cloud)</span>
                    {aiStatus?.claude_configured && (
                      <Check className="w-4 h-4 text-green-500" />
                    )}
                  </label>
                </div>
                
                <div className="flex items-center gap-3">
                  <label className="flex items-center gap-2 cursor-pointer">
                    <input
                      type="radio"
                      name="aiProvider"
                      value="ollama"
                      checked={aiProvider === 'ollama'}
                      onChange={() => setAiProvider('ollama')}
                      className="text-blue-500"
                    />
                    <Cpu className="w-4 h-4 text-green-400" />
                    <span className="text-sm text-gray-700 dark:text-gray-300">Ollama (Local)</span>
                    {aiStatus?.ollama_available && (
                      <Check className="w-4 h-4 text-green-500" />
                    )}
                  </label>
                </div>
              </div>
              
              {/* Claude Settings */}
              {aiProvider === 'claude' && (
                <div className="mt-4 space-y-3 pl-6 border-l-2 border-gray-200 dark:border-gray-700">
                  <p className="text-xs text-gray-600 dark:text-gray-400">
                    Your API key is stored locally and never sent to any server except Anthropic's API.
                  </p>
                  
                  <div>
                    <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                      Claude API Key
                    </label>
                    <div className="relative">
                      <input
                        type={showApiKey ? 'text' : 'password'}
                        value={localApiKey}
                        onChange={(e) => setLocalApiKey(e.target.value)}
                        placeholder="sk-ant-..."
                        className="w-full px-3 py-2 pr-10 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border-0 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 placeholder-gray-400"
                      />
                      <button
                        type="button"
                        onClick={() => setShowApiKey(!showApiKey)}
                        className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
                      >
                        {showApiKey ? (
                          <EyeOff className="w-4 h-4" />
                        ) : (
                          <Eye className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                  </div>

                  <div className="flex items-center gap-2">
                    <button
                      onClick={handleSaveApiKey}
                      className="flex items-center gap-2 px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors"
                    >
                      {saved ? (
                        <>
                          <Check className="w-4 h-4" />
                          Saved!
                        </>
                      ) : (
                        'Save API Key'
                      )}
                    </button>
                    
                    {localApiKey && (
                      <button
                        onClick={() => {
                          setLocalApiKey('');
                          setApiKey('');
                        }}
                        className="px-4 py-2 text-sm font-medium rounded-lg transition-colors border bg-transparent dark:bg-transparent border-gray-300 dark:border-gray-500 text-gray-600 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 hover:border-gray-400 dark:hover:border-gray-400 hover:text-gray-900 dark:hover:text-white focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                      >
                        Clear
                      </button>
                    )}
                  </div>

                  <p className="text-xs text-gray-500 dark:text-gray-400">
                    Get your API key from{' '}
                    <button
                      onClick={() => openUrl('https://console.anthropic.com/').catch(console.error)}
                      className="text-blue-500 hover:text-blue-600 underline"
                    >
                      console.anthropic.com
                    </button>
                  </p>
                </div>
              )}
              
              {/* Ollama Settings */}
              {aiProvider === 'ollama' && (
                <div className="mt-4 space-y-3 pl-6 border-l-2 border-gray-200 dark:border-gray-700">
                  <div>
                    <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                      Ollama URL
                    </label>
                    <input
                      type="text"
                      value={ollamaUrl}
                      onChange={(e) => setOllamaUrl(e.target.value)}
                      placeholder="http://localhost:11434"
                      className="w-full px-3 py-2 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border-0 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                  </div>
                  
                  <div>
                    <label className="block text-xs font-medium text-gray-700 dark:text-gray-300 mb-1">
                      Model
                    </label>
                    <div className="flex gap-2">
                      <select
                        value={ollamaModel}
                        onChange={(e) => setOllamaModel(e.target.value)}
                        className="flex-1 px-3 py-2 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border-0 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                      >
                        {availableModels.length > 0 ? (
                          availableModels.map((model) => (
                            <option key={model} value={model}>
                              {model}
                            </option>
                          ))
                        ) : (
                          <>
                            <option value="llama3.1:8b">llama3.1:8b (recommended)</option>
                            <option value="llama3.1:70b">llama3.1:70b (best quality)</option>
                            <option value="mistral:7b">mistral:7b (fast)</option>
                          </>
                        )}
                      </select>
                      <button
                        onClick={loadModels}
                        disabled={loadingModels}
                        className="px-3 py-2 text-sm font-medium rounded-lg transition-colors border bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-500 text-gray-800 dark:text-gray-100 hover:bg-gray-200 dark:hover:bg-gray-600 hover:border-gray-400 dark:hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {loadingModels ? (
                          <Loader2 className="w-4 h-4 animate-spin" />
                        ) : (
                          <RefreshCw className="w-4 h-4" />
                        )}
                      </button>
                    </div>
                  </div>
                  
                  <div className="p-2 bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg">
                    <p className="text-xs text-blue-700 dark:text-blue-300">
                      <strong>Setup:</strong> Install from{' '}
                      <button onClick={() => openUrl('https://ollama.ai').catch(console.error)} className="underline">
                        ollama.ai
                      </button>
                      {' '}then run: <code className="bg-blue-100 dark:bg-blue-900/50 px-1 rounded">ollama pull llama3.1:8b</code>
                    </p>
                  </div>
                  
                  <button
                    onClick={handleSaveAISettings}
                    disabled={savingAI}
                    className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
                  >
                    {savingAI && <Loader2 className="w-4 h-4 animate-spin" />}
                    Save AI Settings
                  </button>
                  
                  {aiError && (
                    <div className="p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-xs text-red-700 dark:text-red-300 flex items-center gap-2">
                      <AlertCircle className="w-4 h-4" />
                      {aiError}
                    </div>
                  )}
                  
                  {aiSuccess && (
                    <div className="p-2 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-xs text-green-700 dark:text-green-300 flex items-center gap-2">
                      <Check className="w-4 h-4" />
                      {aiSuccess}
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Component Classifier Section */}
            {aiProvider === 'ollama' && (
              <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
                <label className="block text-sm font-medium text-gray-900 dark:text-white mb-3">
                  <span className="flex items-center gap-2">
                    <Cpu className="w-4 h-4" />
                    Component Role Classifier
                  </span>
                </label>
                
                <div className="space-y-3 pl-6 border-l-2 border-gray-200 dark:border-gray-700">
                  <p className="text-xs text-gray-600 dark:text-gray-400">
                    Uses Phi-3 model via Ollama to classify component roles (MCU, Regulator, Sensor, etc.)
                  </p>
                  
                  <div className="flex items-center gap-2">
                    <button
                      onClick={async () => {
                        try {
                          const isAvailable = await api.checkClassifierAvailable();
                          if (isAvailable) {
                            setAiSuccess('Phi-3 classifier is available');
                          } else {
                            setAiError('Phi-3 not available. Run: ollama pull phi3');
                          }
                          setTimeout(() => {
                            setAiSuccess(null);
                            setAiError(null);
                          }, 3000);
                        } catch (e) {
                          setAiError(`Failed to check classifier: ${e}`);
                        }
                      }}
                      className="px-3 py-2 text-sm font-medium rounded-lg transition-colors border bg-gray-100 dark:bg-gray-700 border-gray-300 dark:border-gray-500 text-gray-800 dark:text-gray-100 hover:bg-gray-200 dark:hover:bg-gray-600 hover:border-gray-400 dark:hover:border-gray-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                    >
                      Test Connection
                    </button>
                    <span className="text-xs text-gray-500 dark:text-gray-400">
                      Model: phi3
                    </span>
                  </div>
                </div>
              </div>
            )}

            {/* Datasheets Section */}
            {isTauri() && (
              <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
                <label className="block text-sm font-medium text-gray-900 dark:text-white mb-3">
                  <span className="flex items-center gap-2">
                    <FileJson className="w-4 h-4" />
                    User Datasheets
                  </span>
                </label>
                
                <div className="space-y-3">
                  {/* Upload Area */}
                  <div
                    onClick={handleFileSelect}
                    className="border-2 border-dashed border-gray-300 dark:border-gray-600 rounded-lg p-6 text-center cursor-pointer hover:border-blue-500 dark:hover:border-blue-400 transition-colors"
                  >
                    {uploadingDatasheet ? (
                      <div className="flex items-center justify-center gap-2">
                        <Loader2 className="w-5 h-5 animate-spin text-gray-400" />
                        <span className="text-sm text-gray-600 dark:text-gray-400">Uploading...</span>
                      </div>
                    ) : (
                      <>
                        <Upload className="w-8 h-8 mx-auto mb-2 text-gray-400 dark:text-gray-500" />
                        <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">
                          Click to upload datasheet JSON file
                        </p>
                        <p className="text-xs text-gray-500 dark:text-gray-500">
                          JSON files only
                        </p>
                      </>
                    )}
                  </div>
                  
                  {/* User Datasheets List */}
                  {loadingDatasheets ? (
                    <div className="flex items-center justify-center py-4">
                      <Loader2 className="w-5 h-5 animate-spin text-gray-400" />
                    </div>
                  ) : userDatasheets.length > 0 ? (
                    <div className="space-y-2">
                      {userDatasheets.map((ds, idx) => (
                        <div
                          key={idx}
                          className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700/50 rounded-lg"
                        >
                          <div className="flex-1 min-w-0">
                            <p className="text-sm font-medium text-gray-900 dark:text-white truncate">
                              {ds.filename}
                            </p>
                            <p className="text-xs text-gray-500 dark:text-gray-400">
                              {ds.part_numbers.join(', ')} • {ds.manufacturer}
                            </p>
                          </div>
                          <button
                            onClick={() => handleDeleteDatasheet(ds.filename)}
                            className="p-2 rounded-lg transition-colors border border-transparent text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/30 hover:border-red-200 dark:hover:border-red-800 focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800"
                            title="Delete datasheet"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <p className="text-sm text-gray-500 dark:text-gray-400 text-center py-4">
                      No user datasheets uploaded yet
                    </p>
                  )}
                  
                  {datasheetError && (
                    <div className="p-2 bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg text-xs text-red-700 dark:text-red-300 flex items-center gap-2">
                      <AlertCircle className="w-4 h-4" />
                      {datasheetError}
                    </div>
                  )}
                  
                  {datasheetSuccess && (
                    <div className="p-2 bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800 rounded-lg text-xs text-green-700 dark:text-green-300 flex items-center gap-2">
                      <Check className="w-4 h-4" />
                      {datasheetSuccess}
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Analytics Section */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
              <label className="block text-sm font-medium text-gray-900 dark:text-white mb-3">
                <span className="flex items-center gap-2">
                  <BarChart3 className="w-4 h-4" />
                  Usage Analytics
                </span>
              </label>
              
              <div className="flex items-center justify-between">
                <div className="flex-1 min-w-0 pr-4">
                  <p className="text-xs text-gray-600 dark:text-gray-400 leading-relaxed">
                    Share anonymous usage data to help improve DesignGuard.
                    No personal information or schematic data is collected.
                  </p>
                </div>
                <button
                  role="switch"
                  aria-checked={analyticsEnabled}
                  onClick={handleAnalyticsToggle}
                  className={`relative inline-flex h-6 w-11 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 dark:focus:ring-offset-gray-800 ${
                    analyticsEnabled ? 'bg-blue-600' : 'bg-gray-300 dark:bg-gray-600'
                  }`}
                >
                  <span
                    className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow ring-0 transition duration-200 ease-in-out ${
                      analyticsEnabled ? 'translate-x-5' : 'translate-x-0'
                    }`}
                  />
                </button>
              </div>
            </div>

            {/* About Section */}
            <div className="pt-4 border-t border-gray-200 dark:border-gray-700">
              <p className="text-xs text-gray-500 dark:text-gray-400 text-center">
                DesignGuard v0.1.0
              </p>
            </div>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
