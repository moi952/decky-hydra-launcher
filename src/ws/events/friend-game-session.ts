import { toaster } from "@decky/api";

import type { FriendGameSession } from "../../generated/envelope";
import { api } from "../../hydra-api";
import { composeToastLogo } from "../../helpers";
import type { UserProfile } from "./types";
import type { GameAssets } from "../../api-types";

export const friendGameSessionEvent = async (payload: FriendGameSession) => {
  const [friend, gameAssets] = await Promise.all([
    api.get<UserProfile>(`users/${payload.friendId}`).json(),
    api.get<GameAssets>(`games/steam/${payload.objectId}/assets`).json(),
  ]);

  if (friend && gameAssets) {
    toaster.toast({
      title: `${friend.displayName} started playing`,
      body: gameAssets.title,
      logo: composeToastLogo(friend.profileImageUrl),
    });
  }
};
