export interface QuotaWindow { usedPercent: number; windowDurationMins: number | null; resetsAt: number | null }
export interface Bucket { id: string; name: string | null; primary: QuotaWindow | null; secondary: QuotaWindow | null; planType: string | null; credits: { unlimited: boolean | null; balance: string | null; hasCredits: boolean | null } | null }
export interface Snapshot { buckets: Bucket[]; tokenSummary: { lifetimeTokens: number | null; peakDailyTokens: number | null; currentStreakDays: number | null } | null; fetchedAt: number; ordinaryUsageAllowed: boolean | null; remainingResetCount: number | null; resetCreditExpirations: (number | null)[] | null }
export interface MonitorState { status: 'connecting' | 'ready' | 'loginRequired' | 'unsupportedAuth' | 'error' | 'stopped'; snapshot: Snapshot | null; stale: boolean; message: string | null }
export interface Settings { showWindow: boolean; showTray: boolean; showConsole: boolean; showTouchbar: boolean; alwaysOnTop: boolean; selectedLimitId: string | null; intervalSecs: number; codexPath: string }
export interface DesktopState { platform: string; usage: MonitorState; settings: Settings; startupWarning: string | null }
export interface TauriApi {
  core: { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> };
  event: { listen<T>(event: string, callback: (event: { payload: T }) => void): Promise<() => void> };
}
declare global { interface Window { __TAURI__?: TauriApi } }
