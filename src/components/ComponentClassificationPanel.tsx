import { useState, useEffect } from 'react';
import { RefreshCw, Search, Filter, Loader2, CheckCircle, AlertCircle, Cpu, X } from 'lucide-react';
import { api } from '../lib/api';
import { useStore } from '../lib/store';
import type { ClassificationResult, RoleCategoryInfo } from '../types';

export function ComponentClassificationPanel() {
  const { project } = useStore();
  const [classifications, setClassifications] = useState<ClassificationResult[]>([]);
  const [categories, setCategories] = useState<RoleCategoryInfo[]>([]);
  const [classifying, setClassifying] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [available, setAvailable] = useState<boolean | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [expandedComponents, setExpandedComponents] = useState<Set<string>>(new Set());

  // Load categories and check availability on mount
  useEffect(() => {
    loadCategories();
    checkAvailability();
  }, []);

  const loadCategories = async () => {
    try {
      const cats = await api.getComponentRoleCategories();
      setCategories(cats);
    } catch (e) {
      console.error('Failed to load categories:', e);
    }
  };

  const checkAvailability = async () => {
    try {
      const isAvailable = await api.checkClassifierAvailable();
      setAvailable(isAvailable);
    } catch (e) {
      console.error('Failed to check classifier availability:', e);
      setAvailable(false);
    }
  };

  const handleClassifyAll = async () => {
    if (!project) {
      setError('No project loaded');
      return;
    }

    setClassifying(true);
    setError(null);

    try {
      const results = await api.classifySchematicComponents();
      setClassifications(results);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to classify components');
    } finally {
      setClassifying(false);
    }
  };

  const toggleExpand = (refDes: string) => {
    const newExpanded = new Set(expandedComponents);
    if (newExpanded.has(refDes)) {
      newExpanded.delete(refDes);
    } else {
      newExpanded.add(refDes);
    }
    setExpandedComponents(newExpanded);
  };

  // Filter classifications
  const filteredClassifications = classifications.filter((result) => {
    const matchesSearch = 
      result.component.ref_des.toLowerCase().includes(searchTerm.toLowerCase()) ||
      result.component.part_number.toLowerCase().includes(searchTerm.toLowerCase()) ||
      result.role.toLowerCase().includes(searchTerm.toLowerCase());
    
    if (!matchesSearch) return false;

    if (selectedCategory) {
      const category = categories.find(cat => 
        cat.roles.some(r => r.name === result.role)
      );
      return category?.category === selectedCategory;
    }

    return true;
  });

  // Group by category
  const groupedByCategory = filteredClassifications.reduce((acc, result) => {
    const category = categories.find(cat => 
      cat.roles.some(r => r.name === result.role)
    )?.category || 'Other';
    
    if (!acc[category]) {
      acc[category] = [];
    }
    acc[category].push(result);
    return acc;
  }, {} as Record<string, ClassificationResult[]>);

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 0.8) return 'text-green-500';
    if (confidence >= 0.6) return 'text-yellow-500';
    return 'text-red-500';
  };

  const getConfidenceBg = (confidence: number) => {
    if (confidence >= 0.8) return 'bg-green-500';
    if (confidence >= 0.6) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  const formatRoleName = (role: string) => {
    // Convert snake_case to readable name
    return role.replace(/_/g, ' ').replace(/\b\w/g, (c) => c.toUpperCase());
  };

  return (
    <div className="flex flex-col h-full bg-white dark:bg-gray-800">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 dark:border-gray-700">
        <div className="flex items-center gap-2">
          <Cpu className="w-5 h-5 text-blue-500" />
          <h2 className="text-lg font-semibold text-gray-900 dark:text-white">
            Component Classification
          </h2>
        </div>
        <button
          onClick={checkAvailability}
          className="p-2 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg transition-colors"
          title="Check availability"
        >
          <RefreshCw className="w-4 h-4" />
        </button>
      </div>

      {/* Intro / explanation */}
      <div className="px-4 pt-3 text-xs text-gray-600 dark:text-gray-400 border-b border-gray-200 dark:border-gray-700">
        This tool uses a small local AI model (Phi‑3 via Ollama) to guess the functional{" "}
        role of each component (MCU, regulator, sensor, etc.). The colored dot and{" "}
        percentage show how confident the model is in its guess.
      </div>

      {/* Status */}
      {available !== null && (
        <div className={`p-3 mx-4 mt-4 rounded-lg flex items-center gap-2 ${
          available 
            ? 'bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800' 
            : 'bg-yellow-50 dark:bg-yellow-900/20 border border-yellow-200 dark:border-yellow-800'
        }`}>
          {available ? (
            <>
              <CheckCircle className="w-4 h-4 text-green-500" />
              <span className="text-sm text-green-700 dark:text-green-300">
                Phi-3 classifier available via Ollama
              </span>
            </>
          ) : (
            <>
              <AlertCircle className="w-4 h-4 text-yellow-500" />
              <span className="text-sm text-yellow-700 dark:text-yellow-300">
                Phi-3 not available. Install Ollama and run: <code className="bg-yellow-100 dark:bg-yellow-900/50 px-1 rounded">ollama pull phi3</code>
              </span>
            </>
          )}
        </div>
      )}

      {/* Controls */}
      <div className="p-4 space-y-3 border-b border-gray-200 dark:border-gray-700">
        <button
          onClick={handleClassifyAll}
          disabled={!project || !available || classifying}
          className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {classifying ? (
            <>
              <Loader2 className="w-4 h-4 animate-spin" />
              Classifying...
            </>
          ) : (
            <>
              <Cpu className="w-4 h-4" />
              Classify All Components
            </>
          )}
        </button>

        {classifications.length > 0 && (
          <>
            {/* Search */}
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-400" />
              <input
                type="text"
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                placeholder="Search by ref, part number, or role..."
                className="w-full pl-10 pr-4 py-2 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border-0 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>

            {/* Category Filter */}
            <div className="flex items-center gap-2">
              <Filter className="w-4 h-4 text-gray-400" />
              <select
                value={selectedCategory || ''}
                onChange={(e) => setSelectedCategory(e.target.value || null)}
                className="flex-1 px-3 py-2 text-sm text-gray-900 dark:text-white bg-gray-100 dark:bg-gray-700 border-0 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                <option value="">All Categories</option>
                {categories.map((cat) => (
                  <option key={cat.category} value={cat.category}>
                    {cat.category}
                  </option>
                ))}
              </select>
            </div>
          </>
        )}
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

      {/* Results */}
      <div className="flex-1 overflow-y-auto p-4">
        {classifications.length === 0 ? (
          <div className="text-center py-12">
            <Cpu className="w-12 h-12 mx-auto mb-4 text-gray-400" />
            <p className="text-gray-500 dark:text-gray-400">
              {!project 
                ? 'Open a project to classify components'
                : available === false
                ? 'Phi-3 classifier not available. Please configure Ollama.'
                : 'Click "Classify All Components" to start classification'}
            </p>
          </div>
        ) : filteredClassifications.length === 0 ? (
          <div className="text-center py-12">
            <Search className="w-12 h-12 mx-auto mb-4 text-gray-400" />
            <p className="text-gray-500 dark:text-gray-400">No components match your filters</p>
          </div>
        ) : (
          <div className="space-y-6">
            {Object.entries(groupedByCategory).map(([category, results]) => (
              <div key={category}>
                <h3 className="text-sm font-semibold text-gray-700 dark:text-gray-300 mb-2">
                  {category} ({results.length})
                </h3>
                <div className="space-y-2">
                  {results.map((result, idx) => {
                    const isExpanded = expandedComponents.has(result.component.ref_des);
                    const roleInfo = categories
                      .flatMap(cat => cat.roles)
                      .find(r => r.name === result.role);

                    return (
                      <div
                        key={idx}
                        className="bg-gray-50 dark:bg-gray-700/50 rounded-lg border border-gray-200 dark:border-gray-600"
                      >
                        <button
                          onClick={() => toggleExpand(result.component.ref_des)}
                          className="w-full p-3 flex items-center justify-between hover:bg-gray-100 dark:hover:bg-gray-700 transition-colors"
                        >
                          <div className="flex items-center gap-3 flex-1 min-w-0">
                            <div className="flex-shrink-0">
                              <div className={`w-2 h-2 rounded-full ${getConfidenceBg(result.confidence)}`} />
                            </div>
                            <div className="flex-1 min-w-0">
                              <div className="flex items-center gap-2">
                                <span className="font-medium text-gray-900 dark:text-white">
                                  {result.component.ref_des}
                                </span>
                                <span className="text-sm text-gray-500 dark:text-gray-400">
                                  {result.component.part_number}
                                </span>
                              </div>
                              <div className="text-sm text-gray-600 dark:text-gray-300 mt-0.5">
                                {formatRoleName(result.role)}
                              </div>
                            </div>
                            <div className="flex flex-col items-end gap-0.5">
                              <span className="text-[10px] uppercase tracking-wide text-gray-400 dark:text-gray-500">
                                Confidence
                              </span>
                              <span className={`text-xs font-medium ${getConfidenceColor(result.confidence)}`}>
                                {(result.confidence * 100).toFixed(0)}%
                              </span>
                            </div>
                          </div>
                        </button>

                        {isExpanded && (
                          <div className="px-3 pb-3 pt-0 border-t border-gray-200 dark:border-gray-600">
                            <div className="pt-3 space-y-2">
                              {roleInfo && (
                                <p className="text-xs text-gray-600 dark:text-gray-400">
                                  {roleInfo.description}
                                </p>
                              )}
                              {result.reasoning && (
                                <div className="text-xs text-gray-600 dark:text-gray-400">
                                  <strong>Reasoning:</strong> {result.reasoning}
                                </div>
                              )}
                              {result.alternatives && result.alternatives.length > 0 && (
                                <div className="text-xs">
                                  <strong className="text-gray-700 dark:text-gray-300">Alternatives:</strong>
                                  <ul className="mt-1 space-y-1">
                                    {result.alternatives.map((alt, altIdx) => (
                                      <li key={altIdx} className="text-gray-600 dark:text-gray-400">
                                        {formatRoleName(alt.role)} ({(alt.confidence * 100).toFixed(0)}%)
                                      </li>
                                    ))}
                                  </ul>
                                </div>
                              )}
                              {!roleInfo && !result.reasoning && (!result.alternatives || result.alternatives.length === 0) && (
                                <p className="text-xs text-gray-500 dark:text-gray-400">
                                  No additional explanation from the model. This role was inferred mainly from the
                                  reference designator and value.
                                </p>
                              )}
                            </div>
                          </div>
                        )}
                      </div>
                    );
                  })}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
