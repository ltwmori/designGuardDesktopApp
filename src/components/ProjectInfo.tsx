import { FileCode, Cpu, Cable, Tag, Clock } from 'lucide-react';
import { useStore } from '../lib/store';

export function ProjectInfo() {
  const { project, schematic, isLoading } = useStore();

  if (isLoading) {
    return (
      <div className="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700">
        <div className="animate-pulse space-y-3">
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4"></div>
          <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/2"></div>
          <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-2/3"></div>
        </div>
      </div>
    );
  }

  if (!project || !schematic) {
    return (
      <div className="p-4 bg-gray-50 dark:bg-gray-800/50 rounded-lg border border-dashed border-gray-300 dark:border-gray-600">
        <div className="text-center text-gray-500 dark:text-gray-400">
          <FileCode className="w-8 h-8 mx-auto mb-2 opacity-50" />
          <p className="text-sm">No project loaded</p>
          <p className="text-xs mt-1">Open a KiCAD schematic to begin</p>
        </div>
      </div>
    );
  }

  const componentCount = schematic.components.length + schematic.power_symbols.length;
  const netCount = schematic.nets.length;
  const labelCount = schematic.labels.length;

  return (
    <div className="p-4 bg-white dark:bg-gray-800 rounded-lg border border-gray-200 dark:border-gray-700 space-y-4">
      {/* Project name */}
      <div>
        <h3 className="font-semibold text-gray-900 dark:text-white truncate" title={project.name}>
          {project.name}
        </h3>
        <p className="text-xs text-gray-500 dark:text-gray-400 truncate mt-1" title={project.path}>
          {project.path}
        </p>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-3">
        <div className="flex flex-col items-center p-2 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
          <Cpu className="w-4 h-4 text-blue-500 mb-1" />
          <span className="text-lg font-semibold text-gray-900 dark:text-white">
            {componentCount}
          </span>
          <span className="text-xs text-gray-500 dark:text-gray-400">Components</span>
        </div>
        
        <div className="flex flex-col items-center p-2 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
          <Cable className="w-4 h-4 text-green-500 mb-1" />
          <span className="text-lg font-semibold text-gray-900 dark:text-white">
            {netCount}
          </span>
          <span className="text-xs text-gray-500 dark:text-gray-400">Nets</span>
        </div>
        
        <div className="flex flex-col items-center p-2 bg-gray-50 dark:bg-gray-700/50 rounded-lg">
          <Tag className="w-4 h-4 text-purple-500 mb-1" />
          <span className="text-lg font-semibold text-gray-900 dark:text-white">
            {labelCount}
          </span>
          <span className="text-xs text-gray-500 dark:text-gray-400">Labels</span>
        </div>
      </div>

      {/* Version info */}
      {schematic.version && (
        <div className="pt-2 border-t border-gray-200 dark:border-gray-700">
          <p className="text-xs text-gray-500 dark:text-gray-400">
            KiCAD Version: {schematic.version}
          </p>
        </div>
      )}

      {/* Last analyzed */}
      {project.last_analyzed && (
        <div className="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
          <Clock className="w-3 h-3" />
          <span>
            Last analyzed: {new Date(project.last_analyzed).toLocaleString()}
          </span>
        </div>
      )}
    </div>
  );
}
