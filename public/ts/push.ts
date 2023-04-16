// Keep types in sync with data/push.rs

export type BashoId = string
export type PlayerId = number
export type Day = number

export type PushType =
  { 'Test': {} } |
  { 'Announcement': string } |
  { 'EntriesOpen': BashoId } |
  { 'BashoStartCountdown': BashoId } |
  { 'DayResult': [BashoId, PlayerId, Day] } |
  { 'BashoResult': [BashoId, PlayerId] }

export type Payload =
  { title: string, body: string }
  &
  (
    {
      type: 'Empty'
    }
    |
    {
      type: 'EntriesOpen',
      basho_id: BashoId,
      start_date: number,
    }
    |
    {
      type: 'BashoStartCountdown',
      basho_id: BashoId,
      start_date: number,
    }
    |
    {
      type: 'DayResult',
      basho_id: BashoId,
      name: string,
      day: Day,
      rikishi: [RikishiDayResult, RikishiDayResult, RikishiDayResult, RikishiDayResult, RikishiDayResult],
      rank: number,
      leaders: String[],
      leader_score: number
    }
  )

export type RikishiDayResult = {
  name: string,
  won?: boolean,
  against?: string,
  wins: number,
  losses: number,
  absence: number
}

export async function sendTestNotification() {
  await fetch('/push/test', {
    method: 'POST',
    credentials: 'same-origin'
  })
}

export async function adminTrigger(pushType: PushType) {
  await fetch('/push/trigger', {
    method: 'POST',
    credentials: 'same-origin',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(pushType)
  })
}
