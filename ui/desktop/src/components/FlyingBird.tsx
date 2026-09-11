import { CodyNoMark } from './icons';

interface FlyingBirdProps {
  className?: string;
  cycleInterval?: number; // milliseconds between bird frame changes
}

export default function FlyingBird({ className = '' }: FlyingBirdProps) {
  return (
    <div className={`transition-opacity duration-75 animate-pulse ${className}`}>
      <CodyNoMark className="w-4 h-4" />
    </div>
  );
}
