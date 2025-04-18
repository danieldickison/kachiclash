// Keep types in sync with data/push.rs

export type BashoId = string;
export type PlayerId = number;
export type Day = number;

export type PushType =
  | { Test: undefined }
  | { Announcement: string }
  | { EntriesOpen: BashoId }
  | { BashoStartCountdown: BashoId }
  | { DayResult: [BashoId, PlayerId, Day] }
  | { BashoResult: [BashoId, PlayerId] };

export type Payload = { title: string; body: string; url: string } & (
  | {
      type: "Empty";
    }
  | {
      type: "EntriesOpen";
      basho_id: BashoId;
      start_date: number;
    }
  | {
      type: "BashoStartCountdown";
      basho_id: BashoId;
      start_date: number;
    }
  | {
      type: "DayResult";
      basho_id: BashoId;
      name: string;
      day: Day;
      rikishi: [
        RikishiDayResult,
        RikishiDayResult,
        RikishiDayResult,
        RikishiDayResult,
        RikishiDayResult,
      ];
      rank: number;
      leader_score: number;
    }
);

export interface RikishiDayResult {
  name: string;
  win?: boolean;
}

export async function sendTestNotification(): Promise<void> {
  await fetch("/push/test", {
    method: "POST",
    credentials: "same-origin",
  });
}

interface SendStats {
  success: number;
  invalid: number;
  fail: number;
  players: number;
}

export async function adminTrigger(pushType: PushType): Promise<void> {
  const res = await fetch("/push/trigger", {
    method: "POST",
    credentials: "same-origin",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(pushType),
  });
  alertSendStats(await res.json());
}

export function alertSendStats(res: any): void {
  const stats = res as SendStats;
  console.debug("push send stats", stats);
  alert(`Notified ${stats.players} players with devices:
    ${stats.success} success
    ${stats.invalid} invalid (unsusbscribed)
    ${stats.fail} fail`);
}
