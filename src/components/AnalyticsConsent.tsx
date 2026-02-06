import { useState, useEffect } from 'react';
import { BarChart3, X } from 'lucide-react';
import { getConsentStatus, setConsentStatus } from '../lib/analytics';

/**
 * One-time consent banner shown on first launch.
 * Once the user accepts or declines, the banner never appears again
 * (the choice is persisted in localStorage and can be changed in Settings).
 */
export function AnalyticsConsent() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    // Only show if no decision has been recorded yet
    const consent = getConsentStatus();
    if (consent === null) {
      setVisible(true);
    }
  }, []);

  if (!visible) return null;

  const handleDecision = (accepted: boolean) => {
    setConsentStatus(accepted);
    setVisible(false);
  };

  return (
    <div className="fixed bottom-4 left-4 right-4 z-50 flex justify-center animate-in slide-in-from-bottom-2 fade-in duration-300">
      <div className="max-w-lg w-full bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-xl shadow-lg p-5">
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0 p-2 bg-blue-50 dark:bg-blue-900/30 rounded-lg">
            <BarChart3 className="w-5 h-5 text-blue-500" />
          </div>
          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-semibold text-gray-900 dark:text-white">
              Help improve DesignGuard
            </h3>
            <p className="mt-1 text-xs text-gray-600 dark:text-gray-400 leading-relaxed">
              Share anonymous usage data to help us improve the app.
              No personal information or schematic data is collected.
              You can change this anytime in Settings.
            </p>
            <div className="mt-3 flex items-center gap-2">
              <button
                onClick={() => handleDecision(true)}
                className="px-4 py-1.5 text-xs font-medium text-white bg-blue-600 rounded-lg hover:bg-blue-700 focus:outline-none focus:ring-2 focus:ring-blue-500 transition-colors"
              >
                Accept
              </button>
              <button
                onClick={() => handleDecision(false)}
                className="px-4 py-1.5 text-xs font-medium rounded-lg transition-colors border border-gray-300 dark:border-gray-600 text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 focus:outline-none focus:ring-2 focus:ring-blue-500"
              >
                Decline
              </button>
            </div>
          </div>
          <button
            onClick={() => handleDecision(false)}
            className="flex-shrink-0 p-1 text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            aria-label="Dismiss"
          >
            <X className="w-4 h-4" />
          </button>
        </div>
      </div>
    </div>
  );
}
