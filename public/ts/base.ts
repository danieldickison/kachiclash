import { pushPermissionState } from "./service-client.js";

initBashoCountDown();
initUserMenu();
void initPushPromo();

// Init basho start count down clock
function initBashoCountDown(): void {
  for (const timeSpan of document.querySelectorAll(".js-basho-count-down")) {
    if (
      !(timeSpan instanceof HTMLElement) ||
      timeSpan.dataset.startDate === undefined
    ) {
      throw new Error(
        `unexpected element with css class .js-basho-count-down: ${timeSpan.tagName}`,
      );
    }
    const startTimestamp = parseInt(timeSpan.dataset.startDate);
    const updateTimeRemaining = function (): void {
      const remaining = (startTimestamp - Date.now()) / 1000;
      const seconds = Math.floor(remaining % 60);
      const minutes = Math.floor(remaining / 60) % 60;
      const hours = Math.floor(remaining / 60 / 60) % 24;
      const days = Math.floor(remaining / 60 / 60 / 24);
      let str = "";

      if (days > 1) str += `${days} days `;
      else if (days > 0) str += "1 day ";

      if (hours > 1) str += `${hours} hours `;
      else if (hours === 1) str += "1 hour ";
      else if (days > 0) str += "0 hours ";

      if (minutes > 1) str += `${minutes} minutes `;
      else if (minutes === 1) str += "1 minute ";
      else if (hours > 0) str += "0 minutes ";

      if (seconds > 1) str += `${seconds} seconds `;
      else if (seconds === 1) str += "1 second ";
      else if (minutes > 0) str += "0 seconds ";

      timeSpan.innerText = str.trim();
    };

    updateTimeRemaining();
    setInterval(updateTimeRemaining, 1000);
  }

  // Show local time of basho start times
  const DATETIME_FORMAT = new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "long",
    day: "numeric",
    hour: "numeric",
    minute: "numeric",
    timeZoneName: "short",
  });
  for (const el of document.querySelectorAll(".js-local-datetime")) {
    if (!(el instanceof HTMLElement) || el.dataset.timestamp === undefined) {
      throw new Error(
        `unexpected element with css class .js-basho-count-down: ${el.tagName}`,
      );
    }
    const timestamp = parseInt(el.dataset.timestamp);
    if (!isNaN(timestamp)) {
      const date = new Date(timestamp);
      el.innerText = DATETIME_FORMAT.format(date);
    }
  }
}

// User menu
function initUserMenu(): void {
  const playerMenu = document.querySelector(
    "#g-header .player-menu",
  ) as HTMLElement;
  const menuHeader = playerMenu.querySelector(".g-player-listing");
  if (menuHeader instanceof HTMLAnchorElement) {
    const bodyClickHandler = (event: Event): void => {
      const target = event.target;
      if (target instanceof Element && !target.matches(".player-menu *")) {
        playerMenu.classList.remove("open");
        window.removeEventListener("click", bodyClickHandler, {
          capture: true,
        });
      }
    };
    menuHeader.addEventListener("click", (event) => {
      event.preventDefault();
      if (playerMenu.classList.toggle("open")) {
        window.addEventListener("click", bodyClickHandler, { capture: true });
      } else {
        window.removeEventListener("click", bodyClickHandler, {
          capture: true,
        });
      }
    });
  }
}

async function initPushPromo(): Promise<void> {
  const promo = document.getElementById("push-promo");

  if (
    promo === null ||
    localStorage.getItem("push-promo-dismissed") === "1" ||
    (await pushPermissionState()) !== "prompt"
  ) {
    return;
  }

  promo.style.display = "block";

  const dismiss = promo.querySelector("button") as HTMLButtonElement;
  dismiss.addEventListener("click", (event) => {
    event.preventDefault();
    promo.style.display = "";
    localStorage.setItem("push-promo-dismissed", "1");
  });
}
