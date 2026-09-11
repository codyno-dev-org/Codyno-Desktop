import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, ExternalLink, LoaderCircle, LogIn } from 'lucide-react';
import {
  acpResolveCodyNoModel,
  acpSaveProviderConfig,
} from '../../acp/providers';
import { Button } from '../ui/button';
import { defineMessages, useIntl } from '../../i18n';

const CODYNO_PROVIDER_ID = 'litellm';
const CODYNO_BASE_PATH = 'v1/chat/completions';
const POLL_INTERVAL_MS = 2500;

const i18n = defineMessages({
  title: {
    id: 'providerSelector.title',
    defaultMessage: 'Sign in to CodyNo',
  },
  description: {
    id: 'providerSelector.description',
    defaultMessage:
      'Sign in with your CodyNo account. Your requests will be routed through the CodyNo AI gateway.',
  },
  signIn: {
    id: 'providerSelector.signIn',
    defaultMessage: 'Sign in with CodyNo',
  },
  signingIn: {
    id: 'providerSelector.signingIn',
    defaultMessage: 'Starting CodyNo sign-in…',
  },
  waiting: {
    id: 'providerSelector.waiting',
    defaultMessage:
      'Finish signing in in your browser. CodyNo will connect automatically when you approve this device.',
  },
  codeLabel: {
    id: 'providerSelector.codeLabel',
    defaultMessage: 'Your sign-in code',
  },
  openBrowser: {
    id: 'providerSelector.openBrowser',
    defaultMessage: 'Open CodyNo in browser',
  },
  connected: {
    id: 'providerSelector.connected',
    defaultMessage: 'Connected to CodyNo',
  },
  connectedDescription: {
    id: 'providerSelector.connectedDescription',
    defaultMessage: 'Your CodyNo account is connected. Requests use the CodyNo gateway.',
  },
  error: {
    id: 'providerSelector.error',
    defaultMessage: 'CodyNo sign-in could not be completed. Please try again.',
  },
  expired: {
    id: 'providerSelector.expired',
    defaultMessage: 'This sign-in code expired. Start again to get a new code.',
  },
  denied: {
    id: 'providerSelector.denied',
    defaultMessage: 'This sign-in request was denied. Start again when you are ready.',
  },
  retry: {
    id: 'providerSelector.retry',
    defaultMessage: 'Try again',
  },
});

type AuthState = 'idle' | 'starting' | 'waiting' | 'connected' | 'error';

type DeviceAuthBody = {
  status?: string;
  token?: string;
};

interface ProviderSelectorProps {
  onConfigured: (providerName: string, modelId?: string) => void | Promise<void>;
  onFirstSelection?: () => void;
}

function isDeviceAuthBody(value: unknown): value is DeviceAuthBody {
  return typeof value === 'object' && value !== null;
}

function wait(ms: number): Promise<void> {
  return new Promise((resolve) => window.setTimeout(resolve, ms));
}

function splitDeviceToken(token: string): { host: string; apiKey: string } {
  const separator = token.lastIndexOf(':');
  if (separator <= 'https://'.length || separator === token.length - 1) {
    throw new Error('CodyNo returned an invalid device token');
  }

  const host = token.slice(0, separator);
  const apiKey = token.slice(separator + 1);
  if (!/^https?:\/\/[^/]+(?:\/[^/]*)?$/.test(host) || !apiKey) {
    throw new Error('CodyNo returned an invalid device token');
  }

  return { host, apiKey };
}

export default function ProviderSelector({
  onConfigured,
  onFirstSelection,
}: ProviderSelectorProps) {
  const intl = useIntl();
  const [authState, setAuthState] = useState<AuthState>('idle');
  const [code, setCode] = useState<string | null>(null);
  const [verificationUrl, setVerificationUrl] = useState<string | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const authAttempt = useRef(0);

  useEffect(() => {
    return () => {
      authAttempt.current += 1;
    };
  }, []);

  const startSignIn = async () => {
    const attempt = ++authAttempt.current;
    onFirstSelection?.();
    setAuthState('starting');
    setCode(null);
    setVerificationUrl(null);
    setErrorMessage(null);

    try {
      const start = await window.electron.startCodyNoDeviceAuth();
      if (authAttempt.current !== attempt) return;

      setCode(start.code);
      setVerificationUrl(start.verificationUrl);
      setAuthState('waiting');
      await window.electron.openExternal(start.verificationUrl);

      while (authAttempt.current === attempt) {
        const result = await window.electron.pollCodyNoDeviceAuth(start.code);
        if (authAttempt.current !== attempt) return;

        const body = isDeviceAuthBody(result.body) ? result.body : {};
        if (result.status === 200 && body.status === 'approved' && typeof body.token === 'string') {
          const { host, apiKey } = splitDeviceToken(body.token);

          await acpSaveProviderConfig(CODYNO_PROVIDER_ID, [
            { key: 'LITELLM_HOST', value: host },
            { key: 'LITELLM_API_KEY', value: apiKey },
            { key: 'LITELLM_BASE_PATH', value: CODYNO_BASE_PATH },
          ]);
          const modelId = await acpResolveCodyNoModel();

          if (authAttempt.current !== attempt) return;
          setAuthState('connected');
          await onConfigured(CODYNO_PROVIDER_ID, modelId);
          return;
        }

        if (result.status === 410 || body.status === 'expired') {
          throw new Error(intl.formatMessage(i18n.expired));
        }
        if (result.status === 403 || body.status === 'denied') {
          throw new Error(intl.formatMessage(i18n.denied));
        }
        if (result.status >= 400 && result.status !== 404) {
          throw new Error(intl.formatMessage(i18n.error));
        }

        await wait(POLL_INTERVAL_MS);
      }
    } catch (error) {
      if (authAttempt.current !== attempt) return;
      console.error('CodyNo sign-in failed:', error);
      setAuthState('error');
      setErrorMessage(error instanceof Error ? error.message : intl.formatMessage(i18n.error));
    }
  };

  const isBusy = authState === 'starting' || authState === 'waiting';

  return (
    <div className="max-w-xl">
      <div className="rounded-xl border border-border-default bg-background-muted p-6 sm:p-8">
        <div className="mb-5 flex items-center gap-3">
          <div className="flex size-10 items-center justify-center rounded-full bg-blue-500/10 text-blue-500">
            {authState === 'connected' ? <CheckCircle2 size={20} /> : <LogIn size={20} />}
          </div>
          <div>
            <h2 className="text-lg font-medium text-text-default">
              {authState === 'connected'
                ? intl.formatMessage(i18n.connected)
                : intl.formatMessage(i18n.title)}
            </h2>
            <p className="text-sm text-text-muted">
              {authState === 'connected'
                ? intl.formatMessage(i18n.connectedDescription)
                : intl.formatMessage(i18n.description)}
            </p>
          </div>
        </div>

        {authState === 'waiting' && code && verificationUrl ? (
          <div className="space-y-4">
            <p className="text-sm text-text-muted">{intl.formatMessage(i18n.waiting)}</p>
            <div className="rounded-lg border border-border-default bg-background-primary p-4 text-center">
              <p className="mb-2 text-xs uppercase tracking-wide text-text-muted">
                {intl.formatMessage(i18n.codeLabel)}
              </p>
              <p className="font-mono text-2xl font-semibold tracking-widest text-text-default">
                {code}
              </p>
            </div>
            <Button
              variant="outline"
              className="w-full"
              onClick={() => window.electron.openExternal(verificationUrl)}
            >
              <ExternalLink size={16} />
              {intl.formatMessage(i18n.openBrowser)}
            </Button>
          </div>
        ) : authState === 'connected' ? (
          <div className="flex items-center gap-2 text-sm text-green-600 dark:text-green-400">
            <CheckCircle2 size={16} />
            {intl.formatMessage(i18n.connectedDescription)}
          </div>
        ) : (
          <Button className="w-full" onClick={startSignIn} disabled={isBusy}>
            {authState === 'starting' ? <LoaderCircle className="animate-spin" size={16} /> : <LogIn size={16} />}
            {authState === 'starting'
              ? intl.formatMessage(i18n.signingIn)
              : authState === 'error'
                ? intl.formatMessage(i18n.retry)
                : intl.formatMessage(i18n.signIn)}
          </Button>
        )}

        {authState === 'error' && errorMessage && (
          <p className="mt-3 text-sm text-red-600 dark:text-red-400">{errorMessage}</p>
        )}
      </div>
    </div>
  );
}
