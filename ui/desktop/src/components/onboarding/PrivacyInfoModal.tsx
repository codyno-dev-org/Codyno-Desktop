import { Dialog, DialogContent, DialogHeader, DialogTitle } from '../ui/dialog';
import { defineMessages, useIntl } from '../../i18n';

const i18n = defineMessages({
  title: {
    id: 'privacyInfoModal.title',
    defaultMessage: 'Privacy details',
  },
  description: {
    id: 'privacyInfoModal.description',
    defaultMessage: 'CodyNo telemetry is disabled. Your conversations and code stay out of analytics.',
  },
  whatWeCollect: {
    id: 'privacyInfoModal.whatWeCollect',
    defaultMessage: 'CodyNo does not collect:',
  },
  collectOs: {
    id: 'privacyInfoModal.collectOs',
    defaultMessage: 'Conversations or code',
  },
  collectVersion: {
    id: 'privacyInfoModal.collectVersion',
    defaultMessage: 'Provider credentials or model prompts',
  },
  collectProvider: {
    id: 'privacyInfoModal.collectProvider',
    defaultMessage: 'Personal usage analytics',
  },
  collectExtensions: {
    id: 'privacyInfoModal.collectExtensions',
    defaultMessage: 'Background telemetry',
  },
  collectSession: {
    id: 'privacyInfoModal.collectSession',
    defaultMessage: 'Automatic updater checks',
  },
  collectErrors: {
    id: 'privacyInfoModal.collectErrors',
    defaultMessage: 'Third-party provider account data',
  },
  neverCollect: {
    id: 'privacyInfoModal.neverCollect',
    defaultMessage: 'Telemetry and automatic updates are disabled for CodyNo.',
  },
});

interface PrivacyInfoModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export default function PrivacyInfoModal({ isOpen, onClose }: PrivacyInfoModalProps) {
  const intl = useIntl();

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent className="w-[440px]">
        <DialogHeader>
          <DialogTitle className="text-center">{intl.formatMessage(i18n.title)}</DialogTitle>
        </DialogHeader>

        <div>
          <p className="text-text-muted text-sm mb-3">
            {intl.formatMessage(i18n.description)}
          </p>
          <p className="font-medium text-text-default text-sm mb-1.5">{intl.formatMessage(i18n.whatWeCollect)}</p>
          <ul className="text-text-muted text-sm list-disc list-outside space-y-0.5 ml-5 mb-3">
            <li>{intl.formatMessage(i18n.collectOs)}</li>
            <li>{intl.formatMessage(i18n.collectVersion)}</li>
            <li>{intl.formatMessage(i18n.collectProvider)}</li>
            <li>{intl.formatMessage(i18n.collectExtensions)}</li>
            <li>{intl.formatMessage(i18n.collectSession)}</li>
            <li>{intl.formatMessage(i18n.collectErrors)}</li>
          </ul>
          <p className="text-text-muted text-sm">
            {intl.formatMessage(i18n.neverCollect)}
          </p>
        </div>
      </DialogContent>
    </Dialog>
  );
}
