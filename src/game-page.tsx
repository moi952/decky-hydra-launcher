import { useEffect, useRef } from "react";
import { GameCloudSaves } from "./game-cloud-saves";
import { SteamEmuSettings } from "./steam-emu-settings";
import type { Game } from "./api-types";

export interface GamePageProps {
  game: Game;
  onBack: () => void;
}

export function GamePage({ game, onBack }: GamePageProps) {
  const backButtonRef = useRef<HTMLButtonElement>(null);

  // Steam's gamepad navigation only recognizes real, visible interactive
  // controls as focus targets — a plain <div>, an empty Focusable, and an
  // off-screen dummy button were all tried and none registered. Without
  // grabbing a real one on mount, the B button falls through to the QAM's
  // default handler (closing the plugin) instead of our BackHandler, until
  // the player nudges a direction into the page. A visible back button
  // doubles as a safe initial focus target — an accidental A press just
  // navigates back, unlike the page's real action buttons (e.g. "Sync
  // Now", which fires immediately).
  useEffect(() => {
    backButtonRef.current?.focus();
  }, []);

  return (
    <>
      <button
        ref={backButtonRef}
        className="game-page__back-button"
        onClick={onBack}
      >
        ← Back
      </button>
      <GameCloudSaves game={game} />
      <SteamEmuSettings game={game} />
    </>
  );
}
