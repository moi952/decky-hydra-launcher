import { create } from "zustand";
import type { GameAssets } from "../api-types";

interface CurrentGameStore {
  objectId: string | null;
  remoteId: string | null;
  gameAssets: GameAssets | null;
  startedAt: Date | null;
  elapsedTimeInMillis: number;
  setGameAssets: (gameAssets: GameAssets) => void;
  clearGame: () => void;
  setStartedAt: (startedAt: Date) => void;
  setElapsedTimeInMillis: (elapsedTimeInMillis: number) => void;
  setObjectId: (objectId: string | null) => void;
  setRemoteId: (remoteId: string | null) => void;
}

export const useCurrentGame = create<CurrentGameStore>((set) => ({
  objectId: null,
  remoteId: null,
  gameAssets: null,
  startedAt: null,
  elapsedTimeInMillis: 0,
  setGameAssets: (gameAssets) => set({ gameAssets }),
  clearGame: () =>
    set({
      objectId: null,
      remoteId: null,
      gameAssets: null,
      startedAt: null,
      elapsedTimeInMillis: 0,
    }),
  setStartedAt: (startedAt) => set({ startedAt }),
  setElapsedTimeInMillis: (elapsedTimeInMillis) => set({ elapsedTimeInMillis }),
  setObjectId: (objectId) => set({ objectId }),
  setRemoteId: (remoteId) => set({ remoteId }),
}));
