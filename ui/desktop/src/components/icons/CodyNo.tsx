import CodyNoLogo from '../../images/codyno-logo.svg';

/** CodyNo's official mark used throughout the desktop application. */
export function CodyNoMark({ className = '' }: { className?: string }) {
  return (
    <img
      src={CodyNoLogo}
      className={className}
      alt="CodyNo"
    />
  );
}
