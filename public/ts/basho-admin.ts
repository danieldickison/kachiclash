import { adminTrigger, alertSendStats } from "./push.js";

document.querySelectorAll(".bestow-emperors-cup-button").forEach((button) => {
  button.addEventListener("click", () => {
    void postCup(button, true);
  });
});
document.querySelectorAll(".revoke-emperors-cup-button").forEach((button) => {
  button.addEventListener("click", () => {
    void postCup(button, false);
  });
});

async function postCup(button: Element, bestow: boolean): Promise<void> {
  if (
    !(button instanceof HTMLButtonElement) ||
    button.dataset.playerId === undefined
  ) {
    throw new Error(`unexpected button element: ${button.tagName}`);
  }
  const data = {
    player_id: parseInt(button.dataset.playerId),
  };
  const url =
    location.href + "/" + (bestow ? "bestow" : "revoke") + "_emperors_cup";
  const response = await fetch(url, {
    method: "POST",
    body: JSON.stringify(data),
    headers: new Headers({
      "Content-Type": "application/json",
    }),
    credentials: "same-origin",
  });
  if (response.ok) {
    alert("Emperor's Cup has been " + (bestow ? "bestowed" : "revoked"));
  } else {
    const text = await response.text();
    alert("error: " + text);
  }
}

const bashoId = (
  document.querySelector('meta[name="basho_id"]') as HTMLMetaElement
).content;

document
  .querySelector(".trigger-announcement")
  ?.addEventListener("click", (event) => {
    (event.target as HTMLButtonElement).disabled = true;
    event.preventDefault();
    const msg = prompt("Message:");
    if (msg === null || msg === "") return;
    void adminTrigger({ Announcement: msg });
  });
document
  .querySelector(".trigger-entries-open")
  ?.addEventListener("click", (event) => {
    (event.target as HTMLButtonElement).disabled = true;
    event.preventDefault();
    void adminTrigger({ EntriesOpen: bashoId });
  });
document
  .querySelector(".trigger-countdown")
  ?.addEventListener("click", (event) => {
    (event.target as HTMLButtonElement).disabled = true;
    event.preventDefault();
    void adminTrigger({ BashoStartCountdown: bashoId });
  });

document
  .querySelector(".update-torikumi")
  ?.addEventListener("click", (event) => {
    event.preventDefault();
    const button = event.target as HTMLButtonElement;
    button.disabled = true;
    updateTorikumi(button.dataset.day ?? "1").finally(() => {
      button.disabled = false;
    });
  });
async function updateTorikumi(defaultDay: string): Promise<void> {
  const day = parseInt(prompt("Day:", defaultDay) ?? "NaN");
  if (isNaN(day)) return;

  const notify = confirm("Send push notifications?");

  const res = await fetch(`/basho/${bashoId}/day/${day}`, {
    method: "POST",
    credentials: "same-origin",
    body: JSON.stringify({ notify }),
    headers: {
      "Content-Type": "application/json",
    },
  });
  alertSendStats(await res.json());
  location.reload();
}

document
  .querySelector(".finalize-basho")
  ?.addEventListener("click", (event) => {
    (event.target as HTMLButtonElement).disabled = true;
    event.preventDefault();
    void finalizeBasho();
  });
async function finalizeBasho(): Promise<void> {
  const res = await fetch(`/basho/${bashoId}/finalize`, {
    method: "POST",
    credentials: "same-origin",
  });
  alertSendStats(await res.json());
  location.reload();
}

document.querySelector(".hide-admin")?.addEventListener("click", (event) => {
  event.preventDefault();
  document.getElementById("admin")?.remove();
});

export default {};
