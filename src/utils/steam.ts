import { invoke } from "@tauri-apps/api/core";

const APP_ID = 480;
let isInitialized = false;

export async function initSteam(): Promise<boolean> {
  if (isInitialized) return true;
  try {
    const res = await invoke<string>("steam_init", { appId: APP_ID });
    console.log("[Steam]", res);
    isInitialized = true;
    return true;
  } catch (err) {
    console.error("[Steam] Initialization failed:", err);
    return false;
  }
}

// Example function
export async function uploadBestTime(timeInSeconds: number): Promise<void> {
  try {
    if (!isInitialized) await initSteam();
    const res = await invoke<string>("upload_score", {
      leaderboardName: "best_time",
      score: Math.floor(timeInSeconds),
    });
    console.log("[Steam Leaderboard] Best time response:", res);
  } catch (error) {
    console.error("[Steam Leaderboard] Error uploading best time:", error);
  }
}

// Example function
export async function uploadTotalWins(winsCount: number): Promise<void> {
  try {
    if (!isInitialized) await initSteam();
    const res = await invoke<string>("upload_score", {
      leaderboardName: "most_wins",
      score: winsCount,
    });
    console.log("[Steam Leaderboard] Wins response:", res);
  } catch (error) {
    console.error("[Steam Leaderboard] Error uploading total wins:", error);
  }
}

// Example function
export const triggerAchievement = async (achievementId: string) => {
  try {
    await invoke("unlock_achievement", { achievementId });
  } catch (error) {
    console.error("Error while trying to unlock achievement:", error);
  }
};
