import { useState } from 'react';
import { ScrollArea } from '../../ui/scroll-area';
import BackButton from '../../ui/BackButton';
import ProviderSelector from '../../onboarding/ProviderSelector';
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  onboardingTitle: {
    id: 'providerSettings.onboardingTitle',
    defaultMessage: 'Connect CodyNo',
  },
  settingsTitle: {
    id: 'providerSettings.settingsTitle',
    defaultMessage: 'CodyNo account',
  },
  onboardingDescription: {
    id: 'providerSettings.onboardingDescription',
    defaultMessage:
      'Sign in to CodyNo to use the CodyNo AI gateway. Provider selection and direct provider keys are not available in this app.',
  },
  settingsDescription: {
    id: 'providerSettings.settingsDescription',
    defaultMessage:
      'Manage the CodyNo account used by this desktop app. All model requests go through CodyNo.',
  },
  connected: {
    id: 'providerSettings.connected',
    defaultMessage: 'CodyNo is connected',
  },
});

interface ProviderSettingsProps {
  onClose: () => void;
  isOnboarding: boolean;
  onProviderLaunched?: (model?: string) => void;
}

export default function ProviderSettings({
  onClose,
  isOnboarding,
  onProviderLaunched,
}: ProviderSettingsProps) {
  const intl = useIntl();
  const [connected, setConnected] = useState(false);

  const handleConfigured = async (_providerName: string, modelId?: string) => {
    setConnected(true);
    onProviderLaunched?.(modelId);
  };

  return (
    <div className="h-screen w-full flex flex-col bg-background-primary text-text-primary">
      <ScrollArea className="flex-1 w-full">
        <div className="w-full max-w-4xl mx-auto px-4 sm:px-6 md:px-8 pt-12 pb-4">
          <div className="flex flex-col pb-8 border-b border-border-primary">
            <div className="flex items-center pt-2 mb-1 no-drag">
              <BackButton onClick={onClose} />
            </div>
            <h1 className="text-4xl font-light mb-4 pt-6" data-testid="provider-selection-heading">
              {intl.formatMessage(isOnboarding ? i18n.onboardingTitle : i18n.settingsTitle)}
            </h1>
            <p className="text-sm sm:text-base text-text-secondary max-w-2xl">
              {intl.formatMessage(
                isOnboarding ? i18n.onboardingDescription : i18n.settingsDescription
              )}
            </p>
            {connected && (
              <p className="mt-4 text-sm text-green-600 dark:text-green-400">
                {intl.formatMessage(i18n.connected)}
              </p>
            )}
          </div>
        </div>

        <div className="py-8 pt-[20px]">
          <div className="w-full max-w-4xl mx-auto pt-4 px-4 sm:px-6 md:px-8">
            <ProviderSelector onConfigured={handleConfigured} />
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
