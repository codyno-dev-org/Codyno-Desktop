// CodyNo does not have its own release feed yet. Keep the updater completely
// disabled until the desktop updater points at CodyNo-owned artifacts.
export const UPDATES_ENABLED = false;
export const COST_TRACKING_ENABLED = true;
export const ANNOUNCEMENTS_ENABLED = false;
export const CONFIGURATION_ENABLED = true;
// Telemetry is compiled out of the CLI. Do not expose a desktop control that
// suggests it can be enabled or that writes the now-unused setting.
export const TELEMETRY_UI_ENABLED = false;
export const DICTATION_ALLOWED_PROVIDERS: string[] | null = null;
