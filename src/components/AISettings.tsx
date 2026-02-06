import { useState, useEffect } from 'react';
import { Settings, Cpu, Cloud, RefreshCw, Check, X, Loader2 } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import { api } from '../lib/api';
import type { ProviderStatus } from '../types';

interface AISettingsProps {
  isOpen: boolean;
  onClose: () => void;
}

export function AISettings({ isOpen, onClose }: AISettingsProps) {
  const [provider, setProvider] = useState<'claude' | 'ollama'>('claude');
  const [claudeKey, setClaudeKey] = useState('');
  const [ollamaUrl, setOllamaUrl] = useState('http://localhost:11434');
  const [ollamaModel, setOllamaModel] = useState('llama3.1:8b');
  const [availableModels, setAvailableModels] = useState<string[]>([]);
  const [status, setStatus] = useState<ProviderStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);

  // Load status on mount
  useEffect(() => {
    if (isOpen) {
      loadStatus();
    }
  }, [isOpen]);

  const loadStatus = async () => {
    try {
      const s = await api.getAIStatus();
      setStatus(s);
      setProvider(s.preferred as 'claude' | 'ollama');
      if (s.ollama_models.length > 0) {
        setAvailableModels(s.ollama_models);
      }
    } catch (e) {
      console.error('Failed to load AI status:', e);
    }
  };

  const loadModels = async () => {
    setLoading(true);
    setError(null);
    try {
      const models = await api.listOllamaModels(ollamaUrl);
      setAvailableModels(models);
      if (models.length === 0) {
        setError('No models found. Run "ollama pull llama3.1:8b" to download a model.');
      } else {
        setSuccess(`Found ${models.length} models`);
        setTimeout(() => setSuccess(null), 3000);
      }
    } catch (e) {
      setError('Failed to connect to Ollama. Is it running?');
      setAvailableModels([]);
    } finally {
      setLoading(false);
    }
  };

  const handleSave = async () => {
    setLoading(true);
    setError(null);
    setSuccess(null);

    try {
      // Set preferred provider
      await api.setAIProvider(provider);

      // Configure Claude if selected and key provided
      if (provider === 'claude' && claudeKey) {
        await api.configureClaude(claudeKey);
      }

      // Configure Ollama if selected
      if (provider === 'ollama') {
        const available = await api.configureOllama(ollamaUrl, ollamaModel);
        if (!available) {
          setError('Ollama is not available. Please ensure it is running.');
          setLoading(false);
          return;
        }
      }

      setSuccess('Settings saved successfully!');
      await loadStatus();
      setTimeout(() => {
        setSuccess(null);
        onClose();
      }, 1500);
    } catch (e) {
      setError(`Failed to save settings: ${e}`);
    } finally {
      setLoading(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50">
      <div className="bg-gray-800 rounded-lg shadow-xl w-full max-w-md mx-4">
        {/* Header */}
        <div className="flex items-center justify-between p-4 border-b border-gray-700">
          <div className="flex items-center gap-2">
            <Settings className="w-5 h-5 text-blue-400" />
            <h2 className="text-lg font-semibold text-white">AI Provider Settings</h2>
          </div>
          <button
            onClick={onClose}
            className="text-gray-400 hover:text-white transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>

        {/* Content */}
        <div className="p-4 space-y-6">
          {/* Provider Selection */}
          <div>
            <label className="block text-sm font-medium text-gray-300 mb-2">
              AI Provider
            </label>
            <div className="space-y-2">
              <label className="flex items-center gap-3 p-3 rounded-lg border border-gray-600 cursor-pointer hover:border-blue-500 transition-colors">
                <input
                  type="radio"
                  name="provider"
                  value="claude"
                  checked={provider === 'claude'}
                  onChange={() => setProvider('claude')}
                  className="text-blue-500"
                />
                <Cloud className="w-5 h-5 text-purple-400" />
                <div className="flex-1">
                  <div className="text-white font-medium">Claude (Cloud)</div>
                  <div className="text-xs text-gray-400">Best quality, requires API key</div>
                </div>
                {status?.claude_configured && (
                  <Check className="w-4 h-4 text-green-400" />
                )}
              </label>

              <label className="flex items-center gap-3 p-3 rounded-lg border border-gray-600 cursor-pointer hover:border-blue-500 transition-colors">
                <input
                  type="radio"
                  name="provider"
                  value="ollama"
                  checked={provider === 'ollama'}
                  onChange={() => setProvider('ollama')}
                  className="text-blue-500"
                />
                <Cpu className="w-5 h-5 text-green-400" />
                <div className="flex-1">
                  <div className="text-white font-medium">Ollama (Local)</div>
                  <div className="text-xs text-gray-400">100% offline, requires local install</div>
                </div>
                {status?.ollama_available && (
                  <Check className="w-4 h-4 text-green-400" />
                )}
              </label>
            </div>
          </div>

          {/* Claude Settings */}
          {provider === 'claude' && (
            <div>
              <label className="block text-sm font-medium text-gray-300 mb-2">
                Claude API Key
              </label>
              <input
                type="password"
                value={claudeKey}
                onChange={(e) => setClaudeKey(e.target.value)}
                placeholder="sk-ant-..."
                className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
              />
              <p className="mt-1 text-xs text-gray-400">
                Get your API key from{' '}
                <button
                  onClick={() => openUrl('https://console.anthropic.com').catch(console.error)}
                  className="text-blue-400 hover:underline"
                >
                  console.anthropic.com
                </button>
              </p>
            </div>
          )}

          {/* Ollama Settings */}
          {provider === 'ollama' && (
            <>
              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Ollama URL
                </label>
                <input
                  type="text"
                  value={ollamaUrl}
                  onChange={(e) => setOllamaUrl(e.target.value)}
                  placeholder="http://localhost:11434"
                  className="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white placeholder-gray-400 focus:outline-none focus:border-blue-500"
                />
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-300 mb-2">
                  Model
                </label>
                <div className="flex gap-2">
                  <select
                    value={ollamaModel}
                    onChange={(e) => setOllamaModel(e.target.value)}
                    className="flex-1 px-3 py-2 bg-gray-700 border border-gray-600 rounded-lg text-white focus:outline-none focus:border-blue-500"
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
                        <option value="codellama:13b">codellama:13b (code-focused)</option>
                      </>
                    )}
                  </select>
                  <button
                    onClick={loadModels}
                    disabled={loading}
                    className="px-3 py-2 bg-gray-600 hover:bg-gray-500 rounded-lg text-white transition-colors disabled:opacity-50"
                  >
                    {loading ? (
                      <Loader2 className="w-5 h-5 animate-spin" />
                    ) : (
                      <RefreshCw className="w-5 h-5" />
                    )}
                  </button>
                </div>
              </div>

              <div className="p-3 bg-blue-900/30 border border-blue-700 rounded-lg">
                <h4 className="text-sm font-medium text-blue-300 mb-2">Setup Ollama:</h4>
                <ol className="text-xs text-blue-200 space-y-1 list-decimal ml-4">
                  <li>Install from <button onClick={() => openUrl('https://ollama.ai').catch(console.error)} className="underline">ollama.ai</button></li>
                  <li>Run: <code className="bg-blue-900/50 px-1 rounded">ollama pull llama3.1:8b</code></li>
                  <li>Ollama runs automatically in background</li>
                </ol>
              </div>
            </>
          )}

          {/* Status */}
          {status && (
            <div className="flex items-center gap-2 text-sm">
              <div
                className={`w-2 h-2 rounded-full ${
                  (provider === 'claude' && status.claude_configured) ||
                  (provider === 'ollama' && status.ollama_available)
                    ? 'bg-green-500'
                    : 'bg-red-500'
                }`}
              />
              <span className="text-gray-300">
                {provider === 'claude'
                  ? status.claude_configured
                    ? 'Claude configured'
                    : 'API key not set'
                  : status.ollama_available
                  ? 'Ollama connected'
                  : 'Ollama not running'}
              </span>
            </div>
          )}

          {/* Error/Success Messages */}
          {error && (
            <div className="p-3 bg-red-900/30 border border-red-700 rounded-lg text-red-300 text-sm">
              {error}
            </div>
          )}
          {success && (
            <div className="p-3 bg-green-900/30 border border-green-700 rounded-lg text-green-300 text-sm">
              {success}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="flex justify-end gap-3 p-4 border-t border-gray-700">
          <button
            onClick={onClose}
            className="px-4 py-2 text-gray-300 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={loading}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-500 text-white rounded-lg transition-colors disabled:opacity-50 flex items-center gap-2"
          >
            {loading && <Loader2 className="w-4 h-4 animate-spin" />}
            Save Settings
          </button>
        </div>
      </div>
    </div>
  );
}
