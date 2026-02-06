import { FolderOpen, FolderX, Settings } from 'lucide-react';
import { useStore } from '../lib/store';
import { isTauri } from '../lib/api';

interface ProjectControlsProps {
  onOpenSettings: () => void;
}

export function ProjectControls({ onOpenSettings }: ProjectControlsProps) {
  const { project, isLoading, openProject, closeProject, setError } = useStore();

  const handleOpenProject = async () => {
    if (!isTauri()) {
      setError('File dialog is only available in the Tauri app. Please run "npm run tauri dev".');
      return;
    }

    try {
      // Dynamic import to avoid issues when not in Tauri
      const { open } = await import('@tauri-apps/plugin-dialog');
      
      // Allow selecting directories (project folders)
      // Backend handles both files and directories, and supports KiCad 4-9 formats
      // When a directory is selected, backend searches for all KiCad files (legacy and modern)
      const selected = await open({
        multiple: false,
        directory: true, // Allow selecting folders
        title: 'Select KiCAD Project Folder',
      });

      if (selected) {
        const path = Array.isArray(selected) ? selected[0] : selected;
        if (path) {
          await openProject(path);
        }
      }
    } catch (error) {
      console.error('Failed to open file dialog:', error);
      setError(error instanceof Error ? error.message : 'Failed to open file dialog');
    }
  };

  return (
    <div className="flex items-center gap-2">
      {!project ? (
        <button
          onClick={handleOpenProject}
          disabled={isLoading}
          className="flex items-center gap-2 px-3 py-2 text-sm font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed transition-colors dark:focus:ring-offset-gray-800"
        >
          <FolderOpen className="w-4 h-4" />
          {isLoading ? 'Opening...' : 'Open Project Folder'}
        </button>
      ) : (
        <button
          onClick={closeProject}
          className="flex items-center gap-2 px-3 py-2 text-sm font-medium text-gray-700 dark:text-gray-200 bg-gray-100 dark:bg-gray-700 rounded-lg hover:bg-gray-200 dark:hover:bg-gray-600 focus:outline-none focus:ring-2 focus:ring-gray-500 focus:ring-offset-2 transition-colors dark:focus:ring-offset-gray-800"
        >
          <FolderX className="w-4 h-4" />
          Close Project
        </button>
      )}
      
      <button
        onClick={onOpenSettings}
        className="p-2 text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200 hover:bg-gray-100 dark:hover:bg-gray-700 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-500 transition-colors"
        title="Settings"
      >
        <Settings className="w-5 h-5" />
      </button>
    </div>
  );
}
