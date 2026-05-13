import { Circle, Wifi, WifiOff } from "lucide-react";

interface StatusPillProps {
  active: boolean;
  activeText: string;
  inactiveText: string;
}

export function StatusPill({ active, activeText, inactiveText }: StatusPillProps) {
  const Icon = active ? Wifi : WifiOff;

  return (
    <span className={`status-pill ${active ? "is-active" : "is-idle"}`}>
      <Icon aria-hidden="true" size={14} />
      {active ? activeText : inactiveText}
    </span>
  );
}

interface LevelDotProps {
  level: "info" | "warning" | "error";
}

export function LevelDot({ level }: LevelDotProps) {
  return (
    <span className={`level-dot level-${level}`}>
      <Circle aria-hidden="true" size={8} fill="currentColor" />
    </span>
  );
}

