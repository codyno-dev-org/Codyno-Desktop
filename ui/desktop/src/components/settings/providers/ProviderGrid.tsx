import React, { memo, useMemo, useCallback, useState } from 'react';
import { ProviderCard } from './subcomponents/ProviderCard';
import ProviderConfigurationModal from './modal/ProviderConfigurationModal';
import type { ProviderDetails } from '../../../types/providers';
import { Search } from 'lucide-react';
import { Input } from '../../ui/input';
import { SwitchModelModal } from '../models/subcomponents/SwitchModelModal';
import type { View } from '../../../utils/navigationUtils';
import { defineMessages, useIntl } from '../../../i18n';

const i18n = defineMessages({
  chooseModel: {
    id: 'providerGrid.chooseModel',
    defaultMessage: 'Choose Model',
  },
  searchPlaceholder: {
    id: 'providerGrid.searchPlaceholder',
    defaultMessage: 'Search providers...',
  },
  noMatch: {
    id: 'providerGrid.noMatch',
    defaultMessage: 'No providers match "{query}"',
  },
});

const GridLayout = memo(function GridLayout({ children }: { children: React.ReactNode }) {
  return (
    <div
      className="grid gap-4 [&_*]:z-20 p-1"
      style={{
        gridTemplateColumns: 'repeat(auto-fill, minmax(200px, 200px))',
        justifyContent: 'center',
      }}
    >
      {children}
    </div>
  );
});

function ProviderCards({
  providers,
  isOnboarding,
  refreshProviders,
  setView,
  onModelSelected,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
  refreshProviders?: () => void;
  setView?: (view: View) => void;
  onModelSelected?: (model?: string) => void;
}) {
  const intl = useIntl();
  const [searchQuery, setSearchQuery] = useState('');
  const [configuringProvider, setConfiguringProvider] = useState<ProviderDetails | null>(null);
  const [showSwitchModelModal, setShowSwitchModelModal] = useState(false);
  const [switchModelProvider, setSwitchModelProvider] = useState<string | null>(null);

  const handleProviderLaunchWithModelSelection = useCallback((provider: ProviderDetails) => {
    setSwitchModelProvider(provider.name);
    setShowSwitchModelModal(true);
  }, []);

  const openModal = useCallback(
    (provider: ProviderDetails) => setConfiguringProvider(provider),
    []
  );

  const configureProviderViaModal = openModal;

  const onCloseProviderConfig = useCallback(() => {
    setConfiguringProvider(null);
    if (refreshProviders) {
      refreshProviders();
    }
  }, [refreshProviders]);

  const onProviderConfigured = useCallback(
    async (provider: ProviderDetails) => {
      setConfiguringProvider(null);
      if (refreshProviders) {
        await refreshProviders();
      }
      setSwitchModelProvider(provider.name);
      setShowSwitchModelModal(true);
    },
    [refreshProviders]
  );

  const onCloseSwitchModelModal = useCallback(() => {
    setShowSwitchModelModal(false);
  }, []);

  const handleSetView = useCallback(
    (view: View) => {
      setShowSwitchModelModal(false);
      if (setView) {
        setView(view);
      }
    },
    [setView]
  );

  const query = searchQuery.trim().toLowerCase();

  const providerCards = useMemo(() => {
    // providers needs to be an array
    const providersArray = Array.isArray(providers) ? providers : [];
    const sortedProviders = [...providersArray].sort(
      (a, b) =>
        Number(b.is_configured) - Number(a.is_configured) ||
        a.metadata.display_name.localeCompare(b.metadata.display_name)
    );
    const filteredProviders = query
      ? sortedProviders.filter(
          (provider) =>
            provider.metadata.display_name.toLowerCase().includes(query) ||
            provider.metadata.description.toLowerCase().includes(query)
        )
      : sortedProviders;
    const cards = filteredProviders.map((provider) => (
      <ProviderCard
        key={provider.name}
        provider={provider}
        onConfigure={() => configureProviderViaModal(provider)}
        onLaunch={() => handleProviderLaunchWithModelSelection(provider)}
        isOnboarding={isOnboarding}
      />
    ));

    return cards;
  }, [
    providers,
    query,
    isOnboarding,
    configureProviderViaModal,
    handleProviderLaunchWithModelSelection,
  ]);

  const hasNoMatches =
    query.length > 0 &&
    providerCards.length === 0 &&
    (Array.isArray(providers) ? providers.length : 0) > 0;

  return (
    <>
      <div className="mx-auto mb-4 max-w-md px-1">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-text-secondary" />
          <Input
            type="search"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder={intl.formatMessage(i18n.searchPlaceholder)}
            className="pl-9"
            data-testid="provider-search-input"
          />
        </div>
      </div>
      <GridLayout>{providerCards}</GridLayout>
      {hasNoMatches && (
        <div className="mt-2 text-center text-sm text-text-secondary">
          {intl.formatMessage(i18n.noMatch, { query: searchQuery.trim() })}
        </div>
      )}
      {configuringProvider && (
        <ProviderConfigurationModal
          provider={configuringProvider}
          onClose={onCloseProviderConfig}
          onConfigured={onProviderConfigured}
        />
      )}
      {showSwitchModelModal && (
        <SwitchModelModal
          sessionId={null}
          onClose={onCloseSwitchModelModal}
          setView={handleSetView}
          onModelSelected={onModelSelected}
          initialProvider={switchModelProvider}
          titleOverride={intl.formatMessage(i18n.chooseModel)}
        />
      )}
    </>
  );
}

export default function ProviderGrid({
  providers,
  isOnboarding,
  refreshProviders,
  setView,
  onModelSelected,
}: {
  providers: ProviderDetails[];
  isOnboarding: boolean;
  refreshProviders?: () => void;
  setView?: (view: View) => void;
  onModelSelected?: (model?: string) => void;
}) {
  return (
    <ProviderCards
      providers={providers}
      isOnboarding={isOnboarding}
      refreshProviders={refreshProviders}
      setView={setView}
      onModelSelected={onModelSelected}
    />
  );
}
