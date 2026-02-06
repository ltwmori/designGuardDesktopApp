import posthog from 'posthog-js';

// PostHog configuration
// Set VITE_POSTHOG_KEY and VITE_POSTHOG_HOST in a .env file to enable analytics.
// Without a key, analytics is silently disabled.
const POSTHOG_KEY = import.meta.env.VITE_POSTHOG_KEY || '';
const POSTHOG_HOST = import.meta.env.VITE_POSTHOG_HOST || 'https://eu.i.posthog.com';

const CONSENT_STORAGE_KEY = 'analytics_consent';

let initialized = false;

// ============================================================================
// Initialization & Consent
// ============================================================================

/**
 * Initialize PostHog analytics with explicit user consent.
 * - autocapture disabled: we only track events we explicitly define
 * - session recording disabled: privacy-first for desktop app
 * - opt-out by default: requires explicit opt-in
 */
export function initAnalytics(userConsented: boolean): void {
  if (!userConsented || !POSTHOG_KEY || initialized) return;

  try {
    posthog.init(POSTHOG_KEY, {
      api_host: POSTHOG_HOST,
      autocapture: false,
      disable_session_recording: true,
      opt_out_capturing_by_default: true,
      persistence: 'localStorage',
      // Desktop app — no cookies needed
      disable_cookie: true,
      // Respect Do Not Track browser setting
      respect_dnt: true,
      // Batch events for efficiency
      loaded: (ph) => {
        ph.opt_in_capturing();
        initialized = true;
      },
    });
  } catch (error) {
    console.warn('Analytics initialization failed:', error);
  }
}

/**
 * Shut down analytics and clear local data when user revokes consent.
 */
export function disableAnalytics(): void {
  if (initialized) {
    posthog.opt_out_capturing();
    posthog.reset();
    initialized = false;
  }
}

/**
 * Check whether the user has previously given analytics consent.
 * Returns null if no decision has been made yet (first launch).
 */
export function getConsentStatus(): boolean | null {
  const stored = localStorage.getItem(CONSENT_STORAGE_KEY);
  if (stored === null) return null;
  return stored === 'true';
}

/**
 * Persist the user's consent decision and initialize/disable accordingly.
 */
export function setConsentStatus(accepted: boolean): void {
  localStorage.setItem(CONSENT_STORAGE_KEY, String(accepted));
  if (accepted) {
    initAnalytics(true);
  } else {
    disableAnalytics();
  }
}

// ============================================================================
// Event Tracking
// ============================================================================

/**
 * Track a named event with optional properties.
 * No-op if analytics is not active.
 */
export function trackEvent(eventName: string, properties?: Record<string, unknown>): void {
  if (!initialized || !posthog.has_opted_in_capturing()) return;
  posthog.capture(eventName, properties);
}

/**
 * Track a screen/tab view.
 */
export function trackScreen(screenName: string): void {
  trackEvent('screen_viewed', { screen: screenName });
}

// ============================================================================
// Typed Event Helpers — keep tracking consistent across the app
// ============================================================================

export function trackProjectOpened(properties: {
  component_count: number;
  net_count: number;
  source_cad?: string;
}): void {
  trackEvent('project_opened', properties);
}

export function trackProjectClosed(): void {
  trackEvent('project_closed');
}

export function trackValidationStarted(checkTypes: string[]): void {
  trackEvent('validation_started', { check_types: checkTypes });
}

export function trackValidationCompleted(properties: {
  issues_found: number;
  duration_ms: number;
  check_types: string[];
}): void {
  trackEvent('validation_completed', properties);
}

export function trackAIAnalysisStarted(provider: string): void {
  trackEvent('ai_analysis_started', { provider });
}

export function trackAIAnalysisCompleted(properties: {
  provider: string;
  issues_found: number;
  suggestions_count: number;
  duration_ms: number;
}): void {
  trackEvent('ai_analysis_completed', properties);
}

export function trackAIAnalysisFailed(properties: {
  provider: string;
  error_type: string;
}): void {
  trackEvent('ai_analysis_failed', properties);
}

export function trackChatMessageSent(): void {
  trackEvent('chat_message_sent');
}

export function trackTabChanged(tabName: string): void {
  trackScreen(tabName);
}

export function trackSettingsChanged(setting: string, value: string): void {
  trackEvent('settings_changed', { setting, value });
}

export function trackError(context: string, errorMessage: string): void {
  trackEvent('app_error', { context, error_message: errorMessage });
}

// ============================================================================
// Boot — call once on app startup
// ============================================================================

/**
 * Restore analytics state from a previous session.
 * Call this once in the app entry point.
 */
export function bootAnalytics(): void {
  const consent = getConsentStatus();
  if (consent === true) {
    initAnalytics(true);
  }
}
