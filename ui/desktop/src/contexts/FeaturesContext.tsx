import { createContext, useContext, useMemo } from 'react';

interface FeaturesContextValue {
  localInference: boolean;
  isLoading: boolean;
}

const FeaturesContext = createContext<FeaturesContextValue | null>(null);

export function FeaturesProvider({ children }: { children: React.ReactNode }) {
  const value = useMemo<FeaturesContextValue>(
    () => ({
      // CodyNo routes all inference through LiteLLM. Local models are not a
      // supported product path even if a binary was built with that feature.
      localInference: false,
      isLoading: false,
    }),
    []
  );

  return <FeaturesContext.Provider value={value}>{children}</FeaturesContext.Provider>;
}

export function useFeatures(): FeaturesContextValue {
  const context = useContext(FeaturesContext);
  if (!context) {
    throw new Error('useFeatures must be used within a FeaturesProvider');
  }
  return context;
}
